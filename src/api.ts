import { invoke } from "@tauri-apps/api/core";

export interface AppConfig {
  provider: string;
  model: string;
  sttProvider: string;
  sttModel: string;
  language: string;
  systemPrompt: string;
  maxTokens: number;
  historyTurns: number;
  hotkey: string;
  voice: string;
  voiceRate: number;
  voiceVolume: number;
  orbAlwaysVisible: boolean;
  inputDevice: string;
  outputDevice: string;
  setupDone: boolean;
  hotkeyMode: "toggle" | "hold";
  wakeWord: boolean;
  theme: string;
  orbScale: number;
  orbCorner: string;
  actionsEnabled: boolean;
  permissionLevel: "estricto" | "normal" | "libre";
  silenceMs: number;
  maxRecordMs: number;
  startTimeoutMs: number;
}

export interface ProviderInfo {
  id: string;
  name: string;
  defaultModel: string;
  suggestedModels: string[];
  keyUrl: string;
}

export interface Voice {
  id: string;
  label: string;
  language: string;
  kind: "neural" | "windows";
}

export interface ConfirmInfo {
  text: string;
  danger: boolean;
}

export interface ListenResult {
  kind: "text" | "nospeech" | "cancelled";
  text: string;
}

export interface AskResult {
  answer: string;
  chunks: string[];
}

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  saveConfig: (config: AppConfig) => invoke<AppConfig>("save_config", { config }),
  getProviders: () => invoke<ProviderInfo[]>("get_providers"),
  keyStatus: () => invoke<{ provider: string; saved: boolean }[]>("key_status"),
  setApiKey: (provider: string, key: string) => invoke<void>("set_api_key", { provider, key }),
  deleteApiKey: (provider: string) => invoke<void>("delete_api_key", { provider }),
  listen: () => invoke<ListenResult>("listen"),
  stopListen: () => invoke<void>("stop_listen"),
  getConfirm: () => invoke<ConfirmInfo | null>("get_confirm"),
  answerConfirm: (allow: boolean) => invoke<void>("answer_confirm", { allow }),
  cancelListen: () => invoke<void>("cancel_listen"),
  ask: (text: string) => invoke<AskResult>("ask", { text }),
  testProvider: () => invoke<string>("test_provider"),
  resetConversation: () => invoke<void>("reset_conversation"),
  speak: (chunks: string[]) => invoke<void>("speak", { chunks }),
  stopSpeaking: () => invoke<void>("stop_speaking"),
  listDevices: () => invoke<{ inputs: string[]; outputs: string[] }>("list_devices"),
  testMic: (device: string) => invoke<number>("test_mic", { device }),
  testSpeaker: (device: string, voice: string) => invoke<void>("test_speaker", { device, voice }),
  setupProvider: (provider: string, key: string) => invoke<string>("setup_provider", { provider, key }),
  setupStt: (provider: string, key: string) => invoke<void>("setup_stt", { provider, key }),
  listVoices: () => invoke<Voice[]>("list_voices"),
  setActive: (active: boolean) => invoke<void>("set_active", { active }),
  hideOrb: () => invoke<void>("hide_orb"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  showOrb: () => invoke<void>("show_orb_cmd"),
  logPath: () => invoke<string>("log_path"),
};

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "Ha ocurrido un error.";
}
