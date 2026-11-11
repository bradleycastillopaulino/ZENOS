import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type AppConfig } from "../api";
import "./overlay.css";

function useCorner(): string {
  const [corner, setCorner] = useState("br");
  useEffect(() => {
    document.documentElement.classList.add("overlay");
    void api.getConfig().then((c) => setCorner(c.orbCorner));
    const u = listen<AppConfig>("zenos://config", (e) => setCorner(e.payload.orbCorner));
    return () => void u.then((f) => f());
  }, []);
  return corner;
}

/** Recuadro de lectura: muestra lo que dice ZENOS. Transparente y sin interacción. */
export function Bubble() {
  const corner = useCorner();
  const [user, setUser] = useState("");
  const [answer, setAnswer] = useState("");
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    const u = listen<{ kind: string; text: string }>("zenos://bubble", (e) => {
      window.clearTimeout(timer.current);
      const { kind, text } = e.payload;
      if (kind === "user") {
        setUser(text);
        setAnswer("");
      } else if (kind === "zenos") {
        setAnswer((a) => (a ? `${a} ${text}` : text));
      } else if (kind === "end") {
        timer.current = window.setTimeout(() => void api.hideBubble(), 5000);
      }
    });
    return () => void u.then((f) => f());
  }, []);

  return (
    <div className={`bubble-wrap ${corner.startsWith("b") ? "bottom" : "top"} ${corner.endsWith("r") ? "right" : "left"}`}>
      {(user || answer) && (
        <div className="bubble">
          {user && <div className="you">{user}</div>}
          {answer && <div className="zenos">{answer}</div>}
        </div>
      )}
    </div>
  );
}

/** Cuadro para escribirle a ZENOS a mano. */
export function TypeBox() {
  const corner = useCorner();
  const [text, setText] = useState("");
  const [mic, setMic] = useState(true);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void api.hasMicrophone().then(setMic);
    const u = listen("zenos://typebox-open", () => {
      setText("");
      setTimeout(() => input.current?.focus(), 50);
    });
    return () => void u.then((f) => f());
  }, []);

  const send = () => {
    if (text.trim()) void api.submitText(text);
    setText("");
  };

  return (
    <div className={`typebox-wrap ${corner.startsWith("b") ? "bottom" : "top"} ${corner.endsWith("r") ? "right" : "left"}`}>
      <div className="typebox">
        <input
          ref={input}
          autoFocus
          spellCheck={false}
          placeholder="Escríbele a ZENOS…"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") send();
            if (e.key === "Escape") void api.closeTypebox();
          }}
        />
        {mic && (
          <button className="mic" title="Hablar en vez de escribir" aria-label="Hablar" onClick={() => void api.startVoice()}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#1e1b2e" strokeWidth="2.6" strokeLinecap="round">
              <rect x="9" y="3" width="6" height="11" rx="3" />
              <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
            </svg>
          </button>
        )}
        <button onClick={send} disabled={!text.trim()}>
          Enviar
        </button>
      </div>
      <div className="tip">Enter envía · Esc cierra</div>
    </div>
  );
}
