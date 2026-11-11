import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, errorText, type AppConfig, type ProviderInfo, type Voice } from "../api";
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

// ------------------------------------------------------------------ audio (común)

interface AudioState {
  input: string;
  output: string;
  voice: string;
}

function AudioPicker({ value, onChange }: { value: AudioState; onChange: (v: AudioState) => void }) {
  const [inputs, setInputs] = useState<string[]>([]);
  const [outputs, setOutputs] = useState<string[]>([]);
  const [voices, setVoices] = useState<Voice[]>([]);
  const [micNote, setMicNote] = useState<Note>(null);
  const [spkNote, setSpkNote] = useState<Note>(null);
  const [busy, setBusy] = useState<"" | "mic" | "spk">("");

  const refresh = useCallback(() => {
    void api.listDevices().then((d) => {
      setInputs(d.inputs);
      setOutputs(d.outputs);
    });
    void api.listVoices().then(setVoices).catch(() => setVoices([]));
  }, []);
  useEffect(refresh, [refresh]);

  const testMic = async () => {
    setBusy("mic");
    setMicNote({ kind: "info", text: "Habla ahora durante 3 segundos…" });
    try {
      const peak = await api.testMic(value.input);
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
      await api.testSpeaker(value.output, value.voice);
      setSpkNote({ kind: "ok", text: "¿Me escuchaste? Si no, elige otro altavoz." });
    } catch (e) {
      setSpkNote({ kind: "error", text: errorText(e) });
    } finally {
      setBusy("");
    }
  };

  const spanish = voices.filter((v) => v.language.toLowerCase().startsWith("es"));
  const voiceList = spanish.length ? spanish : voices;

  return (
    <>
      <div className="field">
        <label>Micrófono</label>
        <div className="row">
          <select value={value.input} onChange={(e) => onChange({ ...value, input: e.target.value })}>
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
          <select value={value.output} onChange={(e) => onChange({ ...value, output: e.target.value })}>
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
      <div className="field">
        <label>Voz</label>
        <select value={value.voice} onChange={(e) => onChange({ ...value, voice: e.target.value })}>
          <option value="">Automática (español)</option>
          {voiceList.map((v) => (
            <option key={v.name} value={v.name}>
              {v.name} ({v.language})
            </option>
          ))}
        </select>
      </div>
    </>
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

// ------------------------------------------------------------------ asistente inicial

function Wizard({ initial, providers, onFinish }: { initial: AppConfig; providers: ProviderInfo[]; onFinish: () => void }) {
  const [step, setStep] = useState(0);
  const [provider, setProvider] = useState("");
  const [chatDone, setChatDone] = useState(false);
  const [sttDone, setSttDone] = useState(false);
  const [audio, setAudio] = useState<AudioState>({ input: initial.inputDevice, output: initial.outputDevice, voice: initial.voice });
  const [autostart, setAutostart] = useState(true);
  const [err, setErr] = useState("");

  const needsStt = provider !== "" && provider !== "groq" && provider !== "openai";
  const steps = ["Bienvenida", "Audio", "IA", ...(needsStt ? ["Voz"] : []), "Listo"];
  const name = steps[step];

  const finish = async () => {
    try {
      const cfg = await api.getConfig();
      await api.saveConfig({ ...cfg, inputDevice: audio.input, outputDevice: audio.output, voice: audio.voice, setupDone: true });
      await api.setAutostart(autostart);
      await api.showOrb();
      onFinish();
    } catch (e) {
      setErr(errorText(e));
    }
  };

  const canNext = name === "IA" ? chatDone : name === "Voz" ? sttDone : true;

  return (
    <div className="wizard">
      <div className="dots">
        {steps.map((s, i) => (
          <i key={s} className={i === step ? "on" : i < step ? "past" : ""} />
        ))}
      </div>

      <div className="page" key={name}>
        {name === "Bienvenida" && (
          <>
            <div className="logo" />
            <h1>ZENOS</h1>
            <p className="lead">
              Tu asistente de voz. Pulsa <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Espacio</kbd>, habla, y escucha la
              respuesta de tu IA.
            </p>
            <p className="hint">Vamos a dejarlo listo en un minuto: eliges tu micrófono, tu altavoz y tu IA.</p>
          </>
        )}

        {name === "Audio" && (
          <>
            <h2>Micrófono y altavoz</h2>
            <p className="hint">Elige cuáles usar y pruébalos.</p>
            <AudioPicker value={audio} onChange={setAudio} />
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

        {name === "Voz" && (
          <>
            <h2>Reconocimiento de voz</h2>
            <p className="hint">Tu IA no transcribe audio, así que usamos Whisper en la nube.</p>
            <SttKey current={initial.sttProvider} onDone={() => setSttDone(true)} />
          </>
        )}

        {name === "Listo" && (
          <>
            <div className="logo ok" />
            <h2>Todo listo</h2>
            <p className="lead">
              Pulsa <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Espacio</kbd> en cualquier momento. Para cancelar, pulsa <kbd>Esc</kbd>.
            </p>
            <label className="check">
              <input type="checkbox" checked={autostart} onChange={(e) => setAutostart(e.target.checked)} />
              Iniciar ZENOS con Windows
            </label>
            {err && <div className="note error">{err}</div>}
          </>
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

// ------------------------------------------------------------------ panel

function Panel({ initial, providers, rerun }: { initial: AppConfig; providers: ProviderInfo[]; rerun: () => void }) {
  const [cfg, setCfg] = useState(initial);
  const [saved, setSaved] = useState<Note>(null);
  const [keys, setKeys] = useState<Record<string, boolean>>({});
  const [autostart, setAutostart] = useState(false);
  const [logFile, setLogFile] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [aiNote, setAiNote] = useState<Note>(null);

  const refreshKeys = useCallback(() => {
    void api.keyStatus().then((l) => setKeys(Object.fromEntries(l.map((k) => [k.provider, k.saved]))));
  }, []);
  useEffect(() => {
    refreshKeys();
    void api.getAutostart().then(setAutostart);
    void api.logPath().then(setLogFile);
  }, [refreshKeys]);

  const info = providers.find((p) => p.id === cfg.provider);
  const set = <K extends keyof AppConfig>(k: K, v: AppConfig[K]) => setCfg((c) => ({ ...c, [k]: v }));

  const save = async () => {
    try {
      const out = await api.saveConfig(cfg);
      await api.setAutostart(autostart);
      setCfg(out);
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

  return (
    <div className="panel">
      <header>
        <div className="logo small" />
        <h1>ZENOS</h1>
      </header>

      <section>
        <h3>Inteligencia artificial</h3>
        <div className="field">
          <label>Proveedor</label>
          <select value={cfg.provider} onChange={(e) => setCfg({ ...cfg, provider: e.target.value, model: "" })}>
            {providers.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} {keys[p.id] ? "· clave guardada" : ""}
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <label>Modelo</label>
          <input
            list="models"
            value={cfg.model}
            placeholder={`Automático (${info?.defaultModel ?? ""})`}
            onChange={(e) => set("model", e.target.value)}
          />
          <datalist id="models">
            {info?.suggestedModels.map((m) => (
              <option key={m} value={m} />
            ))}
          </datalist>
          <p className="hint">Déjalo vacío para que ZENOS use el modelo recomendado.</p>
        </div>
        <button className="ghost" onClick={() => setShowKey(!showKey)}>
          {keys[cfg.provider] ? "Cambiar clave" : "Añadir clave"}
        </button>{" "}
        <button className="ghost" onClick={() => void testAi()}>
          Probar IA
        </button>
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
      </section>

      {cfg.provider !== "groq" && cfg.provider !== "openai" && (
        <section>
          <h3>Reconocimiento de voz</h3>
          <p className="hint">
            Servicio actual: {STT_NAMES[cfg.sttProvider]} · clave {keys[cfg.sttProvider] ? "guardada" : "falta"}
          </p>
          <SttKey current={cfg.sttProvider} onDone={refreshKeys} />
        </section>
      )}

      <section>
        <h3>Audio</h3>
        <AudioPicker
          value={{ input: cfg.inputDevice, output: cfg.outputDevice, voice: cfg.voice }}
          onChange={(v) => setCfg({ ...cfg, inputDevice: v.input, outputDevice: v.output, voice: v.voice })}
        />
        <div className="field">
          <label>Velocidad de la voz: {cfg.voiceRate.toFixed(1)}×</label>
          <input type="range" min={0.6} max={1.6} step={0.1} value={cfg.voiceRate} onChange={(e) => set("voiceRate", Number(e.target.value))} />
        </div>
        <div className="field">
          <label>Volumen: {Math.round(cfg.voiceVolume * 100)}%</label>
          <input type="range" min={0.1} max={1} step={0.05} value={cfg.voiceVolume} onChange={(e) => set("voiceVolume", Number(e.target.value))} />
        </div>
      </section>

      <section>
        <h3>General</h3>
        <div className="field">
          <label>Atajo de teclado (haz clic y pulsa la combinación)</label>
          <HotkeyInput value={cfg.hotkey} onChange={(v) => set("hotkey", v)} />
        </div>
        <label className="check">
          <input type="checkbox" checked={cfg.orbAlwaysVisible} onChange={(e) => set("orbAlwaysVisible", e.target.checked)} />
          Mantener la esfera visible siempre
        </label>
        <label className="check">
          <input type="checkbox" checked={autostart} onChange={(e) => setAutostart(e.target.checked)} />
          Iniciar ZENOS con Windows
        </label>
      </section>

      <section>
        <h3>Ayuda</h3>
        <p className="hint">Registro de errores (envíalo si algo falla):</p>
        <code className="path">{logFile}</code>
        <p>
          <button className="ghost" onClick={rerun}>
            Repetir asistente de configuración
          </button>
        </p>
      </section>

      <footer>
        <Message note={saved} />
        <button onClick={() => void save()}>Guardar cambios</button>
      </footer>
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
