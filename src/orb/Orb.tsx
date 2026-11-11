import { useEffect, useRef, type MutableRefObject } from "react";
import "./orb.css";

export type Phase = "hidden" | "idle" | "listening" | "thinking" | "speaking" | "error";

export const THEMES: Record<string, { body: string; label: string }> = {
  menta: { body: "#62D9B4", label: "Menta" },
  uva: { body: "#A98BFF", label: "Uva" },
  mandarina: { body: "#FFA94D", label: "Mandarina" },
  chicle: { body: "#FF7EB6", label: "Chicle" },
  cielo: { body: "#6EC1FF", label: "Cielo" },
};

const INK = "#1E1B2E";
const N = 72;
const CX = 60;
const CY = 58;
const R = 32;
/** Formas por las que pasa el orbe (0 = círculo, n = polígono regular de n lados). */
const SEQ = [0, 3, 4, 5, 6, 0, 4, 3];

interface Mood {
  spin: number; // rad/s
  hold: number; // s quieto en cada forma
  morph: number; // s de transformación
  amp: number; // cuánto crece con el nivel
  sparks: boolean;
  inner: string;
}
const MOODS: Record<Exclude<Phase, "hidden">, Mood> = {
  idle: { spin: 0.45, hold: 1.6, morph: 1.1, amp: 0.03, sparks: false, inner: "#FFF6D6" },
  listening: { spin: 0.9, hold: 1.0, morph: 0.7, amp: 0.16, sparks: false, inner: "#FFFFFF" },
  thinking: { spin: 3.2, hold: 0.35, morph: 0.5, amp: 0.04, sparks: true, inner: "#FFD25E" },
  speaking: { spin: 1.3, hold: 0.8, morph: 0.6, amp: 0.12, sparks: false, inner: "#FFF6D6" },
  error: { spin: 0, hold: 99, morph: 0.3, amp: 0.05, sparks: false, inner: "#FFF6D6" },
};

function radius(theta: number, n: number): number {
  if (!n) return 1;
  const a = (2 * Math.PI) / n;
  const m = ((theta % a) + a) % a;
  return Math.cos(Math.PI / n) / Math.cos(m - a / 2);
}
const ease = (x: number) => (x < 0.5 ? 2 * x * x : 1 - Math.pow(-2 * x + 2, 2) / 2);

function points(ni: number, nj: number, k: number, rot: number, cx: number, cy: number, size: number): string {
  const out: string[] = [];
  for (let i = 0; i < N; i++) {
    const th = (i / N) * 2 * Math.PI;
    const r = (radius(th, ni) + (radius(th, nj) - radius(th, ni)) * k) * size;
    const an = th + rot - Math.PI / 2;
    out.push(`${(cx + Math.cos(an) * r).toFixed(1)},${(cy + Math.sin(an) * r).toFixed(1)}`);
  }
  return out.join(" ");
}

/**
 * Orbe geométrico: un círculo de trazo grueso que gira y se transforma entre formas.
 * `level` (0..1) es el volumen del micrófono o de la voz; se lee en cada fotograma.
 */
export function Orb({ phase, theme, level }: { phase: Phase; theme: string; level: MutableRefObject<number> }) {
  const shadow = useRef<SVGPolygonElement>(null);
  const body = useRef<SVGPolygonElement>(null);
  const inner = useRef<SVGPolygonElement>(null);
  const sparks = useRef<SVGGElement>(null);
  const phaseRef = useRef(phase);
  phaseRef.current = phase;

  useEffect(() => {
    if (phase === "hidden") return;
    let raf = 0;
    let last = performance.now();
    let rot = 0;
    let idx = 0;
    let stage = 0; // 0 quieto, 1 transformando
    let tm = 0;
    let smooth = 0;
    const loop = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const p = phaseRef.current === "hidden" ? "idle" : phaseRef.current;
      const mood = MOODS[p];
      tm += dt;
      if (tm > (stage === 0 ? mood.hold : mood.morph)) {
        tm = 0;
        if (stage === 0) stage = 1;
        else {
          stage = 0;
          idx = (idx + 1) % SEQ.length;
        }
      }
      const k = stage === 1 ? ease(Math.min(1, tm / mood.morph)) : 0;
      let a = SEQ[idx];
      let b = SEQ[(idx + 1) % SEQ.length];
      if (p === "error") a = b = 4;
      rot += mood.spin * dt;
      const t = now / 1000;
      const target = p === "listening" || p === "speaking" ? level.current : p === "idle" ? 0.5 + 0.5 * Math.sin(t * 1.6) : 0.4;
      smooth += (target - smooth) * (target > smooth ? 0.45 : 0.12);
      const sc = 1 + mood.amp * smooth * (p === "idle" ? 1 : 2.2);
      const jx = p === "error" ? Math.sin(t * 40) * 1.6 : 0;
      shadow.current?.setAttribute("points", points(a, b, k, rot, CX + 3.5 + jx, CY + 4, R * sc));
      body.current?.setAttribute("points", points(a, b, k, rot, CX + jx, CY, R * sc));
      inner.current?.setAttribute("points", points(b, a, 1 - k, -rot * 1.4, CX + jx, CY, R * 0.5 * sc));
      const g = sparks.current;
      if (g) {
        g.style.opacity = mood.sparks ? "1" : "0";
        if (mood.sparks) {
          for (let i = 0; i < g.children.length; i++) {
            const an = t * 3 + i * 2.1;
            g.children[i].setAttribute("cx", (CX + Math.cos(an) * (R + 10 + i * 3)).toFixed(1));
            g.children[i].setAttribute("cy", (CY + Math.sin(an) * (R * 0.55 + 8 + i * 2)).toFixed(1));
          }
        }
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [phase === "hidden", level]);

  const body_color = phase === "error" ? "#FF8A7A" : (THEMES[theme] ?? THEMES.menta).body;
  const inner_color = MOODS[phase === "hidden" ? "idle" : phase].inner;

  return (
    <div className="stage" data-phase={phase}>
      <svg viewBox="0 0 120 120" width="100%" height="100%">
        <polygon ref={shadow} fill={INK} />
        <polygon ref={body} fill={body_color} stroke={INK} strokeWidth={3.2} strokeLinejoin="round" />
        <polygon ref={inner} fill={inner_color} stroke={INK} strokeWidth={2.6} strokeLinejoin="round" />
        <g ref={sparks} fill="#FFD25E" stroke={INK} strokeWidth={1.6} style={{ opacity: 0 }}>
          <circle r={3} />
          <circle r={2.5} />
          <circle r={2} />
        </g>
      </svg>
    </div>
  );
}
