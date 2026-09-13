# LinMic

[Português (Brasil)](../../README.md) · [English](README.en.md) · **Español** · [Русский](README.ru.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

Usa tu teléfono Android como micrófono para Linux. Aplicaciones nativas, sin cuenta ni nube. **0.3.0 (Android) / 0.2.0 (Linux) · Android 8+ · Linux con PipeWire**, principalmente CachyOS/Arch.

## Instalar y conectar

1. Descarga la [última versión](https://github.com/goszc/LinMic/releases/latest).
2. Extrae el archivo Linux y ejecuta `./scripts/install-linux.sh` como usuario normal, sin sudo. Abre **LinMic** desde el menú de aplicaciones.
3. Instala el APK Android. Conecta ambos dispositivos a la misma red privada o utiliza la conexión compartida por USB.
4. Pulsa **Vincular teléfono** en el escritorio. En Android, selecciona el PC descubierto o introduce su IP, permite el micrófono e introduce el **código de seis dígitos**. No necesitas copiar una huella digital.
5. Selecciona **LinMic** como entrada en Discord, OBS u otra aplicación.

Los códigos caducan en dos minutos, son de un solo uso y permiten hasta cinco intentos. Las reconexiones se autentican automáticamente. Debes desinstalar una vez el APK de depuración 0.1.0 antes de instalar el APK de release, porque cambia la clave de firma. Las futuras versiones conservarán la clave de release.

## Audio e interfaz

Opus mono a 48 kHz; Ultra (5 ms), Balanced (10 ms), Stable (20 ms). Ganancia hasta +24 dB, cambios suaves y limitador a -1 dBFS. La ganancia del teléfono y la del escritorio son independientes. Empieza con 0 dB: la ganancia digital también amplifica el ruido existente.

Las entradas combinan el tipo y nombre del dispositivo Android y se actualizan al conectar o retirar accesorios. Android no siempre expone cada cápsula de micrófono. Bluetooth puede aumentar la latencia. La latencia mostrada es estimada, no una medición acústica.

Incluye silencio, reconexión automática, forma de onda, medidores, estadísticas, bandeja y atajos globales opcionales mediante el portal del escritorio. Cerrar la interfaz no detiene el servicio. Ambos apps ofrecen portugués, inglés, español, ruso, chino simplificado y japonés. Idioma Android: Ajustes → Interfaz. El selector de idioma del escritorio requiere reiniciar la interfaz.

## Compilar

```sh
sudo pacman -S --needed rust cmake ninja clang pkgconf pipewire opus gtk4
cargo build --locked --release --workspace
cargo test --workspace
```

Android requiere JDK 17, SDK 36, NDK 28.2.13676358, CMake 3.22.1, Python y los destinos Rust `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`. Ejecuta `./gradlew assembleDebug testDebugUnitTest lintDebug` en `android/`.

Firma local: `python3 scripts/sign-android.py`. Guarda una copia privada de `~/.local/share/linmic-signing`; nunca la publiques. Puertos LAN: TCP 39820, UDP 39821 y mDNS UDP 5353.

[Seguridad](../../SECURITY.md) · [Diseño de seguridad](../security.md) · [Validación](../validation.md) · [Contribuir](../../CONTRIBUTING.md). Las pruebas funcionales no son una auditoría independiente de seguridad; la cobertura de dispositivos y distribuciones es limitada.

**GPL-3.0-or-later**. Consulta la [licencia](../../LICENSE) y los [avisos de terceros](../../THIRD_PARTY_NOTICES.md). Distribuye el código fuente correspondiente y los avisos junto con los binarios.
