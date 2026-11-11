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

export interface AvatarProps {
  phase: Phase;
  theme: string;
  level: MutableRefObject<number>;
  /** Multiplicador del grosor de los trazos (más grueso en tamaños pequeños). */
  sw: number;
}

/**
 * Orbe geométrico: un círculo de trazo grueso que gira y se transforma entre formas.
 * `level` (0..1) es el volumen del micrófono o de la voz; se lee en cada fotograma.
 */
function GeoOrb({ phase, theme, level, sw }: AvatarProps) {
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
        <polygon ref={body} fill={body_color} stroke={INK} strokeWidth={3.2 * sw} strokeLinejoin="round" />
        <polygon ref={inner} fill={inner_color} stroke={INK} strokeWidth={2.6 * sw} strokeLinejoin="round" />
        <g ref={sparks} fill="#FFD25E" stroke={INK} strokeWidth={1.6 * sw} style={{ opacity: 0 }}>
          <circle r={3} />
          <circle r={2.5} />
          <circle r={2} />
        </g>
      </svg>
    </div>
  );
}

// ------------------------------------------------------------------ avatares nuevos

type PK = Exclude<Phase, "hidden">;
interface Frame {
  t: number;
  dt: number;
  p: PK;
  /** Intensidad suavizada: respiración en reposo, volumen al escuchar o hablar. */
  lv: number;
  /** Solo el volumen real (0 si no se está escuchando ni hablando). */
  voice: number;
}

/** Bucle de animación compartido por los avatares nuevos. */
function useAnim(phase: Phase, level: MutableRefObject<number>, tick: (f: Frame) => void) {
  const phaseRef = useRef(phase);
  phaseRef.current = phase;
  const tickRef = useRef(tick);
  tickRef.current = tick;
  useEffect(() => {
    if (phase === "hidden") return;
    let raf = 0;
    let last = performance.now();
    let smooth = 0;
    const loop = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const p: PK = phaseRef.current === "hidden" ? "idle" : phaseRef.current;
      const t = now / 1000;
      const talking = p === "listening" || p === "speaking";
      const target = talking ? level.current : p === "idle" ? 0.5 + 0.5 * Math.sin(t * 1.6) : 0.4;
      smooth += (target - smooth) * (target > smooth ? 0.45 : 0.12);
      tickRef.current({ t, dt, p, lv: smooth, voice: talking ? smooth : 0 });
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [phase === "hidden", level]);
}

const attr = (el: Element | null | undefined, k: string, v: number | string) => el?.setAttribute(k, String(v));
const bodyColor = (phase: Phase, theme: string) => (phase === "error" ? "#FF8A7A" : (THEMES[theme] ?? THEMES.menta).body);
const innerColor = (phase: Phase) => MOODS[phase === "hidden" ? "idle" : phase].inner;

function Stage({ phase, children }: { phase: Phase; children: React.ReactNode }) {
  return (
    <div className="stage" data-phase={phase}>
      <svg viewBox="0 0 120 120" width="100%" height="100%">
        {children}
      </svg>
    </div>
  );
}

/** Gelatina: una criatura blanda con ojos que parpadean, miran alrededor y hablan. */
function Gelatina({ phase, theme, level, sw }: AvatarProps) {
  const shadow = useRef<SVGPolygonElement>(null);
  const body = useRef<SVGPolygonElement>(null);
  const eyes = useRef<(SVGEllipseElement | null)[]>([]);
  const pupils = useRef<(SVGCircleElement | null)[]>([]);
  const crosses = useRef<(SVGPathElement | null)[]>([]);
  const mouth = useRef<SVGPathElement>(null);
  const sparks = useRef<SVGGElement>(null);
  const st = useRef({ next: 2.2, blink: 0 });

  useAnim(phase, level, ({ t, dt, p, lv, voice }) => {
    const s = st.current;
    const amp = p === "idle" ? 0.03 + 0.01 * lv : p === "listening" ? 0.05 + 0.12 * voice : p === "thinking" ? 0.09 : p === "speaking" ? 0.04 + 0.14 * voice : 0.02;
    const sp = p === "thinking" ? 5 : p === "idle" ? 1.4 : 2.6;
    const jx = p === "error" ? Math.sin(t * 40) * 1.6 : 0;
    const squash = voice * 0.1;
    const pts = (ox: number, oy: number) => {
      const out: string[] = [];
      for (let i = 0; i < N; i++) {
        const th = (i / N) * 2 * Math.PI;
        const r = R * (1 + amp * (Math.sin(3 * th + t * sp) + 0.7 * Math.sin(2 * th - t * sp * 1.3)));
        out.push(`${(CX + ox + jx + Math.cos(th) * r * (1 + squash)).toFixed(1)},${(CY + oy + Math.sin(th) * r * (1 - squash)).toFixed(1)}`);
      }
      return out.join(" ");
    };
    attr(shadow.current, "points", pts(3.5, 4));
    attr(body.current, "points", pts(0, 0));

    s.next -= dt;
    if (s.next < 0) {
      s.blink = 0.16;
      s.next = 2.2 + Math.random() * 2.5;
    }
    const blinking = s.blink > 0;
    if (blinking) s.blink -= dt;
    const open = blinking ? 0.12 : 1;
    const ry = (p === "listening" ? 10.5 : 8.5) * open;
    let lx = Math.sin(t * 0.7) * 2.6;
    let ly = Math.sin(t * 0.5) * 1.4;
    if (p === "thinking") {
      lx = Math.cos(t * 5) * 3;
      ly = -2 + Math.sin(t * 5) * 2;
    } else if (p === "listening") {
      lx = 0;
      ly = 0;
    } else if (p === "speaking") {
      lx = Math.sin(t * 1.5) * 1.5;
      ly = 0;
    }
    for (let i = 0; i < 2; i++) {
      const ex = CX + jx + (i ? 11.5 : -11.5);
      const ey = CY - 5;
      attr(eyes.current[i], "cx", ex);
      attr(eyes.current[i], "cy", ey);
      attr(eyes.current[i], "ry", ry);
      const pu = pupils.current[i];
      attr(pu, "cx", ex + lx);
      attr(pu, "cy", ey + ly);
      attr(pu, "r", p === "listening" ? 4 : 3.4);
      if (pu) pu.style.display = blinking || p === "error" ? "none" : "";
      const x = crosses.current[i];
      attr(x, "d", `M${ex - 4},${ey - 4}L${ex + 4},${ey + 4}M${ex + 4},${ey - 4}L${ex - 4},${ey + 4}`);
      if (x) x.style.display = p === "error" ? "" : "none";
    }

    const mo = p === "speaking" ? Math.min(1, voice * 1.5) : p === "listening" ? 0.3 : 0;
    const smile = p === "error" ? 0 : p === "thinking" ? 1 : 6;
    const w = p === "listening" ? 4.5 : 7 - 2 * mo;
    const mx = CX + jx;
    const my = CY + 10;
    attr(mouth.current, "d", `M${mx - w},${my}Q${mx},${my + smile} ${mx + w},${my}Q${mx},${my + smile + 2 + 16 * mo} ${mx - w},${my}Z`);

    const g = sparks.current;
    if (g) {
      g.style.opacity = p === "thinking" ? "1" : "0";
      if (p === "thinking") {
        for (let i = 0; i < g.children.length; i++) {
          const an = t * 3 + i * 2.1;
          attr(g.children[i], "cx", (CX + Math.cos(an) * (R + 10 + i * 3)).toFixed(1));
          attr(g.children[i], "cy", (CY + Math.sin(an) * (R * 0.55 + 8 + i * 2)).toFixed(1));
        }
      }
    }
  });

  return (
    <Stage phase={phase}>
      <polygon ref={shadow} fill={INK} />
      <polygon ref={body} fill={bodyColor(phase, theme)} stroke={INK} strokeWidth={3.2 * sw} strokeLinejoin="round" />
      {[0, 1].map((i) => (
        <g key={i}>
          <ellipse ref={(el) => void (eyes.current[i] = el)} rx={6.5} ry={8.5} fill="#FFFFFF" stroke={INK} strokeWidth={2.4 * sw} />
          <circle ref={(el) => void (pupils.current[i] = el)} r={3.4} fill={INK} />
          <path ref={(el) => void (crosses.current[i] = el)} fill="none" stroke={INK} strokeWidth={2.6 * sw} strokeLinecap="round" style={{ display: "none" }} />
        </g>
      ))}
      <path ref={mouth} fill={INK} stroke={INK} strokeWidth={2 * sw} strokeLinejoin="round" />
      <g ref={sparks} fill="#FFD25E" stroke={INK} strokeWidth={1.6 * sw} style={{ opacity: 0 }}>
        <circle r={3} />
        <circle r={2.5} />
        <circle r={2} />
      </g>
    </Stage>
  );
}

/** Pulso: un círculo con ecualizador de voz en el centro. */
function Pulso({ phase, theme, level, sw }: AvatarProps) {
  const shadow = useRef<SVGCircleElement>(null);
  const body = useRef<SVGCircleElement>(null);
  const ring = useRef<SVGCircleElement>(null);
  const bars = useRef<(SVGRectElement | null)[]>([]);

  useAnim(phase, level, ({ t, p, lv, voice }) => {
    const jx = p === "error" ? Math.sin(t * 40) * 1.6 : 0;
    const r = R * (1 + 0.04 * voice + (p === "idle" ? 0.02 * lv : 0));
    attr(shadow.current, "cx", CX + 3.5 + jx);
    attr(shadow.current, "cy", CY + 4);
    attr(shadow.current, "r", r);
    attr(body.current, "cx", CX + jx);
    attr(body.current, "cy", CY);
    attr(body.current, "r", r);
    for (let i = 0; i < 5; i++) {
      const dx = (i - 2) * 11.5;
      let h = 6;
      if (p === "idle") h = 9 + 7 * (0.5 + 0.5 * Math.sin(t * 1.8 + i * 0.9));
      else if (p === "listening" || p === "speaking") h = 9 + 34 * voice * (0.45 + 0.55 * Math.abs(Math.sin(t * 10 + i * 1.9)));
      else if (p === "thinking") h = 10 + 18 * (0.5 + 0.5 * Math.sin(t * 8 - i * 1.2));
      h = Math.min(h, 2 * Math.sqrt(Math.max(0, (r - 7) ** 2 - dx * dx)));
      const b = bars.current[i];
      attr(b, "x", CX + jx + dx - 3.5);
      attr(b, "y", CY - h / 2);
      attr(b, "height", h);
    }
    const rg = ring.current;
    if (rg) {
      rg.style.opacity = p === "thinking" ? "1" : "0";
      attr(rg, "transform", `rotate(${((t * 90) % 360).toFixed(1)} ${CX} ${CY})`);
    }
  });

  return (
    <Stage phase={phase}>
      <circle ref={shadow} fill={INK} />
      <circle ref={body} fill={bodyColor(phase, theme)} stroke={INK} strokeWidth={3.2 * sw} />
      <circle ref={ring} cx={CX} cy={CY} r={R + 8} fill="none" stroke={INK} strokeWidth={2.4 * sw} strokeDasharray="5 7" strokeLinecap="round" style={{ opacity: 0 }} />
      {[0, 1, 2, 3, 4].map((i) => (
        <rect key={i} ref={(el) => void (bars.current[i] = el)} width={7} rx={3.5} fill={innerColor(phase)} stroke={INK} strokeWidth={2.2 * sw} />
      ))}
    </Stage>
  );
}

/** Órbita: un planeta con anillo y una luna que le da vueltas. */
function Orbita({ phase, theme, level, sw }: AvatarProps) {
  const ringG = useRef<SVGGElement>(null);
  const ringF = useRef<SVGGElement>(null);
  const ringParts = useRef<(SVGPathElement | null)[]>([]);
  const moonBack = useRef<SVGCircleElement>(null);
  const moonFront = useRef<SVGCircleElement>(null);
  const planet = useRef<SVGGElement>(null);
  const planetBody = useRef<SVGCircleElement>(null);
  const planetShadow = useRef<SVGCircleElement>(null);
  const st = useRef({ phi: 0.6 });

  useAnim(phase, level, ({ t, dt, p, lv, voice }) => {
    const s = st.current;
    const sp = p === "idle" ? 0.9 : p === "listening" ? 1.4 : p === "thinking" ? 5 : p === "speaking" ? 2 : 0;
    s.phi += sp * dt;
    const jx = p === "error" ? Math.sin(t * 40) * 1.6 : 0;
    const tilt = p === "error" ? -62 + Math.sin(t * 30) * 3 : -22 + (p === "listening" ? 6 * Math.sin(t * 2) : 0) + (p === "thinking" ? 8 * Math.sin(t * 3) : 0);
    const rx = 44 * (1 + 0.04 * voice);
    const ry = 12;
    const back = `M${CX - rx},${CY}A${rx} ${ry} 0 0 1 ${CX + rx},${CY}`;
    const front = `M${CX + rx},${CY}A${rx} ${ry} 0 0 1 ${CX - rx},${CY}`;
    ringParts.current.forEach((el, i) => attr(el, "d", i < 2 ? back : front));
    const tr = `translate(${jx} 0) rotate(${tilt.toFixed(1)} ${CX} ${CY})`;
    attr(ringG.current, "transform", tr);
    attr(ringF.current, "transform", tr);

    const pr = 21 * (1 + 0.06 * voice + (p === "idle" ? 0.015 * lv : 0));
    attr(planetBody.current, "r", pr);
    attr(planetShadow.current, "r", pr);
    attr(planet.current, "transform", `translate(${jx} 0)`);

    const mx = rx * Math.cos(s.phi);
    const my = ry * Math.sin(s.phi);
    const a = (tilt * Math.PI) / 180;
    const x = CX + jx + mx * Math.cos(a) - my * Math.sin(a);
    const y = CY + mx * Math.sin(a) + my * Math.cos(a);
    const front_ = Math.sin(s.phi) > 0;
    for (const [el, show] of [[moonBack.current, !front_], [moonFront.current, front_]] as const) {
      attr(el, "cx", x.toFixed(1));
      attr(el, "cy", y.toFixed(1));
      if (el) el.style.display = show ? "" : "none";
    }
  });

  const band = innerColor(phase);
  const part = (i: number, outline: boolean) => (
    <path
      key={i}
      ref={(el) => void (ringParts.current[i] = el)}
      fill="none"
      stroke={outline ? INK : band}
      strokeWidth={(outline ? 9 : 4.6) * sw}
      strokeLinecap="butt"
    />
  );
  return (
    <Stage phase={phase}>
      <g ref={ringG}>
        {part(0, true)}
        {part(1, false)}
      </g>
      <circle ref={moonBack} r={5.5} fill="#FFD25E" stroke={INK} strokeWidth={2.4 * sw} />
      <g ref={planet}>
        <circle ref={planetShadow} cx={CX + 3.5} cy={CY + 4} fill={INK} />
        <circle ref={planetBody} cx={CX} cy={CY} fill={bodyColor(phase, theme)} stroke={INK} strokeWidth={3.2 * sw} />
        <ellipse cx={CX - 8} cy={CY - 8} rx={5.5} ry={3} fill="#FFFFFF" opacity={0.55} transform={`rotate(-35 ${CX - 8} ${CY - 8})`} />
      </g>
      <g ref={ringF}>
        {part(2, true)}
        {part(3, false)}
      </g>
      <circle ref={moonFront} r={5.5} fill="#FFD25E" stroke={INK} strokeWidth={2.4 * sw} />
    </Stage>
  );
}

// ------------------------------------------------------------------ selección

export const AVATARS: { id: string; name: string; text: string }[] = [
  { id: "formas", name: "Formas", text: "Analítica · «Veamos los datos»" },
  { id: "gelatina", name: "Gelatina", text: "Divertida y optimista · «Eso estuvo de cabra»" },
  { id: "pulso", name: "Pulso", text: "Cálida · «Con calma, paso a paso»" },
  { id: "orbita", name: "Órbita", text: "Creativa · «¿Y si…?»" },
];

/** Avatar de ZENOS. `level` (0..1) es el volumen del micrófono o de la voz; se lee en cada fotograma. */
export function Orb({ phase, theme, level, avatar = "formas", sw = 1 }: { phase: Phase; theme: string; level: MutableRefObject<number>; avatar?: string; sw?: number }) {
  const props = { phase, theme, level, sw };
  switch (avatar) {
    case "gelatina":
      return <Gelatina {...props} />;
    case "pulso":
      return <Pulso {...props} />;
    case "orbita":
      return <Orbita {...props} />;
    default:
      return <GeoOrb {...props} />;
  }
}

const ZERO: MutableRefObject<number> = { current: 0 };

/** Versión pequeña y siempre animada del avatar, para la interfaz. */
export function AvatarIcon({ size = 64, theme = "menta", avatar = "formas", phase = "idle" }: { size?: number; theme?: string; avatar?: string; phase?: Phase }) {
  return (
    <div style={{ width: size, height: size, flex: "none" }} aria-hidden="true">
      <Orb phase={phase} theme={theme} level={ZERO} avatar={avatar} sw={size < 70 ? 1.5 : size < 100 ? 1.25 : 1} />
    </div>
  );
}
