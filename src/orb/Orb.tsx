import { forwardRef } from "react";
import "./orb.css";

export type Phase = "hidden" | "idle" | "listening" | "thinking" | "speaking" | "error";

/** Esfera oscura y abstracta. Todo el movimiento es CSS; `--level` (0..1) llega desde fuera. */
export const Orb = forwardRef<HTMLDivElement, { phase: Phase }>(function Orb({ phase }, ref) {
  return (
    <div className="stage" data-phase={phase} ref={ref}>
      <div className="halo" />
      <div className="ring" />
      <div className="ring r2" />
      <div className="core">
        <div className="blob a" />
        <div className="blob b" />
        <div className="blob c" />
        <div className="shade" />
      </div>
    </div>
  );
});
