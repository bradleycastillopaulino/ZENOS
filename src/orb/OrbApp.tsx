import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText, type AppConfig } from "../api";
import { Orb, type Phase } from "./Orb";

const HIDE_DELAY_MS = 700;

/**
 * Ciclo del asistente: escuchar → procesar → hablar → reposo.
 * El atajo inicia el ciclo; pulsarlo de nuevo (o Esc) lo cancela.
 */
export function OrbApp() {
  const [phase, setPhase] = useState<Phase>("hidden");
  const phaseRef = useRef<Phase>("hidden");
  const [theme, setTheme] = useState("menta");
  const generation = useRef(0);
  const config = useRef<AppConfig | null>(null);
  const micLevel = useRef(0);
  const hideTimer = useRef<number | undefined>(undefined);

  const go = useCallback((p: Phase) => {
    phaseRef.current = p;
    setPhase(p);
  }, []);

  const rest = useCallback(
    (gen: number) => {
      if (gen !== generation.current) return;
      void api.setActive(false);
      go("idle");
      window.clearTimeout(hideTimer.current);
      hideTimer.current = window.setTimeout(() => {
        if (gen !== generation.current || config.current?.orbAlwaysVisible) return;
        go("hidden");
        window.setTimeout(() => {
          if (phaseRef.current === "hidden") void api.hideOrb();
        }, 550);
      }, HIDE_DELAY_MS);
    },
    [go],
  );

  const say = useCallback(async (chunks: string[]) => {
    await api.speak(chunks);
  }, []);

  const run = useCallback(async () => {
    const gen = ++generation.current;
    window.clearTimeout(hideTimer.current);
    void api.setActive(true);
    micLevel.current = 0;
    go("listening");
    try {
      const heard = await api.listen();
      if (gen !== generation.current) return;
      if (heard.kind !== "text") return rest(gen);

      go("thinking");
      const reply = await api.ask(heard.text);
      if (gen !== generation.current) return;

      go("speaking");
      micLevel.current = 0;
      await say(reply.chunks);
    } catch (e) {
      if (gen !== generation.current) return;
      go("error");
      try {
        await say([errorText(e)]);
      } catch {
        await new Promise((r) => setTimeout(r, 1500));
      }
    } finally {
      rest(gen);
    }
  }, [go, rest, say]);

  const cancel = useCallback(() => {
    const gen = ++generation.current;
    void api.cancelListen();
    void api.stopSpeaking();
    rest(gen);
  }, [rest]);

  useEffect(() => {
    document.documentElement.classList.add("orb");
    void api.getConfig().then((c) => {
      config.current = c;
      setTheme(c.theme);
      if (c.orbAlwaysVisible) go("idle");
    });
    const unlisten = [
      listen("zenos://hotkey", () => {
        const p = phaseRef.current;
        if (p === "hidden" || p === "idle") void run();
        else if (config.current?.hotkeyMode !== "hold") cancel();
      }),
      listen("zenos://hotkey-up", () => {
        if (config.current?.hotkeyMode === "hold" && phaseRef.current === "listening") void api.stopListen();
      }),
      listen("zenos://wake", () => {
        const p = phaseRef.current;
        if (p === "hidden" || p === "idle") void run();
      }),
      listen("zenos://cancel", () => cancel()),
      listen<number>("zenos://level", (e) => {
        micLevel.current = e.payload;
      }),
      listen("zenos://recorded", () => {
        if (phaseRef.current === "listening") go("thinking");
      }),
      listen<AppConfig>("zenos://config", (e) => {
        config.current = e.payload;
        setTheme(e.payload.theme);
        const p = phaseRef.current;
        if (e.payload.orbAlwaysVisible && p === "hidden") go("idle");
        if (!e.payload.orbAlwaysVisible && p === "idle") rest(generation.current);
      }),
    ];
    return () => unlisten.forEach((u) => void u.then((f) => f()));
  }, [run, cancel, go, rest]);

  return <Orb phase={phase} theme={theme} level={micLevel} />;
}
