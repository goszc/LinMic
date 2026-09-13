# LinMic

**Português (Brasil)** · [English](docs/i18n/README.en.md) · [Español](docs/i18n/README.es.md) · [Русский](docs/i18n/README.ru.md) · [简体中文](docs/i18n/README.zh-CN.md) · [日本語](docs/i18n/README.ja.md)

[Baixar a última versão](https://github.com/goszc/LinMic/releases/latest)

Use o microfone do Android como uma entrada de áudio do Linux: nativo, sem conta, sem nuvem, com Opus e PipeWire.

**Android 0.3.0 / desktop 0.3.0 — Android 8+ e Linux com PipeWire.** O alvo principal é CachyOS/Arch. Desktop GTK4 opcional; o daemon continua funcionando sem janela aberta.

## Instalação e primeira conexão

1. Instale o pacote Linux da release. No arquivo `.tar.zst`, extraia e execute `./scripts/install-linux.sh` **como seu usuário, sem sudo**. Isso instala em `~/.local/bin` e habilita o serviço do usuário.
2. Abra **LinMic** no menu de aplicativos. Clique em **Parear telefone**.
3. Instale o APK de release no Android, permita o microfone e selecione o PC descoberto na mesma rede. A entrada manual por IP permanece disponível.
4. Digite no Android os **seis dígitos** exibidos no desktop. Não é necessário copiar certificados ou SHA-256. O código expira em 120 segundos, aceita até cinco tentativas e é de uso único.
5. No Discord, OBS ou outro aplicativo, escolha **LinMic** como entrada.

O APK de release usa uma chave diferente do APK inicial de depuração. **Quem instalou o 0.1.0 de teste precisa desinstalá-lo uma vez antes de instalar esta release.** Atualizações futuras devem manter a chave de release.

## Áudio

- Oboe → ring SPSC → Opus → UDP autenticado → jitter buffer → decoder → DSP → PipeWire.
- 48 kHz mono. Ultra: 5 ms / 64 kbit/s; Balanced: 10 ms / 48 kbit/s; Stable: 20 ms / 48 kbit/s.
- Ganho de entrada no Android e ganho de recepção no desktop, até **+24 dB**, com transição suave e limiter a -1 dBFS. Ganho digital também aumenta ruído já presente; comece em 0 dB e ajuste falando normalmente.
- Nomes das entradas combinam o tipo (interno, headset, USB, Bluetooth) e o nome fornecido pelo Android. A lista acompanha conexão/remoção de dispositivos. Android nem sempre expõe microfones físicos superior/inferior separadamente.
- Mute sincronizado, reconexão, PLC/FEC, ajuste de jitter e correção gradual de clock drift.
- Latência mostrada é **estimada**, não uma medição acústica ponta a ponta.

Os dois aplicativos têm interface em português, inglês, espanhol, russo, chinês simplificado e japonês. No Android: Configurações → Interface. No desktop: seletor de idioma, com reinício da interface.

## Desktop e CLI

```sh
linmic-gui
linmic status --json
linmic pair
linmic devices
linmic mute
linmic unmute
linmic toggle-mute
linmic disconnect
linmic config set gain-db 6
linmic config set agc true
linmic config set noise-gate true
linmic stats --watch
linmic-hotkeys --check
linmic-hotkeys --mode toggle
```

A bandeja oferece abrir, alternar mute, desconectar e sair da interface. Sair da interface não encerra o daemon. Atalhos globais usam o portal do desktop; seu ambiente mostra a autorização/configuração de tecla. Também é possível vincular `linmic toggle-mute` nas configurações do KDE/GNOME.

## Rede e privacidade

PC e telefone devem estar na mesma rede privada. USB funciona por **ancoragem USB**, usando as mesmas conexões IP. Portas padrão: **TCP 39820**, **UDP 39821**, mDNS **UDP 5353**. Libere-as somente para a rede local no firewall. Não configure redirecionamento de portas no roteador.

O app não grava nem envia áudio a serviços externos. Pareamento usa SPAKE2 com confirmação vinculada ao certificado TLS; reconexões verificam a identidade previamente autenticada. Áudio usa ChaCha20-Poly1305 e proteção contra replay. Veja [segurança](SECURITY.md), [modelo de ameaças](docs/security.md) e [privacidade](docs/privacy.md).

## Compilar

Linux (Arch/CachyOS):

```sh
sudo pacman -S --needed rust cmake ninja clang pkgconf pipewire opus gtk4
cargo build --locked --release --workspace
./scripts/install-linux.sh
```

O `rust-toolchain.toml` fixa o Rust quando usado com rustup. Em sistemas com Rust do gerenciador de pacotes, confira a versão. `linmicd` e `linmic` não dependem de GTK; para instalá-los sem GUI, compile apenas esses dois pacotes.

Android: JDK 17, SDK 36, build-tools 36.0.0, NDK 28.2.13676358, CMake 3.22.1, Python 3 e rustup. Configure `ANDROID_HOME` e instale os alvos:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cd android
./gradlew assembleDebug testDebugUnitTest lintDebug
```

O Gradle compila a mesma implementação Rust de pareamento para as três ABIs. Downloads nativos e o Gradle Wrapper têm SHA-256 fixado; `Cargo.lock` fixa as dependências Rust.

Para release assinada local: `python3 scripts/sign-android.py`. A chave e credenciais são criadas **fora do repositório**, em `~/.local/share/linmic-signing`, com permissões restritas. Faça backup seguro: perder a chave impede atualizações compatíveis. Para CI, forneça os segredos de assinatura descritos em [releases](docs/release.md).

## Verificação

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cmake -S tests/native -B target/native-tests
cmake --build target/native-tests
ctest --test-dir target/native-tests --output-on-failure
python3 tests/network/integration.py
```

O teste de integração precisa de uma sessão PipeWire; usa portas, identidade e microfone de teste isolados. Verifica Opus/TLS/UDP, gravação do tom, mute, desconexão, reconexão e perda simulada. Não altera o microfone físico.

Testes longos são opcionais e não fazem parte da instalação: `python3 tests/soak/run.py --seconds 3600`. Consulte [validação e limites](docs/validation.md). Não se pressupõe compatibilidade verificada com todos os telefones ou distribuições, nem auditoria independente de segurança.

## Licença e contribuição

GPL-3.0-or-later. Veja [LICENSE](LICENSE), [contribuição](CONTRIBUTING.md) e [avisos de terceiros](THIRD_PARTY_NOTICES.md). Código-fonte e pacotes assinados são publicados neste repositório. Os scripts locais não publicam automaticamente.

### Android 0.3.0

A interface separa Microfone, Conexão e Diagnóstico. Captura compatível vem ativada; pode alterar o som por usar o processamento de voz do Android. Travamentos de captura/transmissão têm recuperação automática limitada. Silêncio digital recebe uma tentativa de recuperação até surgir sinal; silêncio imposto pelo Android é informado e respeitado. Em Diagnóstico, veja ajustes de bateria e histórico técnico copiável sem áudio ou credenciais. O botão Recuperar áudio dispensa desconectar manualmente.

No desktop 0.3.0, a aba **Microfone** reúne nível, ganho e **Ouvir meu microfone**, com volume de retorno independente. Use fones para evitar microfonia. O retorno usa a saída padrão do PC e desliga ao desconectar ou encerrar a interface (fechar a janela mantém a bandeja). Requer `pw-loopback`, `pw-dump`, `pw-link` e `wpctl`, fornecidos por PipeWire e WirePlumber.
