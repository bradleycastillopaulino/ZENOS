import { forwardRef, type CSSProperties } from "react";
import "./orb.css";

export type Phase = "hidden" | "idle" | "listening" | "thinking" | "speaking" | "error";

const PARTICLES = Array.from({ length: 14 }, (_, i) => i);

/** Esfera de energía. Todo el movimiento es CSS; `--level` (0..1) llega desde fuera. */
export const Orb = forwardRef<HTMLDivElement, { phase: Phase }>(function Orb({ phase }, ref) {
  return (
    <div className="stage" data-phase={phase} ref={ref}>
      <div className="aura" />
      <div className="ripples">
        <i />
        <i />
        <i />
      </div>
      <div className="particles calm">
        {PARTICLES.map((i) => (
          <span key={i} style={{ "--i": i } as CSSProperties} />
        ))}
      </div>
      <div className="orb-wrap">
        <div className="orb">
          <div className="body" />
          <div className="plasma a" />
          <div className="plasma b" />
          <div className="plasma c" />
          <div className="swirl" />
          <div className="filament f1" />
          <div className="filament f2" />
          <div className="filament f3" />
          <div className="sheen" />
          <div className="rim" />
        </div>
      </div>
      <div className="particles busy">
        {PARTICLES.slice(0, 10).map((i) => (
          <span key={i} style={{ "--i": i } as CSSProperties} />
        ))}
      </div>
    </div>
  );
});
