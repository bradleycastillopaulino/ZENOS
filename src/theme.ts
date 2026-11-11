import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type AppConfig } from "./api";
import "./themes.css";

export interface UiTheme {
  id: string;
  name: string;
  dark: boolean;
  /** Colores de la miniatura: fondo, tarjeta y tinta. */
  bg: string;
  paper: string;
  ink: string;
}

export const UI_THEMES: UiTheme[] = [
  { id: "crema", name: "Crema", dark: false, bg: "#f7f0e1", paper: "#ffffff", ink: "#1e1b2e" },
  { id: "lavanda", name: "Lavanda", dark: false, bg: "#ece6fb", paper: "#ffffff", ink: "#1e1b2e" },
  { id: "durazno", name: "Durazno", dark: false, bg: "#fdebdc", paper: "#ffffff", ink: "#1e1b2e" },
  { id: "medianoche", name: "Medianoche", dark: true, bg: "#1b1830", paper: "#2d2955", ink: "#f4ecdc" },
  { id: "carbon", name: "Carbón", dark: true, bg: "#1c1c1f", paper: "#313136", ink: "#f1eee8" },
  { id: "bosque", name: "Bosque", dark: true, bg: "#13221e", paper: "#234038", ink: "#eef3e6" },
];

export function applyUiTheme(id: string) {
  const ok = UI_THEMES.some((t) => t.id === id);
  document.documentElement.dataset.ui = ok ? id : "crema";
}

export function applyReadSize(size: number) {
  document.documentElement.style.setProperty("--read", String(size));
}

/** Para ventanas sin su propio estado de configuración (confirmación, recuadros): aplica el tema y el tamaño de lectura. */
export function useAppConfig(): AppConfig | null {
  const [cfg, setCfg] = useState<AppConfig | null>(null);
  useEffect(() => {
    const apply = (c: AppConfig) => {
      applyUiTheme(c.uiTheme);
      applyReadSize(c.readSize);
      setCfg(c);
    };
    void api.getConfig().then(apply);
    const u = listen<AppConfig>("zenos://config", (e) => apply(e.payload));
    return () => void u.then((f) => f());
  }, []);
  return cfg;
}
