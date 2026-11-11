import { StrictMode, lazy, Suspense } from "react";
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Una sola página para las dos ventanas: la esfera ("orb"), la configuración y la confirmación.
const label = getCurrentWindow().label;
const View =
  label === "settings"
    ? lazy(() => import("./settings/Settings").then((m) => ({ default: m.Settings })))
    : label === "confirm"
      ? lazy(() => import("./settings/Confirm").then((m) => ({ default: m.Confirm })))
      : lazy(() => import("./orb/OrbApp").then((m) => ({ default: m.OrbApp })));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Suspense fallback={null}>
      <View />
    </Suspense>
  </StrictMode>,
);
