# ZENOS

Asistente de voz para Windows 11. Una pequeña esfera oscura y abstracta flota en la esquina inferior derecha: pulsas **Ctrl + Alt + Espacio**, hablas en español y te responde en voz alta con la IA que elijas.

- Sin ventanas ni paneles: solo la esfera, transparente y sin bordes.
- Proveedores: **OpenAI, Anthropic Claude, Google Gemini, Groq, DeepSeek y OpenRouter**.
- Claves API guardadas en el **Administrador de credenciales de Windows**, nunca en archivos ni en el código.
- Se inicia con Windows y vive en la bandeja del sistema.

## Instalación (usuario final)

1. Descarga `ZENOS_x.y.z_x64-setup.exe` desde la sección **Releases** del repositorio.
2. Ejecútalo. No requiere permisos de administrador, ni Python, Node.js o Rust.
   - Windows SmartScreen puede avisar porque el instalador no está firmado: pulsa **Más información → Ejecutar de todas formas**.
3. Al abrirse por primera vez aparece un **asistente de configuración**: eliges micrófono y altavoz (con prueba), tu IA y pegas la clave API. ZENOS verifica la clave y elige el modelo automáticamente.
4. Pulsa **Ctrl + Alt + Espacio** y habla.

## Claves necesarias

ZENOS necesita dos servicios: uno para **entender tu voz** y otro para **responder**.

| Para qué | Opciones | Dónde conseguir la clave |
|---|---|---|
| Reconocer tu voz (Whisper) | **Groq** (recomendado, plan gratuito) u OpenAI | https://console.groq.com/keys |
| Responder | Claude, ChatGPT, Gemini, Groq, DeepSeek u OpenRouter | En la configuración hay un enlace junto a cada proveedor |

Si usas Groq para ambas cosas, con **una sola clave** es suficiente.

## Uso

| Acción | Cómo |
|---|---|
| Hablar | `Ctrl + Alt + Espacio` (configurable) |
| Cancelar la grabación o interrumpir la respuesta | Pulsa el atajo otra vez, o `Esc` |
| Configuración | Clic en el icono de ZENOS en la bandeja del sistema |
| Mostrar u ocultar la esfera en reposo | Menú del icono → *Mostrar / ocultar esfera* |
| Salir | Menú del icono → *Salir de ZENOS* |

La esfera cambia de color y movimiento según lo que hace:

- **En espera**: brillo tenue y movimiento lento.
- **Escuchando** (cian): reacciona al volumen de tu voz.
- **Procesando** (violeta): anillos girando más rápido.
- **Hablando** (magenta): late con la voz.
- **Error** (rojo/naranja): te dice qué falta, por ejemplo una clave.

La conversación recuerda los últimos turnos durante 5 minutos, así que puedes hacer preguntas de seguimiento.

## Configuración

Todo se cambia desde la ventana de **Configuración**:

- **Proveedor y modelo**: el campo *Modelo* sugiere modelos; si lo dejas vacío se usa el recomendado. Usa **Probar IA** para comprobar la clave.
- **Micrófono y altavoz**: elige el dispositivo y pulsa **Probar** (barra de nivel en vivo).
- **Voz**: usa las voces instaladas en Windows. Para tener voces en español: *Configuración de Windows → Hora e idioma → Voz → Agregar voces → Español*.
- **Velocidad y volumen** de la voz.
- **Atajo de teclado**, **inicio con Windows** y **esfera siempre visible**.
- **Repetir asistente de configuración** desde *Ayuda*.

La configuración se guarda en `%APPDATA%\com.zenos.app\config.json` y no contiene claves.

## Solución de problemas

| Problema | Solución |
|---|---|
| No pasa nada al pulsar el atajo | Comprueba que ZENOS esté en la bandeja del sistema. Si otra aplicación usa el mismo atajo, cámbialo en Configuración. |
| «Falta la clave de…» | Abre la Configuración desde la bandeja y guarda la clave indicada. |
| «La clave de … no es válida» | Copia la clave de nuevo desde la web del proveedor y comprueba que tenga saldo. |
| «El modelo configurado no existe» | Deja el campo *Modelo* vacío para usar el recomendado. |
| No me escucha | Revisa el micrófono predeterminado en *Configuración de Windows → Sistema → Sonido → Entrada* y que las apps de escritorio tengan permiso para usarlo (*Privacidad y seguridad → Micrófono*). |
| Corta antes de que termine de hablar | Edita `silenceMs` en `config.json` (por defecto 1200). |
| La voz no habla en español | Instala una voz española de Windows y selecciónala en Configuración. |
| Otros errores | Revisa el registro en `%APPDATA%\com.zenos.app\logs\zenos.log`. |

Para desinstalar: *Configuración de Windows → Aplicaciones → ZENOS → Desinstalar*. Las claves se borran desde la Configuración de ZENOS (*Cambiar clave*) o desde el *Administrador de credenciales* (entradas «ZENOS»).

---

## Desarrollo

Requisitos: Windows 10/11, [Node.js 22](https://nodejs.org), [Rust](https://rustup.rs) y los [requisitos de Tauri](https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev      # ejecutar en desarrollo
npm run tauri build    # generar el instalador en src-tauri/target/release/bundle/nsis/
cargo test --manifest-path src-tauri/Cargo.toml
```

### Arquitectura

```
src/                     Interfaz (React + TypeScript + Vite)
  orb/                   Esfera: Orb.tsx (estructura), orb.css (animaciones), OrbApp.tsx (ciclo)
  settings/              Ventana de configuración
  (la voz se reproduce en Rust con cpal: src-tauri/src/audio/playback.rs)
src-tauri/src/           Núcleo nativo (Rust)
  lib.rs                 Ventanas, bandeja, atajo global y comandos
  audio/                 Micrófono (cpal), detección de voz y WAV
  stt.rs                 Transcripción Whisper (Groq / OpenAI)
  ai/                    Adaptadores de proveedores de IA
  tts.rs                 Voz nativa de Windows (Windows.Media.SpeechSynthesis)
  secrets.rs             Claves en el Administrador de credenciales
  config.rs              Configuración local
```

Flujo: atajo → Rust muestra la esfera → graba hasta detectar silencio → Whisper → proveedor de IA → la respuesta se divide en frases → Windows sintetiza cada frase mientras suena la anterior → la esfera vuelve a reposo y se oculta.

Las claves API nunca llegan al frontend: Rust las lee del Administrador de credenciales y hace las peticiones HTTP.

### Añadir un proveedor

- Si es **compatible con la API de OpenAI**, basta con añadir una entrada en `PROVIDERS` (`src-tauri/src/ai/mod.rs`) con su URL base y modelo por defecto.
- Si tiene otra API, crea un módulo nuevo en `src-tauri/src/ai/`, igual que `anthropic.rs`, y añade su variante a `ApiKind`.

### Publicar una versión

1. Actualiza `version` en `package.json`, `src-tauri/Cargo.toml` y `src-tauri/tauri.conf.json`.
2. Crea y sube la etiqueta:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. GitHub Actions (`.github/workflows/release.yml`) ejecuta las pruebas, compila el instalador y lo publica en **Releases**.

Cada `push` a `main` ejecuta además `ci.yml`, que prueba y deja el instalador como artefacto descargable de la ejecución.
