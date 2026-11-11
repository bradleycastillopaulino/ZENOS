import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, errorText, type AppConfig, type ProviderInfo, type Voice } from "../api";
import { THEMES } from "../orb/Orb";
import { UI_THEMES, applyReadSize, applyUiTheme } from "../theme";
import "./settings.css";

type Note = { kind: "ok" | "error" | "info"; text: string } | null;

const STT_NAMES: Record<string, string> = { groq: "Groq", openai: "OpenAI" };
const KEY_LINKS: Record<string, string> = {
  groq: "https://console.groq.com/keys",
  openai: "https://platform.openai.com/api-keys",
};

function Message({ note }: { note: Note }) {
  if (!note) return null;
  return <div className={`note ${note.kind}`}>{note.text}</div>;
}

function Link({ url, children }: { url: string; children: ReactNode }) {
  return (
    <button className="link" onClick={() => void api.openUrl(url)}>
      {children}
    </button>
  );
}

/** Barra de nivel alimentada por los eventos de prueba de micrófono / altavoz. */
function LevelBar() {
  const fill = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const u = listen<number>("zenos://test-level", (e) => {
      if (fill.current) fill.current.style.width = `${Math.min(100, Math.round(e.payload * 100))}%`;
    });
    return () => void u.then((f) => f());
  }, []);
  return (
    <div className="level">
      <div ref={fill} />
    </div>
  );
}

// ------------------------------------------------------------------ dispositivos y voz

function Logo({ size = 64, theme = "menta" }: { size?: number; theme?: string }) {
  const c = THEMES[theme]?.body ?? THEMES.menta.body;
  return (
    <svg width={size} height={size} viewBox="0 0 100 100" aria-hidden="true">
      <polygon points="53,8 87,30 87,68 53,90 19,68 19,30" style={{ fill: "var(--ink)" }} transform="translate(4,5)" />
      <polygon points="50,6 84,28 84,66 50,88 16,66 16,28" fill={c} style={{ stroke: "var(--ink)" }} strokeWidth="4.5" strokeLinejoin="round" />
      <polygon points="50,26 68,37 68,57 50,68 32,57 32,37" fill="#FFF6D6" style={{ stroke: "var(--ink)" }} strokeWidth="3.5" strokeLinejoin="round" />
    </svg>
  );
}

function DevicePicker({ input, output, onChange }: { input: string; output: string; onChange: (input: string, output: string) => void }) {
  const [inputs, setInputs] = useState<string[]>([]);
  const [outputs, setOutputs] = useState<string[]>([]);
  const [micNote, setMicNote] = useState<Note>(null);
  const [spkNote, setSpkNote] = useState<Note>(null);
  const [busy, setBusy] = useState<"" | "mic" | "spk">("");

  useEffect(() => {
    void api.listDevices().then((d) => {
      setInputs(d.inputs);
      setOutputs(d.outputs);
    });
  }, []);

  const testMic = async () => {
    setBusy("mic");
    setMicNote({ kind: "info", text: "Habla ahora durante 3 segundos…" });
    try {
      const peak = await api.testMic(input);
      setMicNote(
        peak > 0.12
          ? { kind: "ok", text: "Te escucho bien." }
          : {
              kind: "error",
              text: "No detecto sonido. Prueba otro micrófono o revisa Configuración de Windows → Privacidad → Micrófono y activa «Permitir que las aplicaciones de escritorio accedan».",
            },
      );
    } catch (e) {
      setMicNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy("");
    }
  };
  const testSpeaker = async () => {
    setBusy("spk");
    setSpkNote({ kind: "info", text: "Reproduciendo…" });
    try {
      await api.testSpeaker(output, "");
      setSpkNote({ kind: "ok", text: "¿Me escuchaste? Si no, elige otro altavoz." });
    } catch (e) {
      setSpkNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy("");
    }
  };

  return (
    <>
      <div className="field">
        <label>Micrófono</label>
        <div className="row">
          <select value={input} onChange={(e) => onChange(e.target.value, output)}>
            <option value="">Predeterminado de Windows</option>
            {inputs.map((n) => (
              <option key={n}>{n}</option>
            ))}
          </select>
          <button className="ghost" disabled={busy !== ""} onClick={() => void testMic()}>
            Probar
          </button>
        </div>
        {busy === "mic" && <LevelBar />}
        <Message note={micNote} />
      </div>
      <div className="field">
        <label>Altavoz</label>
        <div className="row">
          <select value={output} onChange={(e) => onChange(input, e.target.value)}>
            <option value="">Predeterminado de Windows</option>
            {outputs.map((n) => (
              <option key={n}>{n}</option>
            ))}
          </select>
          <button className="ghost" disabled={busy !== ""} onClick={() => void testSpeaker()}>
            Probar
          </button>
        </div>
        {busy === "spk" && <LevelBar />}
        <Message note={spkNote} />
      </div>
    </>
  );
}

function VoicePicker({ voice, output, onChange }: { voice: string; output: string; onChange: (v: string) => void }) {
  const [voices, setVoices] = useState<Voice[]>([]);
  const [note, setNote] = useState<Note>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    void api.listVoices().then(setVoices).catch(() => setVoices([]));
  }, []);

  const current = voice === "" ? "edge:dalia" : voice.startsWith("edge:") || voice.startsWith("win:") ? voice : `win:${voice}`;
  const neural = voices.filter((v) => v.kind === "neural");
  const windows = voices.filter((v) => v.kind === "windows");

  const test = async () => {
    setBusy(true);
    setNote({ kind: "info", text: "Generando la voz…" });
    try {
      await api.testSpeaker(output, current);
      setNote({ kind: "ok", text: "Si no sonó natural, puede que el servicio de voces no esté disponible y se usó la voz de Windows." });
    } catch (e) {
      setNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="field">
      <label>Voz del asistente</label>
      <div className="row">
        <select value={current} onChange={(e) => onChange(e.target.value)}>
          <optgroup label="Voces neuronales (necesitan internet)">
            {neural.map((v) => (
              <option key={v.id} value={v.id}>
                {v.label}
              </option>
            ))}
          </optgroup>
          {windows.length > 0 && (
            <optgroup label="Voces de Windows (sin internet)">
              {windows.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.label}
                </option>
              ))}
            </optgroup>
          )}
        </select>
        <button className="ghost" disabled={busy} onClick={() => void test()}>
          Escuchar
        </button>
      </div>
      {busy && <LevelBar />}
      <Message note={note} />
    </div>
  );
}

function ThemePicker({ value, onChange }: { value: string; onChange: (t: string) => void }) {
  return (
    <div className="swatches">
      {Object.entries(THEMES).map(([id, t]) => (
        <button key={id} className={id === value ? "swatch on" : "swatch"} style={{ background: t.body }} title={t.label} aria-label={t.label} onClick={() => onChange(id)} />
      ))}
    </div>
  );
}

function UiThemePicker({ value, onChange }: { value: string; onChange: (t: string) => void }) {
  const group = (dark: boolean, title: string) => (
    <>
      <div className="ui-group">{title}</div>
      <div className="ui-themes">
        {UI_THEMES.filter((t) => t.dark === dark).map((t) => (
          <button key={t.id} className={t.id === value ? "ui-theme on" : "ui-theme"} aria-pressed={t.id === value} onClick={() => onChange(t.id)}>
            <span className="ui-prev" style={{ background: t.bg, borderColor: t.ink }}>
              <i style={{ background: "#62d9b4", borderColor: t.ink }} />
              <i style={{ background: t.paper, borderColor: t.ink }} />
              <i style={{ background: "#ffd25e", borderColor: t.ink }} />
            </span>
            {t.name}
          </button>
        ))}
      </div>
    </>
  );
  return (
    <>
      {group(false, "Claros")}
      {group(true, "Oscuros")}
    </>
  );
}

const LEVELS: { id: AppConfig["permissionLevel"]; name: string; text: string }[] = [
  { id: "estricto", name: "Estricto", text: "Pregunta antes de cualquier cambio: abrir programas, crear o mover archivos, borrar o ejecutar comandos." },
  { id: "normal", name: "Normal (recomendado)", text: "Abre programas y crea carpetas solo. Pregunta antes de borrar, sobrescribir archivos o ejecutar comandos." },
  { id: "libre", name: "Libre", text: "Hace todo sin preguntar. Lo borrado va a la Papelera, pero un comando erróneo puede causar daños." },
];

function PermissionPicker({ value, onChange }: { value: AppConfig["permissionLevel"]; onChange: (v: AppConfig["permissionLevel"]) => void }) {
  return (
    <div className="levels">
      {LEVELS.map((l) => (
        <button key={l.id} className={l.id === value ? "level-card on" : "level-card"} onClick={() => onChange(l.id)}>
          <strong>{l.name}</strong>
          <span>{l.text}</span>
        </button>
      ))}
    </div>
  );
}

// ------------------------------------------------------------------ clave de IA / voz

function ProviderKey({
  providers,
  onDone,
  onPick,
  initial,
}: {
  providers: ProviderInfo[];
  initial: string;
  onDone: (provider: string, model: string) => void;
  onPick?: () => void;
}) {
  const [id, setId] = useState(initial);
  const [key, setKey] = useState("");
  const [note, setNote] = useState<Note>(null);
  const [busy, setBusy] = useState(false);
  const info = providers.find((p) => p.id === id);

  const verify = async () => {
    setBusy(true);
    setNote({ kind: "info", text: "Verificando la clave y eligiendo el mejor modelo…" });
    try {
      const model = await api.setupProvider(id, key);
      setKey("");
      setNote({ kind: "ok", text: `Listo. Modelo elegido automáticamente: ${model}` });
      onDone(id, model);
    } catch (e) {
      setNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div className="providers">
        {providers.map((p) => (
          <button key={p.id} className={p.id === id ? "card sel" : "card"} onClick={() => {
              setId(p.id);
              onPick?.();
            }}>
            {p.name}
          </button>
        ))}
      </div>
      <div className="field">
        <label>Clave API de {info?.name}</label>
        <div className="row">
          <input
            type="password"
            autoComplete="off"
            spellCheck={false}
            placeholder="Pega aquí tu clave"
            value={key}
            onChange={(e) => setKey(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && key.trim() && !busy && void verify()}
          />
          <button disabled={!key.trim() || busy} onClick={() => void verify()}>
            Verificar
          </button>
        </div>
        {info && (
          <p className="hint">
            ¿No tienes clave? <Link url={info.keyUrl}>Obtenerla en {info.name}</Link>. Se guarda cifrada en el
            Administrador de credenciales de Windows.
          </p>
        )}
        <Message note={note} />
      </div>
    </>
  );
}

function SttKey({ onDone, current }: { current: string; onDone: () => void }) {
  const [provider, setProvider] = useState(current === "openai" ? "openai" : "groq");
  const [key, setKey] = useState("");
  const [note, setNote] = useState<Note>(null);
  const [busy, setBusy] = useState(false);
  const verify = async () => {
    setBusy(true);
    setNote({ kind: "info", text: "Verificando…" });
    try {
      await api.setupStt(provider, key);
      setKey("");
      setNote({ kind: "ok", text: "Clave de reconocimiento de voz guardada." });
      onDone();
    } catch (e) {
      setNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="field">
      <label>Servicio de reconocimiento de voz (Whisper)</label>
      <div className="row">
        <select value={provider} onChange={(e) => setProvider(e.target.value)}>
          <option value="groq">Groq (recomendado, tiene plan gratuito)</option>
          <option value="openai">OpenAI</option>
        </select>
      </div>
      <div className="row">
        <input
          type="password"
          autoComplete="off"
          spellCheck={false}
          placeholder={`Clave API de ${STT_NAMES[provider]}`}
          value={key}
          onChange={(e) => setKey(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && key.trim() && !busy && void verify()}
        />
        <button disabled={!key.trim() || busy} onClick={() => void verify()}>
          Verificar
        </button>
      </div>
      <p className="hint">
        Convierte tu voz en texto. <Link url={KEY_LINKS[provider]}>Obtener clave de {STT_NAMES[provider]}</Link>
      </p>
      <Message note={note} />
    </div>
  );
}

// ------------------------------------------------------------------ atajo

function codeToKey(code: string): string | null {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (["Space", "Enter", "Tab", "Backspace"].includes(code)) return code;
  return null;
}

function HotkeyInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <input
      readOnly
      className="hotkey"
      value={value}
      title="Haz clic y pulsa la combinación"
      onKeyDown={(e) => {
        e.preventDefault();
        const key = codeToKey(e.code);
        if (!key) return;
        const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
        if (!mods.length && !key.startsWith("F")) return;
        onChange([...mods, key].join("+"));
      }}
    />
  );
}

// ------------------------------------------------------------------ asistente inicial

function Wizard({ initial, providers, onFinish }: { initial: AppConfig; providers: ProviderInfo[]; onFinish: () => void }) {
  const [step, setStep] = useState(0);
  const [cfg, setCfg] = useState(initial);
  const [provider, setProvider] = useState("");
  const [chatDone, setChatDone] = useState(false);
  const [sttDone, setSttDone] = useState(false);
  const [autostart, setAutostart] = useState(true);
  const [err, setErr] = useState("");
  useEffect(() => {
    applyUiTheme(cfg.uiTheme);
  }, [cfg.uiTheme]);
  const set = <K extends keyof AppConfig>(k: K, v: AppConfig[K]) => setCfg((c) => ({ ...c, [k]: v }));

  const needsStt = provider !== "" && provider !== "groq" && provider !== "openai";
  const steps = ["Bienvenida", "Audio", "IA", ...(needsStt ? ["Reconocimiento"] : []), "Voz", "Estilo", "Permisos", "Listo"];
  const name = steps[step];

  const finish = async () => {
    try {
      const saved = await api.getConfig(); // trae proveedor y modelo guardados por el asistente
      await api.saveConfig({
        ...saved,
        inputDevice: cfg.inputDevice,
        outputDevice: cfg.outputDevice,
        voice: cfg.voice,
        theme: cfg.theme,
        permissionLevel: cfg.permissionLevel,
        setupDone: true,
      });
      await api.setAutostart(autostart);
      await api.showOrb();
      onFinish();
    } catch (e) {
      setErr(errorText(e));
    }
  };

  const canNext = name === "IA" ? chatDone : name === "Reconocimiento" ? sttDone : true;

  return (
    <div className="wizard">
      <div className="dots">
        {steps.map((s, i) => (
          <i key={s} className={i === step ? "on" : i < step ? "past" : ""} />
        ))}
      </div>

      <div className="page" key={name}>
        {name === "Bienvenida" && (
          <div className="center">
            <Logo size={110} theme={cfg.theme} />
            <h1>ZENOS</h1>
            <p className="lead">
              Tu asistente de voz. Pulsa <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Espacio</kbd>, habla y escucha la respuesta. También puede abrir programas y manejar tus archivos.
            </p>
            <p className="hint">Vamos a dejarlo listo en un minuto.</p>
          </div>
        )}

        {name === "Audio" && (
          <>
            <h2>Micrófono y altavoz</h2>
            <p className="hint">Elige cuáles usar y pruébalos.</p>
            <DevicePicker input={cfg.inputDevice} output={cfg.outputDevice} onChange={(i, o) => setCfg((c) => ({ ...c, inputDevice: i, outputDevice: o }))} />
          </>
        )}

        {name === "IA" && (
          <>
            <h2>Elige tu IA</h2>
            <p className="hint">ZENOS elegirá el modelo automáticamente.</p>
            <ProviderKey
              providers={providers}
              initial={provider || initial.provider}
              onPick={() => setChatDone(false)}
              onDone={(p) => {
                setProvider(p);
                setChatDone(true);
              }}
            />
          </>
        )}

        {name === "Reconocimiento" && (
          <>
            <h2>Reconocimiento de voz</h2>
            <p className="hint">Tu IA no transcribe audio, así que usamos Whisper en la nube.</p>
            <SttKey current={initial.sttProvider} onDone={() => setSttDone(true)} />
          </>
        )}

        {name === "Voz" && (
          <>
            <h2>Elige su voz</h2>
            <p className="hint">Escucha cada una y quédate con la que más te guste.</p>
            <VoicePicker voice={cfg.voice} output={cfg.outputDevice} onChange={(v) => set("voice", v)} />
          </>
        )}

        {name === "Estilo" && (
          <>
            <h2>Color de tu orbe</h2>
            <p className="hint">Puedes cambiarlo cuando quieras en Apariencia.</p>
            <div className="center">
              <Logo size={96} theme={cfg.theme} />
            </div>
            <ThemePicker value={cfg.theme} onChange={(t) => set("theme", t)} />
            <h2 style={{ marginTop: 22 }}>Tema de la interfaz</h2>
            <UiThemePicker value={cfg.uiTheme} onChange={(t) => set("uiTheme", t)} />
          </>
        )}

        {name === "Permisos" && (
          <>
            <h2>¿Cuánto control le das?</h2>
            <p className="hint">Puede abrir programas, crear carpetas y mover archivos. Lo que borra va a la Papelera y las carpetas del sistema están protegidas.</p>
            <PermissionPicker value={cfg.permissionLevel} onChange={(v) => set("permissionLevel", v)} />
          </>
        )}

        {name === "Listo" && (
          <div className="center">
            <Logo size={96} theme={cfg.theme} />
            <h2>Todo listo</h2>
            <p className="lead">
              Pulsa <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Espacio</kbd> cuando quieras. Para cancelar, pulsa <kbd>Esc</kbd>.
            </p>
            <label className="check">
              <input type="checkbox" checked={autostart} onChange={(e) => setAutostart(e.target.checked)} />
              Iniciar ZENOS con Windows
            </label>
            {err && <div className="note error">{err}</div>}
          </div>
        )}
      </div>

      <div className="nav">
        {step > 0 && name !== "Listo" ? (
          <button className="ghost" onClick={() => setStep(step - 1)}>
            Atrás
          </button>
        ) : (
          <span />
        )}
        {name === "Listo" ? (
          <button onClick={() => void finish()}>Empezar a usar ZENOS</button>
        ) : (
          <button disabled={!canNext} onClick={() => setStep(step + 1)}>
            {name === "Bienvenida" ? "Empezar" : "Siguiente"}
          </button>
        )}
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ panel

type Page = "inicio" | "ia" | "voz" | "audio" | "texto" | "apariencia" | "permisos" | "atajo" | "ayuda";
const PAGES: { id: Page; name: string }[] = [
  { id: "inicio", name: "Inicio" },
  { id: "ia", name: "Inteligencia" },
  { id: "voz", name: "Voz" },
  { id: "audio", name: "Audio" },
  { id: "texto", name: "Texto" },
  { id: "apariencia", name: "Apariencia" },
  { id: "permisos", name: "Permisos" },
  { id: "atajo", name: "Atajo" },
  { id: "ayuda", name: "Ayuda" },
];
const CORNERS: [string, string][] = [["tl", "Arriba izquierda"], ["tr", "Arriba derecha"], ["bl", "Abajo izquierda"], ["br", "Abajo derecha"]];

function Panel({ initial, providers, rerun }: { initial: AppConfig; providers: ProviderInfo[]; rerun: () => void }) {
  const [page, setPage] = useState<Page>("inicio");
  const [cfg, setCfg] = useState(initial);
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState<Note>(null);
  const [keys, setKeys] = useState<Record<string, boolean>>({});
  const [autostart, setAutostart] = useState(false);
  const [logFile, setLogFile] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [aiNote, setAiNote] = useState<Note>(null);
  const [hasMic, setHasMic] = useState(true);

  const refreshKeys = useCallback(() => {
    void api.keyStatus().then((l) => setKeys(Object.fromEntries(l.map((k) => [k.provider, k.saved]))));
  }, []);
  useEffect(() => {
    refreshKeys();
    void api.getAutostart().then(setAutostart);
    void api.logPath().then(setLogFile);
    void api.hasMicrophone().then(setHasMic);
  }, [refreshKeys]);

  useEffect(() => {
    applyUiTheme(cfg.uiTheme);
    applyReadSize(cfg.readSize);
  }, [cfg.uiTheme, cfg.readSize]);
  const info = providers.find((p) => p.id === cfg.provider);
  const set = <K extends keyof AppConfig>(k: K, v: AppConfig[K]) => {
    setCfg((c) => ({ ...c, [k]: v }));
    setDirty(true);
    setSaved(null);
  };

  const save = async () => {
    try {
      const out = await api.saveConfig(cfg);
      await api.setAutostart(autostart);
      setCfg(out);
      setDirty(false);
      setSaved({ kind: "ok", text: "Cambios guardados." });
    } catch (e) {
      setSaved({ kind: "error", text: errorText(e) });
    }
  };

  const testAi = async () => {
    setAiNote({ kind: "info", text: "Probando…" });
    try {
      await api.saveConfig(cfg);
      const r = await api.testProvider();
      setAiNote({ kind: "ok", text: `Respuesta: ${r}` });
    } catch (e) {
      setAiNote({ kind: "error", text: errorText(e) });
    }
  };

  const needsStt = cfg.provider !== "groq" && cfg.provider !== "openai";

  return (
    <div className="shell">
      <nav>
        <div className="brand">
          <Logo size={54} theme={cfg.theme} />
          <b>ZENOS</b>
        </div>
        {PAGES.map((p) => (
          <button key={p.id} className={p.id === page ? "nv on" : "nv"} onClick={() => setPage(p.id)}>
            {p.name}
          </button>
        ))}
      </nav>

      <main>
        <div className="content" key={page}>
          {page === "inicio" && (
            <>
              <h2>Inicio</h2>
              <div className="card">
                <div className="lbl">Inteligencia</div>
                <b>{info?.name}</b> · {cfg.model || info?.defaultModel} · clave {keys[cfg.provider] ? "guardada" : <span className="bad">falta</span>}
              </div>
              <div className="card">
                <div className="lbl">Reconocimiento de voz</div>
                <b>{STT_NAMES[cfg.sttProvider]}</b> · clave {keys[cfg.sttProvider] ? "guardada" : <span className="bad">falta</span>}
              </div>
              <div className="card">
                <div className="lbl">Cómo hablarle</div>
                {cfg.hotkey.split("+").map((k, i) => (
                  <span key={k + i}>
                    {i > 0 && " + "}
                    <kbd>{k}</kbd>
                  </span>
                ))}{" "}
                {cfg.hotkeyMode === "hold" ? "mantén pulsado mientras hablas" : "pulsa para empezar y de nuevo para cancelar"}
                {cfg.wakeWord && " · o di «Oye Zenos»"}
              </div>
              <div className="card">
                <div className="lbl">Control del PC</div>
                {cfg.actionsEnabled ? `Activado · nivel ${cfg.permissionLevel}` : "Desactivado: solo conversa"}
              </div>
            </>
          )}

          {page === "ia" && (
            <>
              <h2>Inteligencia artificial</h2>
              <div className="field">
                <label>Proveedor</label>
                <select value={cfg.provider} onChange={(e) => { set("provider", e.target.value); set("model", ""); }}>
                  {providers.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name} {keys[p.id] ? "· clave guardada" : ""}
                    </option>
                  ))}
                </select>
              </div>
              <div className="field">
                <label>Modelo</label>
                <input list="models" value={cfg.model} placeholder={`Automático (${info?.defaultModel ?? ""})`} onChange={(e) => set("model", e.target.value)} />
                <datalist id="models">
                  {info?.suggestedModels.map((m) => (
                    <option key={m} value={m} />
                  ))}
                </datalist>
                <p className="hint">Déjalo vacío para que ZENOS use el modelo recomendado.</p>
              </div>
              <div className="row">
                <button className="ghost" onClick={() => setShowKey(!showKey)}>
                  {keys[cfg.provider] ? "Cambiar clave" : "Añadir clave"}
                </button>
                <button className="ghost" onClick={() => void testAi()}>
                  Probar IA
                </button>
              </div>
              <Message note={aiNote} />
              {showKey && (
                <ProviderKey
                  key={cfg.provider}
                  providers={providers.filter((p) => p.id === cfg.provider)}
                  initial={cfg.provider}
                  onDone={(_, model) => {
                    set("model", model);
                    refreshKeys();
                    setShowKey(false);
                  }}
                />
              )}
              {needsStt && (
                <>
                  <h3>Reconocimiento de voz</h3>
                  <p className="hint">Servicio actual: {STT_NAMES[cfg.sttProvider]} · clave {keys[cfg.sttProvider] ? "guardada" : "falta"}</p>
                  <SttKey current={cfg.sttProvider} onDone={refreshKeys} />
                </>
              )}
            </>
          )}

          {page === "voz" && (
            <>
              <h2>Voz</h2>
              <VoicePicker voice={cfg.voice} output={cfg.outputDevice} onChange={(v) => set("voice", v)} />
              <div className="field">
                <label>Velocidad: {cfg.voiceRate.toFixed(1)}×</label>
                <input type="range" min={0.6} max={1.6} step={0.1} value={cfg.voiceRate} onChange={(e) => set("voiceRate", Number(e.target.value))} />
              </div>
              <div className="field">
                <label>Volumen: {Math.round(cfg.voiceVolume * 100)}%</label>
                <input type="range" min={0.1} max={1} step={0.05} value={cfg.voiceVolume} onChange={(e) => set("voiceVolume", Number(e.target.value))} />
              </div>
            </>
          )}

          {page === "audio" && (
            <>
              <h2>Audio</h2>
              <DevicePicker input={cfg.inputDevice} output={cfg.outputDevice} onChange={(i, o) => { set("inputDevice", i); set("outputDevice", o); }} />
            </>
          )}

          {page === "texto" && (
            <>
              <h2>Texto</h2>
              <label className="check">
                <input type="checkbox" checked={cfg.readAlong} onChange={(e) => set("readAlong", e.target.checked)} />
                <span>
                  <b>Lectura:</b> muestra en un recuadro sobre el orbe lo que te dice ZENOS mientras habla
                </span>
              </label>
              <label className="check">
                <input type="checkbox" checked={cfg.typeMode || !hasMic} disabled={!hasMic} onChange={(e) => set("typeMode", e.target.checked)} />
                <span>
                  <b>Escritura:</b> el atajo abre un recuadro para escribirle a mano (Enter envía, Esc cierra)
                </span>
              </label>
              {!hasMic && <div className="note info">No se detectó ningún micrófono, así que la escritura está activada automáticamente.</div>}
              <p className="hint">Con la escritura activada, el recuadro tiene un botón de micrófono para hablar cuando quieras.</p>
            </>
          )}

          {page === "apariencia" && (
            <>
              <h2>Apariencia</h2>
              <div className="field">
                <label>Tema de la interfaz</label>
                <UiThemePicker value={cfg.uiTheme} onChange={(t) => set("uiTheme", t)} />
              </div>
              <h3>Esfera</h3>
              <div className="field">
                <label>Color</label>
                <ThemePicker value={cfg.theme} onChange={(t) => set("theme", t)} />
              </div>
              <div className="field">
                <label>Tamaño: {Math.round(cfg.orbScale * 100)}%</label>
                <input type="range" min={0.7} max={1.8} step={0.1} value={cfg.orbScale} onChange={(e) => set("orbScale", Number(e.target.value))} />
              </div>
              <div className="field">
                <label>Posición en la pantalla</label>
                <div className="corners">
                  {CORNERS.map(([id, name]) => (
                    <button key={id} className={id === cfg.orbCorner ? "corner on" : "corner"} onClick={() => set("orbCorner", id)}>
                      {name}
                    </button>
                  ))}
                </div>
              </div>
              <label className="check">
                <input type="checkbox" checked={cfg.orbAlwaysVisible} onChange={(e) => set("orbAlwaysVisible", e.target.checked)} />
                Mantener el orbe visible siempre
              </label>
              <h3>Modo lectura</h3>
              <label className="check">
                <input type="checkbox" checked={cfg.readAlong} onChange={(e) => set("readAlong", e.target.checked)} />
                <span>Mostrar en un recuadro sobre la esfera lo que dice ZENOS</span>
              </label>
              <div className="field" style={{ opacity: cfg.readAlong ? 1 : 0.5 }}>
                <label>Tamaño del texto: {Math.round(cfg.readSize * 100)}%</label>
                <input type="range" min={0.8} max={2} step={0.1} disabled={!cfg.readAlong} value={cfg.readSize} onChange={(e) => set("readSize", Number(e.target.value))} />
                <div className="read-sample">
                  <div className="you">Tú: abre la calculadora</div>
                  <div className="zenos">Listo, abrí la calculadora.</div>
                </div>
              </div>
            </>
          )}

          {page === "permisos" && (
            <>
              <h2>Permisos</h2>
              <label className="check">
                <input type="checkbox" checked={cfg.actionsEnabled} onChange={(e) => set("actionsEnabled", e.target.checked)} />
                Permitir que el asistente abra programas, maneje archivos y ejecute comandos
              </label>
              {cfg.actionsEnabled && <PermissionPicker value={cfg.permissionLevel} onChange={(v) => set("permissionLevel", v)} />}
              <p className="hint">Lo que se borra va a la Papelera. Las carpetas del sistema, Archivos de programa y tus carpetas personales completas están siempre protegidas.</p>
            </>
          )}

          {page === "atajo" && (
            <>
              <h2>Atajo y activación</h2>
              <div className="field">
                <label>Combinación (haz clic y pulsa las teclas)</label>
                <HotkeyInput value={cfg.hotkey} onChange={(v) => set("hotkey", v)} />
              </div>
              <div className="field">
                <label>Modo</label>
                <div className="corners">
                  <button className={cfg.hotkeyMode === "toggle" ? "corner on" : "corner"} onClick={() => set("hotkeyMode", "toggle")}>
                    Pulsar para hablar
                  </button>
                  <button className={cfg.hotkeyMode === "hold" ? "corner on" : "corner"} onClick={() => set("hotkeyMode", "hold")}>
                    Mantener para hablar
                  </button>
                </div>
              </div>
              <label className="check">
                <input type="checkbox" checked={cfg.wakeWord} onChange={(e) => set("wakeWord", e.target.checked)} />
                Activar diciendo «Oye Zenos» (experimental)
              </label>
              <label className="check">
                <input type="checkbox" checked={autostart} onChange={(e) => { setAutostart(e.target.checked); setDirty(true); }} />
                Iniciar ZENOS con Windows
              </label>
            </>
          )}

          {page === "ayuda" && (
            <>
              <h2>Ayuda</h2>
              <p className="hint">Registro de errores (envíalo si algo falla):</p>
              <code className="path">{logFile}</code>
              <p>
                <button className="ghost" onClick={rerun}>
                  Repetir asistente de configuración
                </button>
              </p>
            </>
          )}
        </div>

        <footer>
          <Message note={saved} />
          <button disabled={!dirty} onClick={() => void save()}>
            Guardar cambios
          </button>
        </footer>
      </main>
    </div>
  );
}

// ------------------------------------------------------------------ raíz

export function Settings() {
  const [cfg, setCfg] = useState<AppConfig | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [wizard, setWizard] = useState(false);

  useEffect(() => {
    document.documentElement.classList.add("settings");
    void Promise.all([api.getConfig(), api.getProviders()]).then(([c, p]) => {
      setCfg(c);
      setProviders(p);
      setWizard(!c.setupDone);
    });
  }, []);

  if (!cfg) return null;
  if (wizard) {
    return <Wizard initial={cfg} providers={providers} onFinish={() => void getCurrentWindow().close()} />;
  }
  return <Panel initial={cfg} providers={providers} rerun={() => setWizard(true)} />;
}
