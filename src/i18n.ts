/** Idiomas de la interfaz y del asistente. Lo que no está en el diccionario se muestra en español. */
export type Lang = "es" | "en" | "fr" | "pt";

export const LANGS: { id: Lang; name: string }[] = [
  { id: "es", name: "Español" },
  { id: "en", name: "English" },
  { id: "fr", name: "Français" },
  { id: "pt", name: "Português" },
];

const D: Record<Lang, Record<string, string>> = {
  es: {
    inicio: "Inicio", ia: "Inteligencia", voz: "Voz", audio: "Audio", texto: "Texto",
    apariencia: "Apariencia", permisos: "Permisos", atajo: "Atajo", ayuda: "Ayuda",
    save: "Guardar cambios", saved: "Cambios guardados.", next: "Siguiente", back: "Atrás", start: "Empezar",
    finish: "Empezar a usar ZENOS", language: "Idioma", languageHint: "Cambia el idioma de las respuestas, de la voz, del reconocimiento y de los menús.",
    listen: "Escuchar", voiceLabel: "Voz del asistente", conversation: "Conversación continua",
    conversationHint: "Al activarlo puedes hacer todas las preguntas que quieras seguidas. Pulsa el atajo mientras ZENOS habla para interrumpirlo, o mientras escucha para terminar.",
    wizBienvenida: "Bienvenida", wizAudio: "Audio", wizIA: "IA", wizReconocimiento: "Reconocimiento", wizVoz: "Voz", wizEstilo: "Estilo", wizPermisos: "Permisos", wizListo: "Listo",
  },
  en: {
    inicio: "Home", ia: "Intelligence", voz: "Voice", audio: "Audio", texto: "Text",
    apariencia: "Appearance", permisos: "Permissions", atajo: "Shortcut", ayuda: "Help",
    save: "Save changes", saved: "Changes saved.", next: "Next", back: "Back", start: "Start",
    finish: "Start using ZENOS", language: "Language", languageHint: "Changes the language of replies, voice, speech recognition and menus.",
    listen: "Listen", voiceLabel: "Assistant voice", conversation: "Continuous conversation",
    conversationHint: "When on, you can ask as many questions in a row as you like. Press the shortcut while ZENOS talks to interrupt, or while it listens to end.",
    wizBienvenida: "Welcome", wizAudio: "Audio", wizIA: "AI", wizReconocimiento: "Recognition", wizVoz: "Voice", wizEstilo: "Style", wizPermisos: "Permissions", wizListo: "Done",
  },
  fr: {
    inicio: "Accueil", ia: "Intelligence", voz: "Voix", audio: "Audio", texto: "Texte",
    apariencia: "Apparence", permisos: "Autorisations", atajo: "Raccourci", ayuda: "Aide",
    save: "Enregistrer", saved: "Modifications enregistrées.", next: "Suivant", back: "Retour", start: "Commencer",
    finish: "Commencer à utiliser ZENOS", language: "Langue", languageHint: "Change la langue des réponses, de la voix, de la reconnaissance et des menus.",
    listen: "Écouter", voiceLabel: "Voix de l'assistant", conversation: "Conversation continue",
    conversationHint: "Activée, vous pouvez poser autant de questions de suite que vous voulez. Appuyez sur le raccourci pendant que ZENOS parle pour l'interrompre, ou pendant qu'il écoute pour terminer.",
    wizBienvenida: "Bienvenue", wizAudio: "Audio", wizIA: "IA", wizReconocimiento: "Reconnaissance", wizVoz: "Voix", wizEstilo: "Style", wizPermisos: "Autorisations", wizListo: "Terminé",
  },
  pt: {
    inicio: "Início", ia: "Inteligência", voz: "Voz", audio: "Áudio", texto: "Texto",
    apariencia: "Aparência", permisos: "Permissões", atajo: "Atalho", ayuda: "Ajuda",
    save: "Salvar alterações", saved: "Alterações salvas.", next: "Próximo", back: "Voltar", start: "Começar",
    finish: "Começar a usar o ZENOS", language: "Idioma", languageHint: "Muda o idioma das respostas, da voz, do reconhecimento e dos menus.",
    listen: "Ouvir", voiceLabel: "Voz do assistente", conversation: "Conversa contínua",
    conversationHint: "Ativada, você pode fazer quantas perguntas seguidas quiser. Pressione o atalho enquanto o ZENOS fala para interromper, ou enquanto ele ouve para encerrar.",
    wizBienvenida: "Boas-vindas", wizAudio: "Áudio", wizIA: "IA", wizReconocimiento: "Reconhecimento", wizVoz: "Voz", wizEstilo: "Estilo", wizPermisos: "Permissões", wizListo: "Pronto",
  },
};

export function tr(lang: string, key: string): string {
  const l = (["es", "en", "fr", "pt"].includes(lang) ? lang : "es") as Lang;
  return D[l][key] ?? D.es[key] ?? key;
}
