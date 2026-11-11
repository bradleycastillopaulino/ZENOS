# ZENOS

Asistente de voz para Windows 11. Un orbe geométrico dibujado a mano flota en la esquina inferior derecha: pulsas **Ctrl + Alt + Espacio**, hablas en español y te responde en voz alta con la IA que elijas.

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

El orbe es un círculo de trazo grueso que gira y se transforma entre formas geométricas:

- **En espera**: gira despacio.
- **Escuchando**: pulsa con el volumen de tu voz.
- **Procesando**: gira rápido y le orbitan chispas.
- **Hablando**: late con la voz.
- **Error**: se queda cuadrado y tiembla.

Cinco temas de color (menta, uva, mandarina, chicle, cielo), tamaño y esquina de la pantalla configurables.

## Control del PC

El asistente puede **abrir programas y páginas, crear carpetas, listar y leer archivos, escribir, mover y copiar, enviar a la Papelera y ejecutar comandos de PowerShell**. Ejemplos: «abre la calculadora», «crea una carpeta Proyectos en el escritorio», «borra la carpeta Pruebas».

Niveles de permiso (Configuración → Permisos):

| Nivel | Pregunta antes de… |
|---|---|
| Estricto | cualquier cambio (abrir, crear, mover, escribir, borrar, comandos) |
| **Normal** (por defecto) | borrar, sobrescribir archivos y ejecutar comandos |
| Libre | nada |

Lo que se borra va a la **Papelera** (recuperable). Las carpetas de Windows, Archivos de programa, la raíz de las unidades y tus carpetas personales completas están **siempre protegidas**. La confirmación es una ventana con botones; cerrarla o no responder en 60 s cancela la acción. Puedes desactivar todo esto y dejar solo la conversación.

### Protecciones adicionales

- `abrir` pide confirmación (en Normal) si lleva argumentos, apunta a un `.exe`, `.bat`, `.ps1` y similares, o nombra un intérprete como `powershell` o `cmd`. El diálogo muestra los argumentos completos.
- Crear o copiar archivos de script (`.bat`, `.ps1`, `.vbs`…) pide confirmación.
- Después de que el asistente lea archivos o carpetas, abrir una página web pide confirmación (evita que el contenido de un archivo saque datos por una dirección web).
- La carpeta de Inicio de Windows está protegida contra escritura.
- El registro (`zenos.log`) no guarda el contenido de los archivos leídos y recorta lo que dices y lo que responde.

## Privacidad

- Tu **audio** se envía a Groq u OpenAI para transcribirlo.
- Tu **texto** (lo que dices, lo que lee de tus archivos y las respuestas) se envía al proveedor de IA que elijas.
- Con voces neuronales, el **texto de la respuesta** se envía al servicio de voz de Microsoft Edge; con voces de Windows, no sale de tu PC.
- Las claves se guardan solo en el Administrador de credenciales de Windows.

## Voces

Voces neuronales gratuitas con varios caracteres (Dalia, Dalia viva, Elvira, Elvira serena, Jorge, Álvaro, Elena, Salomé) y las voces instaladas en Windows. Las neuronales usan el servicio de lectura de Microsoft Edge, que es **no oficial**: si falla, ZENOS usa la voz de Windows automáticamente.

## Avatares y personalidad

Puedes elegir entre cuatro avatares en **Apariencia → Esfera**, y cada uno habla con su propia personalidad (se puede desactivar en la misma página):

| Avatar | Personalidad | Estilo |
|---|---|---|
| **Formas** | Analítica | Conclusión primero, datos, humor seco y preciso |
| **Gelatina** | Divertida y optimista | Chistes ligeros, energía contagiosa, «eso estuvo de cabra» |
| **Órbita** | Creativa | Ideas, analogías y un camino seguro y otro atrevido |
| **Pulso** | Cálida | Calma, cercanía y algún refrán |

Todos comparten el mismo contexto: ZENOS lo creó **Bradley Castillo, «la cabra»**, y ayuda con preguntas, curiosidades y tareas básicas de Windows. Los textos están en `src-tauri/src/persona.rs`. Los datos de contexto no incluyen internet ni la hora: ZENOS no navega ni ve la pantalla.

## Idioma y conversación continua

- **Idioma** (Configuración → Voz): español, inglés, francés o portugués. Cambia el idioma de las respuestas, de la voz (cada idioma tiene sus voces), del reconocimiento y de los menús principales. Los textos largos de ayuda aún están solo en español.
- **Conversación continua** (Configuración → Atajo, activada por defecto): al activar a ZENOS puedes hacerle preguntas seguidas sin volver a activarlo. Pulsa el atajo **mientras habla** para interrumpirlo (te escucha enseguida) o **mientras te escucha** para terminar. `Esc` termina en cualquier momento, y si guardas silencio la conversación se cierra sola. Interrumpir con la voz (sin pulsar nada) no está implementado: con altavoces el micrófono oiría a ZENOS.
- Si las voces neuronales fallan, el botón «Escuchar» ahora indica qué motor sonó y por qué.

## Lectura y escritura

En Configuración → Texto:

- **Lectura**: un recuadro sobre el orbe muestra lo que dices y lo que responde ZENOS mientras habla.
- **Escritura**: el atajo abre un cuadro para escribirle a mano (Enter envía, Esc cierra; hay botón de micrófono para hablar). Si el equipo no tiene micrófono, se activa sola.

## Atajo y activación

Atajo configurable en modo **pulsar** o **mantener pulsado**. Opcionalmente, di **«Oye Zenos»** (experimental: usa el reconocimiento local de Windows).

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
  (la voz se reproduce en Rust con cpal: src-tauri/src/audio/playback.rs;
   acciones del PC: src-tauri/src/tools.rs; voces: neural.rs y tts.rs; palabra de activación: wake.rs)
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

## Licencias de terceros

`src-tauri/src/gec.rs` es una adaptación a Rust del módulo `drm.py` de [edge-tts](https://github.com/rany2/edge-tts) (LGPL-3.0-or-later) y mantiene esa licencia. El servicio de voces neuronales es no oficial y puede cambiar sin aviso; ZENOS vuelve solo a la voz de Windows si falla.
