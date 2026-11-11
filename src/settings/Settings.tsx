import { useEffect, useMemo, useRef, useState } from "react";
import { api, errorText, type AppConfig, type ProviderInfo, type Voice } from "../api";
import { Player } from "../player";
import "./settings.css";

type Notice = { kind: "ok" | "error" | "info"; text: string } | null;

const STT_OPTIONS = [
  { id: "groq", name: "Groq · Whisper (recomendado, tiene plan gratuito)" },
  { id: "openai", name: "OpenAI · Transcribe" },
];

function KeyField({
  provider,
  saved,
  onChanged,
  notify,
}: {
  provider: ProviderInfo;
  saved: boolean;
  onChanged: () => void;
  notify: (n: Notice) => void;
}) {
  const [value, setValue] = useState("");
  const save = async () => {
    try {
      await api.setApiKey(provider.id, value);
      setValue("");
      notify({ kind: "ok", text: `Clave de ${provider.name} guardada en el Administrador de credenciales.` });
      onChanged();
    } catch (e) {
      notify({ kind: "error", text: errorText(e) });
    }
  };
  const remove = async () => {
    await api.deleteApiKey(provider.id);
    notify({ kind: "info", text: `Clave de ${provider.name} eliminada.` });
    onChanged();
  };
  return (
    <div className="field">
      <label>
        Clave API de {provider.name}{" "}
        <span className={saved ? "chip ok" : "chip missing"}>{saved ? "guardada" : "falta"}</span>
      </label>
      <div className="row">
        <input
          type="password"
          autoComplete="off"
          spellCheck={false}
          placeholder={saved ? "•••••••• (escribe para reemplazarla)" : "Pega aquí tu clave"}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && value.trim() && void save()}
        />
        <button disabled={!value.trim()} onClick={() => void save()}>
          Guardar
        </button>
        {saved && (
          <button className="ghost" title="Eliminar clave" onClick={() => void remove()}>
            ✕
          </button>
        )}
      </div>
      <a className="hint" href={provider.keyUrl} target="_blank" rel="noreferrer">
        Obtener una clave →
      </a>
    </div>
  );
}

export function Settings() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [keys, setKeys] = useState<Record<string, boolean>>({});
  const [voices, setVoices] = useState<Voice[]>([]);
  const [autostart, setAutostart] = useState(false);
  const [notice, setNotice] = useState<Notice>(null);
  const [busy, setBusy] = useState(false);
  const [logPath, setLogPath] = useState("");
  const player = useRef(new Player());

  const refreshKeys = () =>
    api.keyStatus().then((s) => setKeys(Object.fromEntries(s.map((k) => [k.provider, k.saved]))));

  useEffect(() => {
    document.documentElement.classList.add("settings");
    void api.getConfig().then(setConfig);
    void api.getProviders().then(setProviders);
    void refreshKeys();
    void api.listVoices().then(setVoices).catch(() => setVoices([]));
    void api.getAutostart().then(setAutostart);
    void api.logPath().then(setLogPath);
  }, []);

  const provider = useMemo(() => providers.find((p) => p.id === config?.provider), [providers, config?.provider]);
  const sttProvider = useMemo(() => providers.find((p) => p.id === config?.sttProvider), [providers, config?.sttProvider]);
  const spanishVoices = voices.filter((v) => v.language.toLowerCase().startsWith("es"));
  const otherVoices = voices.filter((v) => !v.language.toLowerCase().startsWith("es"));

  if (!config) return <div className="loading">Cargando…</div>;

  const set = <K extends keyof AppConfig>(k: K, v: AppConfig[K]) => setConfig({ ...config, [k]: v });

  const save = async () => {
    try {
      const saved = await api.saveConfig(config);
      setConfig(saved);
      setNotice({ kind: "ok", text: "Configuración guardada." });
      return true;
    } catch (e) {
      setNotice({ kind: "error", text: errorText(e) });
      return false;
    }
  };

  const testAi = async () => {
    setBusy(true);
    setNotice({ kind: "info", text: `Probando ${provider?.name}…` });
    try {
      if (!(await save())) return;
      const answer = await api.testProvider();
      setNotice({ kind: "ok", text: `${provider?.name} responde: «${answer}»` });
    } catch (e) {
      setNotice({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  const testVoice = async () => {
    setBusy(true);
    try {
      if (!(await save())) return;
      await player.current.speak(["Hola, soy AI Star. Así sonará mi voz."], config.voiceVolume);
    } catch (e) {
      setNotice({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  const toggleAutostart = async (on: boolean) => {
    try {
      await api.setAutostart(on);
      setAutostart(on);
    } catch (e) {
      setNotice({ kind: "error", text: errorText(e) });
    }
  };

  return (
    <main>
      <header>
        <div className="mini-orb" />
        <div>
          <h1>AI Star</h1>
          <p>
            Pulsa <kbd>{config.hotkey.replace("Space", "Espacio")}</kbd> y habla. Vuelve a pulsarlo o usa <kbd>Esc</kbd>{" "}
            para cancelar.
          </p>
        </div>
      </header>

      <section>
        <h2>Inteligencia artificial</h2>
        <div className="grid2">
          <div className="field">
            <label>Proveedor</label>
            <select value={config.provider} onChange={(e) => setConfig({ ...config, provider: e.target.value, model: "" })}>
              {providers.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </div>
          <div className="field">
            <label>Modelo</label>
            <input
              list="models"
              value={config.model}
              placeholder={provider?.defaultModel}
              spellCheck={false}
              onChange={(e) => set("model", e.target.value)}
            />
            <datalist id="models">
              {provider?.suggestedModels.map((m) => <option key={m} value={m} />)}
            </datalist>
          </div>
        </div>
        {provider && <KeyField provider={provider} saved={!!keys[provider.id]} onChanged={refreshKeys} notify={setNotice} />}
        <button className="secondary" disabled={busy || !keys[config.provider]} onClick={() => void testAi()}>
          Probar conexión
        </button>
      </section>

      <section>
        <h2>Reconocimiento de voz</h2>
        <div className="field">
          <label>Servicio de transcripción (Whisper)</label>
          <select value={config.sttProvider} onChange={(e) => setConfig({ ...config, sttProvider: e.target.value, sttModel: "" })}>
            {STT_OPTIONS.map((o) => (
              <option key={o.id} value={o.id}>
                {o.name}
              </option>
            ))}
          </select>
        </div>
        {sttProvider && sttProvider.id !== config.provider && (
          <KeyField provider={sttProvider} saved={!!keys[sttProvider.id]} onChanged={refreshKeys} notify={setNotice} />
        )}
        {sttProvider && sttProvider.id === config.provider && (
          <p className="hint">Usa la misma clave de {sttProvider.name} de arriba.</p>
        )}
      </section>

      <section>
        <h2>Voz</h2>
        <div className="field">
          <label>Voz de Windows</label>
          <select value={config.voice} onChange={(e) => set("voice", e.target.value)}>
            <option value="">Automática (primera voz en español)</option>
            {spanishVoices.map((v) => (
              <option key={v.name} value={v.name}>
                {v.name} · {v.language}
              </option>
            ))}
            {otherVoices.length > 0 && (
              <optgroup label="Otros idiomas">
                {otherVoices.map((v) => (
                  <option key={v.name} value={v.name}>
                    {v.name} · {v.language}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
          {voices.length > 0 && spanishVoices.length === 0 && (
            <p className="warn">
              No hay voces en español instaladas. Añádelas en Configuración de Windows › Hora e idioma › Voz.
            </p>
          )}
        </div>
        <div className="grid2">
          <div className="field">
            <label>Velocidad · {config.voiceRate.toFixed(1)}×</label>
            <input type="range" min={0.5} max={2} step={0.1} value={config.voiceRate} onChange={(e) => set("voiceRate", +e.target.value)} />
          </div>
          <div className="field">
            <label>Volumen · {Math.round(config.voiceVolume * 100)}%</label>
            <input type="range" min={0} max={1} step={0.05} value={config.voiceVolume} onChange={(e) => set("voiceVolume", +e.target.value)} />
          </div>
        </div>
        <button className="secondary" disabled={busy} onClick={() => void testVoice()}>
          Probar voz
        </button>
      </section>

      <section>
        <h2>General</h2>
        <div className="field">
          <label>Atajo de teclado</label>
          <input value={config.hotkey} spellCheck={false} onChange={(e) => set("hotkey", e.target.value)} />
          <p className="hint">Formato: Ctrl+Alt+Space, Ctrl+Shift+J, Alt+F9…</p>
        </div>
        <label className="check">
          <input type="checkbox" checked={autostart} onChange={(e) => void toggleAutostart(e.target.checked)} />
          Iniciar AI Star con Windows
        </label>
        <label className="check">
          <input type="checkbox" checked={config.orbAlwaysVisible} onChange={(e) => set("orbAlwaysVisible", e.target.checked)} />
          Mantener la esfera visible en reposo
        </label>
        <details>
          <summary>Avanzado</summary>
          <div className="grid2">
            <div className="field">
              <label>Silencio para terminar (ms)</label>
              <input type="number" min={400} max={5000} step={100} value={config.silenceMs} onChange={(e) => set("silenceMs", +e.target.value)} />
            </div>
            <div className="field">
              <label>Grabación máxima (ms)</label>
              <input type="number" min={3000} max={120000} step={1000} value={config.maxRecordMs} onChange={(e) => set("maxRecordMs", +e.target.value)} />
            </div>
          </div>
          <div className="field">
            <label>Instrucciones del asistente</label>
            <textarea rows={4} value={config.systemPrompt} onChange={(e) => set("systemPrompt", e.target.value)} />
          </div>
          <p className="hint">Registro de errores: {logPath}</p>
        </details>
      </section>

      <footer>
        {notice && <div className={`notice ${notice.kind}`}>{notice.text}</div>}
        <button className="primary" disabled={busy} onClick={() => void save()}>
          Guardar cambios
        </button>
      </footer>
    </main>
  );
}
