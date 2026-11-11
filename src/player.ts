import { api } from "./api";

/**
 * Reproduce la respuesta fragmento a fragmento: mientras suena uno se
 * sintetiza el siguiente. Expone el nivel de audio para animar la esfera.
 */
export class Player {
  private ctx: AudioContext | null = null;
  private gain: GainNode | null = null;
  private analyser: AnalyserNode | null = null;
  private samples: Float32Array<ArrayBuffer> | null = null;
  private source: AudioBufferSourceNode | null = null;
  private stopped = false;

  private ensure(): AudioContext {
    if (!this.ctx) {
      this.ctx = new AudioContext();
      this.gain = this.ctx.createGain();
      this.analyser = this.ctx.createAnalyser();
      this.analyser.fftSize = 512;
      this.samples = new Float32Array(this.analyser.fftSize);
      this.gain.connect(this.analyser);
      this.analyser.connect(this.ctx.destination);
    }
    if (this.ctx.state === "suspended") void this.ctx.resume();
    return this.ctx;
  }

  /** Nivel 0..1 del audio que está sonando. */
  level(): number {
    if (!this.analyser || !this.samples || !this.source) return 0;
    this.analyser.getFloatTimeDomainData(this.samples);
    let sum = 0;
    for (const s of this.samples) sum += s * s;
    return Math.min(1, Math.sqrt(sum / this.samples.length) * 4.5);
  }

  async speak(chunks: string[], volume: number): Promise<void> {
    const ctx = this.ensure();
    this.stopped = false;
    this.gain!.gain.value = volume;
    if (chunks.length === 0) return;

    let next: Promise<ArrayBuffer> = api.synthesize(chunks[0]);
    for (let i = 0; i < chunks.length; i++) {
      const data = await next;
      if (this.stopped) return;
      if (i + 1 < chunks.length) {
        next = api.synthesize(chunks[i + 1]);
        next.catch(() => undefined); // se gestiona en la siguiente vuelta
      }
      const buffer = await ctx.decodeAudioData(data);
      if (this.stopped) return;
      await this.play(ctx, buffer);
    }
  }

  private play(ctx: AudioContext, buffer: AudioBuffer): Promise<void> {
    return new Promise((resolve) => {
      const src = ctx.createBufferSource();
      src.buffer = buffer;
      src.connect(this.gain!);
      src.onended = () => {
        if (this.source === src) this.source = null;
        resolve();
      };
      this.source = src;
      src.start();
    });
  }

  stop(): void {
    this.stopped = true;
    try {
      this.source?.stop();
    } catch {
      /* ya detenido */
    }
    this.source = null;
  }
}
