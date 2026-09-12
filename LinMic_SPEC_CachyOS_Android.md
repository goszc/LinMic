# LinMic — Especificação Técnica e de Produto
## Microfone Android de baixa latência para CachyOS/Linux

**Status:** especificação inicial pronta para implementação  
**Nome provisório:** LinMic  
**Prioridade de plataforma:** CachyOS/Arch Linux + Android  
**Objetivo:** substituir soluções como WO Mic com uma implementação nativa, open source, extremamente leve, de baixa latência, sem cloud e sem dependência de driver proprietário.

---

# 0. Instrução principal para a IA de desenvolvimento

Esta especificação deve ser tratada como **fonte de verdade inicial** do projeto.

A IA que iniciar a implementação deve:

1. Não trocar a arquitetura principal sem motivo técnico documentado.
2. Implementar primeiro o caminho mínimo de áudio ponta a ponta:
   - Android captura áudio.
   - Android codifica em Opus.
   - Android envia por UDP.
   - Linux recebe.
   - Linux decodifica.
   - Linux publica como microfone PipeWire.
3. Só depois implementar GUI avançada, pairing completo, efeitos e refinamentos.
4. Nunca realizar alocação, rede, logging pesado ou locks bloqueantes em callbacks de áudio realtime.
5. Manter daemon Linux completamente funcional mesmo sem GUI aberta.
6. Manter o app Android utilizável com a tela apagada enquanto houver streaming ativo.
7. Evitar Electron, Flutter, React Native e runtimes pesados.
8. Priorizar dependências pequenas, maduras e empacotáveis no Arch/CachyOS.
9. Implementar testes automatizados em cada módulo antes de avançar.
10. Documentar qualquer divergência desta especificação em `docs/decisions/ADR-XXXX.md`.

O primeiro resultado funcional esperado não é uma interface bonita. É:

```text
Microfone Android
      ↓
48 kHz mono PCM
      ↓
Opus
      ↓
UDP
      ↓
linmicd
      ↓
Opus decoder
      ↓
PipeWire Audio/Source
      ↓
Discord / OBS / Steam / navegador
```

Quando esse caminho funcionar de maneira estável, construir o restante ao redor dele.

---

# 1. Visão do produto

LinMic permite utilizar o microfone de um telefone Android como um microfone de sistema no Linux.

A experiência ideal é:

1. O usuário instala `linmic` no CachyOS.
2. O daemon inicia como serviço do usuário.
3. O usuário abre LinMic no Android.
4. O PC aparece automaticamente.
5. O usuário toca em **Conectar**.
6. Na primeira conexão ocorre pareamento.
7. O telefone começa a transmitir.
8. Surge no Linux um dispositivo:

```text
LinMic — Galaxy A23
```

9. Esse dispositivo aparece normalmente em:
   - Discord
   - OBS
   - Steam
   - jogos
   - navegador
   - Audacity
   - PipeWire/PulseAudio clients
   - KDE Plasma
   - aplicações Flatpak que tenham permissão de microfone

Nenhuma conta, login ou servidor externo é necessário.

---

# 2. Princípios obrigatórios

## 2.1 Leve

O componente que fica constantemente rodando deve ser o `linmicd`, não a GUI.

Metas iniciais:

- CPU idle do daemon: próximo de 0%.
- RAM idle do daemon: ideal < 15 MB.
- RAM em streaming: ideal < 30 MB.
- GUI Linux: pode consumir mais, pois é opcional.
- Android: evitar frameworks de UI com runtime grande.
- Rede típica: 30–80 kbit/s de áudio + overhead.

Não perseguir números artificiais em detrimento da estabilidade, mas qualquer regressão grande deve ser investigada.

## 2.2 Baixa latência

Metas práticas em rede local boa:

- modo Ultra: latência pipeline estimada < 40 ms quando o hardware permitir;
- modo Balanced: < 60 ms;
- modo Stable: < 100 ms.

Esses números são metas, não garantias universais. Hardware Android, roteador e scheduling do sistema influenciam.

## 2.3 Sem cloud

Nenhum áudio deve sair da LAN/USB por padrão.

Não haverá:

- conta;
- telemetria obrigatória;
- servidor central;
- relay externo;
- analytics de comportamento;
- upload de gravações.

## 2.4 Linux first-class

Linux não deve ser um port secundário.

O backend deve usar PipeWire diretamente.

CachyOS/Arch será o alvo primário de desenvolvimento, mas o código deve evitar dependência específica do CachyOS.

## 2.5 Falhar com segurança

Se a conexão cair:

- o microfone virtual permanece disponível;
- a saída vira silêncio;
- o app tenta reconectar;
- não reproduzir ruído aleatório;
- não reutilizar bytes antigos do buffer;
- não travar PipeWire.

---

# 3. Escopo de versões

## v0.1 — Technical MVP

Obrigatório:

- Android.
- CachyOS/Arch.
- captura via Oboe;
- 48 kHz;
- mono;
- PCM 16-bit na captura;
- Opus;
- UDP;
- conexão por IP manual;
- daemon Linux;
- PipeWire;
- mute;
- ganho básico;
- métricas básicas;
- CLI;
- logs;
- reconexão simples.

Sem GUI Linux obrigatória nesta etapa.

## v0.2 — Usable Alpha

Adicionar:

- descoberta automática mDNS;
- GUI Linux simples;
- visualizer Android;
- visualizer Linux;
- RTT;
- jitter;
- packet loss;
- latência estimada;
- presets de qualidade;
- pareamento por código;
- reconnect automático robusto;
- USB tethering documentado;
- seleção de microfone Android.

## v0.3 — Feature Complete Beta

Adicionar:

- adaptive jitter buffer;
- controle de clock drift;
- noise suppression opcional;
- AGC opcional;
- compressor/limiter simples;
- hotkeys no Linux;
- tray;
- dispositivos pareados;
- autoconnect;
- criptografia de áudio/control channel;
- testes long-running;
- pacotes oficiais de release.

## v1.0

Critérios:

- estável por várias horas;
- reconecta sem reiniciar apps consumidores;
- dispositivo virtual permanece consistente;
- documentação completa;
- empacotamento Arch/CachyOS;
- APK assinado;
- protocolo versionado;
- segurança habilitada por padrão;
- CI reproduzível.

---

# 4. Stack escolhida

## 4.1 Linux daemon

**Linguagem:** Rust

Razões:

- baixo overhead;
- binário nativo;
- segurança de memória;
- bom suporte a threads e atomics;
- bindings atuais de PipeWire;
- excelente ferramenta de testes;
- facilidade de criar CLI e daemon no mesmo workspace.

Bibliotecas previstas:

```text
pipewire
libspa
opus / audiopus ou wrapper mínimo sobre libopus
serde
serde_json
thiserror
tracing
tracing-subscriber
crossbeam-channel ou ring buffer SPSC dedicado
socket2
mdns-sd (quando descoberta automática entrar)
clap (CLI)
toml
```

Evitar Tokio no primeiro MVP se não houver necessidade.

O daemon terá poucas conexões e sockets. `std::net`, threads dedicadas e `poll`/timeouts são suficientes inicialmente e mantêm o sistema simples.

Se posteriormente houver razão real para async, reavaliar.

## 4.2 Linux GUI

Primeira escolha:

**Rust + GTK4 puro**, sem libadwaita obrigatória.

A GUI é pacote opcional.

Motivos:

- toolkit nativo;
- sem navegador embutido;
- integração Linux madura;
- suficiente para uma janela simples;
- UI separada do processo realtime.

Pacotes planejados:

```text
linmic
linmic-gui
```

`linmic` deve funcionar sem `linmic-gui`.

## 4.3 Android

### UI

- Kotlin
- Android SDK nativo
- XML Views
- Custom Views
- sem Jetpack Compose no início
- sem Flutter
- sem React Native

Pode usar AndroidX apenas onde trouxer benefício claro.

### Áudio

- C++
- Oboe
- AAudio quando disponível
- OpenSL ES apenas como fallback fornecido pelo Oboe
- libopus

### Integração

JNI entre Kotlin e C++.

A camada nativa será responsável por:

- captura;
- ring buffer;
- encoder Opus;
- packetização de áudio;
- envio UDP;
- métricas de áudio realtime.

Kotlin será responsável por:

- UI;
- permissões;
- foreground service;
- descoberta;
- pairing/control channel;
- preferências;
- gerenciamento de lifecycle;
- apresentação de métricas.

---

# 5. Estrutura do repositório

```text
linmic/
├── README.md
├── LICENSE
├── CHANGELOG.md
├── Cargo.toml
├── rust-toolchain.toml
├── .editorconfig
├── .gitignore
├── .github/
│   └── workflows/
│       ├── linux.yml
│       ├── android.yml
│       └── release.yml
│
├── protocol/
│   ├── SPEC.md
│   ├── VERSION
│   ├── packets.md
│   └── examples/
│
├── crates/
│   ├── linmic-protocol/
│   ├── linmic-audio/
│   ├── linmic-network/
│   ├── linmic-pipewire/
│   ├── linmic-ipc/
│   └── linmic-common/
│
├── linux/
│   ├── daemon/
│   │   └── linmicd/
│   ├── cli/
│   │   └── linmic/
│   ├── gui/
│   │   └── linmic-gui/
│   ├── systemd/
│   │   └── linmic.service
│   └── packaging/
│       └── arch/
│           ├── PKGBUILD
│           └── linmic.install
│
├── android/
│   ├── app/
│   │   ├── src/main/java/
│   │   ├── src/main/res/
│   │   └── src/main/AndroidManifest.xml
│   └── native/
│       ├── CMakeLists.txt
│       ├── audio_engine/
│       ├── opus/
│       ├── network/
│       └── jni/
│
├── docs/
│   ├── architecture.md
│   ├── android.md
│   ├── linux.md
│   ├── latency.md
│   ├── security.md
│   ├── troubleshooting.md
│   └── decisions/
│
└── tests/
    ├── protocol/
    ├── network/
    ├── audio-fixtures/
    └── soak/
```

---

# 6. Arquitetura geral

```text
                           ANDROID
┌────────────────────────────────────────────────────────┐
│ UI                                                     │
│                                                        │
│ connect / mute / gain / quality / visualizer / stats   │
│                         │                              │
│                         ▼                              │
│                StreamingForegroundService             │
│                         │                              │
│                  JNI control API                      │
│                         │                              │
│                         ▼                              │
│                 NativeAudioEngine                     │
│                                                        │
│ Oboe callback                                          │
│     │                                                  │
│     ▼                                                  │
│ SPSC PCM Ring Buffer                                   │
│     │                                                  │
│     ▼                                                  │
│ Encoder Worker ──► Opus ──► Packetizer ──► UDP        │
│     │                                                  │
│     └────────────► RMS / Peak / Waveform metrics       │
└──────────────────────────────┬─────────────────────────┘
                               │
                          LAN / USB IP
                               │
                               ▼
                           CACHYOS
┌────────────────────────────────────────────────────────┐
│ linmicd                                                │
│                                                        │
│ UDP Receiver                                           │
│     │                                                  │
│     ▼                                                  │
│ packet validation / reorder                            │
│     │                                                  │
│     ▼                                                  │
│ Adaptive Jitter Buffer                                 │
│     │                                                  │
│     ▼                                                  │
│ Opus Decoder                                           │
│     │                                                  │
│     ▼                                                  │
│ DSP: mute / gain / limiter / drift correction         │
│     │                                                  │
│     ▼                                                  │
│ SPSC Output Ring                                       │
│     │                                                  │
│     ▼                                                  │
│ PipeWire realtime callback                            │
│     │                                                  │
│     ▼                                                  │
│ Audio/Source: "LinMic — Phone"                         │
│                                                        │
│ IPC Unix Socket ◄──────── linmic / linmic-gui          │
└────────────────────────────────────────────────────────┘
```

---

# 7. Estados do sistema

A aplicação deve usar uma state machine explícita.

```text
IDLE
DISCOVERING
PAIRING
CONNECTING
CONNECTED
STREAMING
MUTED
RECONNECTING
STOPPING
ERROR
```

Não espalhar `boolean connected`, `boolean streaming`, `boolean muted` sem coordenação.

Definir um estado central.

Exemplo Android:

```kotlin
sealed interface StreamState {
    data object Idle : StreamState
    data object Discovering : StreamState
    data class Connecting(val host: String) : StreamState
    data class Streaming(val sessionId: Long) : StreamState
    data class Muted(val sessionId: Long) : StreamState
    data class Reconnecting(val attempt: Int) : StreamState
    data class Error(val code: ErrorCode, val message: String) : StreamState
}
```

Linux deve usar modelo equivalente.

---

# 8. UX Android

## 8.1 Tela principal

A tela principal deve concentrar quase tudo.

Layout aproximado:

```text
┌──────────────────────────────────┐
│ LinMic                      ⚙    │
│                                  │
│ ● CONECTADO                      │
│ CachyOS-PC                       │
│ Wi‑Fi 5 GHz                      │
│                                  │
│        ~~~ waveform ~~~          │
│   ▁▂▅██▆▃▂▁▃▅▇██▇▅▃▂             │
│                                  │
│   Peak    -6.1 dB                │
│   RMS     -18.4 dB               │
│                                  │
│ [       MUTAR MICROFONE       ]  │
│                                  │
│ Latência estimada      32 ms     │
│ RTT                     8 ms     │
│ Jitter                  2 ms     │
│ Perda                 0.0 %      │
│ Buffer                  16 ms     │
│                                  │
│ Qualidade            Balanced ▾  │
│ Ganho                 100%       │
│ ━━━━━━━━━━━●━━━━━━━━━━            │
│                                  │
│ [ Desconectar ]                  │
└──────────────────────────────────┘
```

## 8.2 Visualizer

Mostrar dois tipos de feedback simultaneamente:

### Waveform

Forma de onda rolando horizontalmente.

- aproximadamente 30 FPS;
- não desenhar diretamente todas as 48.000 amostras;
- gerar buckets min/max ou peak envelope;
- 128–256 pontos visuais;
- usar custom `View.onDraw(Canvas)`;
- dados recebidos por snapshot atômico/buffer de métricas.

### VU meter

Mostrar:

- RMS;
- peak;
- peak hold curto;
- clipping.

Clipping deve acender quando amostras atingirem ~0 dBFS.

## 8.3 Mute

Botão grande central.

Estados:

```text
Ativo:
[ MUTAR MICROFONE ]

Mutado:
[ ATIVAR MICROFONE ]
```

Ao mutar:

- captura pode continuar;
- encoder pode continuar;
- frames devem resultar em silêncio;
- conexão não deve cair;
- latência e RTT continuam sendo medidos;
- PipeWire continua vivo;
- UI Linux recebe `muted=true`.

Motivo: unmute deve ser instantâneo.

Adicionar opção avançada:

```text
Mute econômico
```

Quando habilitado:

- pode interromper encoding contínuo;
- envia keepalive;
- daemon gera silêncio;
- unmute pode demorar alguns milissegundos extras.

Não habilitar por padrão.

## 8.4 Indicador de transmissão real

Não basta mostrar "conectado".

A UI precisa distinguir:

```text
● Conectado e transmitindo
● Conectado, mutado
● Conectado, sem áudio detectado
● Reconectando
● PC não respondendo
```

Exemplo:

```text
TRANSMITINDO
47 packets/s
```

ou:

```text
MICROFONE MUTADO
Conexão permanece ativa
```

## 8.5 Latência

Não chamar RTT de latência de áudio.

Exibir:

```text
Latência estimada: 34 ms
RTT: 9 ms
```

O tooltip/informação explica que "latência estimada" é uma estimativa do pipeline.

## 8.6 Cards de estatísticas

Modo simples:

- Latência.
- Conexão.
- Mic.
- Qualidade.

Modo detalhado:

```text
Capture frame         10 ms
Encode                 1 ms
Network one-way        4 ms
Jitter buffer         15 ms
Decode                <1 ms
PipeWire queue         5 ms
Estimated total       35 ms

RTT                    8 ms
Jitter               1.8 ms
Packet loss          0.1 %
Late packets         0.0 %
Dropped PCM frames      0
Audio xruns              0
```

## 8.7 Seleção do microfone

Tela:

```text
Microfone

● Microfone padrão
○ Microfone inferior
○ Headset USB-C
○ Headset com fio
○ Bluetooth headset
```

Usar dispositivos reportados pelo Android.

Bluetooth deve mostrar aviso:

```text
Bluetooth pode aumentar bastante a latência.
```

## 8.8 Presets de qualidade

### Ultra Low Latency

```text
Frame: 5 ms
Bitrate: 64 kbps
Jitter target: ~10–20 ms
Opus mode: restricted low delay quando apropriado
FEC: off inicialmente
```

### Balanced — padrão

```text
Frame: 10 ms
Bitrate: 48 kbps
Jitter target: ~20–30 ms
Opus application: VOIP
```

### Stable

```text
Frame: 20 ms
Bitrate: 40–48 kbps
Jitter target: ~40–60 ms
FEC: adaptive
```

### Custom

Liberar:

- bitrate;
- frame size;
- jitter target;
- Opus complexity;
- FEC;
- packet loss expectation.

Custom fica em menu avançado.

## 8.9 Gain

Slider:

```text
0% ... 100% ... 200%
```

Internamente usar ganho em dB.

Exemplo:

```text
-∞ a +12 dB
```

100% = 0 dB.

Aplicar limiter leve se ganho positivo gerar clipping.

## 8.10 Noise suppression

Opções:

```text
Noise suppression:
Off
System
Light
Strong
```

Na primeira implementação:

- Off
- System se disponível

Não bloquear MVP esperando DSP perfeito.

A implementação deve permitir substituir depois pelo RNNoise/WebRTC NS sem quebrar a API.

## 8.11 AGC

Opções:

```text
Automatic Gain Control
[ off/on ]
```

Somente ativar se disponível ou se DSP próprio implementado.

## 8.12 Tela desligada

Streaming deve continuar com tela apagada.

Mostrar notificação persistente:

```text
LinMic
Transmitindo para CachyOS-PC

[MUTAR] [DESCONECTAR]
```

A ação MUTE da notificação deve ser imediata.

---

# 9. UX de conexão Android

## 9.1 Descoberta

Tela inicial:

```text
Computadores encontrados

CachyOS-PC
192.168.1.25
LinMic 0.2
[ Conectar ]

Nenhum computador?
[ Inserir IP manualmente ]
```

## 9.2 Primeira conexão

PC mostra:

```text
Novo dispositivo
Galaxy A23

Código de pareamento:
381 729
```

Android:

```text
Digite o código exibido no computador

[ _ _ _  _ _ _ ]

[ Parear ]
```

Após sucesso:

```text
Pareado com CachyOS-PC
```

Guardar associação.

## 9.3 Reconexão

Se Wi-Fi oscilar:

```text
Conexão perdida
Reconectando... tentativa 2
```

Backoff:

```text
0.25 s
0.5 s
1 s
2 s
3 s
5 s
5 s...
```

Ao recuperar:

```text
Reconectado
```

Sem exigir interação.

---

# 10. UX Linux GUI

A GUI deve ser deliberadamente simples.

```text
┌────────────────────────────────────────┐
│ LinMic                              ⚙  │
│                                        │
│ ● Galaxy A23                           │
│   Conectado por Wi‑Fi                  │
│                                        │
│ ▁▂▃▅██▆▄▃▂▃▅▇██▆▄▂                     │
│                                        │
│ ███████████████░░░  -8 dB              │
│                                        │
│ [ Mutar ]                              │
│                                        │
│ Latência       31 ms                   │
│ RTT             7 ms                   │
│ Jitter          2 ms                   │
│ Perda         0.0 %                    │
│                                        │
│ Ganho           0 dB                   │
│ ━━━━━━━●━━━━━━━━                        │
│                                        │
│ [ Desconectar ]                        │
└────────────────────────────────────────┘
```

A GUI nunca manipula PipeWire diretamente.

Fluxo:

```text
linmic-gui
   ↓
Unix domain socket
   ↓
linmicd
```

Fechar GUI não encerra stream.

## 10.1 Tray

Beta:

- connected/disconnected icon;
- mute/unmute;
- open;
- disconnect;
- quit GUI.

"Quit GUI" não encerra daemon.

## 10.2 Atalhos

Beta:

```text
Global mute hotkey
Push-to-talk mode
Push-to-mute mode
```

Não faz parte do MVP.

---

# 11. CLI Linux

Comandos mínimos:

```bash
linmic status
linmic devices
linmic mute
linmic unmute
linmic toggle-mute
linmic disconnect
linmic pair
linmic config show
linmic config set gain-db 3
linmic stats
```

Exemplo:

```text
$ linmic status

State: STREAMING
Phone: Galaxy A23
Address: 192.168.1.37
Codec: Opus
Rate: 48000 Hz
Channels: 1
Frame: 10 ms
Bitrate: 48 kbps
Estimated latency: 31 ms
RTT: 8 ms
Jitter: 1.9 ms
Packet loss: 0.02 %
Muted: no
PipeWire source: LinMic — Galaxy A23
```

Adicionar:

```bash
linmic status --json
```

para scripts.

---

# 12. Android foreground service

Criar:

```text
StreamingForegroundService
```

Responsável por manter sessão ativa.

Manifest deve incluir, conforme API alvo:

```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
<uses-permission android:name="android.permission.FOREGROUND_SERVICE_MICROPHONE" />
<uses-permission android:name="android.permission.WAKE_LOCK" />
```

Service:

```xml
<service
    android:name=".service.StreamingForegroundService"
    android:foregroundServiceType="microphone"
    android:exported="false" />
```

Fluxo:

1. Activity solicita `RECORD_AUDIO`.
2. Usuário toca Conectar/Transmitir.
3. Activity chama `startForegroundService`.
4. Service entra em foreground imediatamente.
5. Service inicia native audio engine.
6. Notificação permanece visível.
7. Ao desconectar, engine para e service é encerrado.

Nunca tentar iniciar serviço de microfone silenciosamente no boot.

---

# 13. Power management Android

Enquanto streaming:

- manter foreground service;
- usar `PARTIAL_WAKE_LOCK` somente durante sessão ativa se testes mostrarem necessidade;
- liberar sempre em `stopStreaming()`;
- considerar `WIFI_MODE_FULL_LOW_LATENCY` em Android compatível enquanto app estiver realmente em foreground;
- não manter locks quando desconectado.

Adicionar configuração:

```text
Priorizar baixa latência Wi‑Fi
[on]
```

Explicar maior consumo de bateria.

Não solicitar "ignore battery optimizations" automaticamente no MVP.

Somente adicionar guia se aparelhos específicos matarem a aplicação.

---

# 14. Captura de áudio Android

## 14.1 Configuração alvo

```text
Direction: Input
Channels: Mono
Format: I16 inicialmente
Requested rate: natural do device ou 48000 com conversão Oboe
PerformanceMode: LowLatency
SharingMode: Exclusive request, Shared fallback
InputPreset: VoiceRecognition padrão
```

O código deve verificar e registrar os valores reais obtidos:

```text
actual sample rate
actual channel count
actual sharing mode
actual performance mode
frames per burst
device id
xrun count
```

Não assumir que o Android honrou todas as requests.

## 14.2 Regra absoluta do callback

No callback Oboe:

Pode:

- copiar amostras;
- calcular operações simples;
- escrever em ring buffer lock-free;
- incrementar counters atomics.

Não pode:

- fazer UDP send;
- acessar arquivo;
- logar a cada callback;
- alocar heap;
- usar mutex que possa bloquear;
- chamar JNI;
- serializar JSON;
- dormir;
- executar Opus encoder se isso causar variabilidade relevante.

Pipeline:

```text
Oboe callback
    ↓
preallocated SPSC ring
    ↓
Encoder thread
```

## 14.3 Overflow

Se ring cheio:

- descartar frame mais recente ou mais antigo conforme estratégia definida;
- incrementar `capture_ring_overruns`;
- nunca bloquear callback.

Preferência: descartar frame mais antigo para minimizar latência acumulada.

---

# 15. Encoder worker

Thread dedicada.

Prioridade acima do normal, mas não abusar de realtime scheduling.

Responsabilidades:

1. retirar exatamente N samples;
2. aplicar mute/gain móvel se configurado no lado Android;
3. calcular RMS/peak;
4. gerar envelope para waveform;
5. codificar Opus;
6. criar packet;
7. enviar UDP;
8. atualizar métricas.

Para 10 ms:

```text
480 samples mono @ 48 kHz
```

Para 5 ms:

```text
240 samples
```

Para 20 ms:

```text
960 samples
```

---

# 16. Opus

Formato wire padrão:

```text
48000 Hz
mono
Opus
```

Mesmo se hardware capturar em sample rate diferente, normalizar antes do encoder.

Configuração Balanced inicial:

```text
OPUS_APPLICATION_VOIP
bitrate = 48000
complexity = 5
VBR = true
DTX = false
in-band FEC = false inicialmente
packet loss perc = 0 inicialmente
frame = 10 ms
```

Complexity deve ser configurável internamente.

Não usar complexity 10 sem benchmark; telefone pode gastar CPU desnecessariamente.

## 16.1 FEC

Ativar adaptativamente somente quando:

```text
packet loss > threshold
```

Exemplo:

```text
> 1% por 3 s
```

Atualizar `OPUS_SET_PACKET_LOSS_PERC`.

## 16.2 PLC

No Linux, quando packet ausente:

- usar Opus PLC;
- não repetir cegamente o pacote anterior.

---

# 17. Protocolo de áudio

Usar UDP.

Porta padrão sugerida:

```text
39821/udp
```

Control:

```text
39820/tcp
```

Ambas configuráveis.

## 17.1 Header binário

Todos os inteiros em network byte order.

Proposta v1:

```c
struct LinMicAudioHeaderV1 {
    uint32_t magic;          // "LMIC" = 0x4C4D4943
    uint8_t  version;        // 1
    uint8_t  type;           // 1 = audio
    uint16_t flags;

    uint32_t session_id;
    uint32_t sequence;

    uint64_t sample_index;
    uint64_t sender_time_us;

    uint16_t frame_samples;
    uint16_t payload_len;

    uint32_t reserved;
};
```

Depois:

```text
Opus payload[payload_len]
```

Flags:

```text
0x0001 MUTED
0x0002 FEC_ENABLED
0x0004 DISCONTINUITY
0x0008 KEEPALIVE_AUDIO
```

O receiver deve:

- validar magic;
- validar versão;
- validar payload_len;
- limitar tamanho;
- ignorar session id inválida;
- descartar packets muito antigos;
- não fazer panic por packet malformado.

## 17.2 Sequence

`sequence` incrementa por frame.

Permite detectar:

- perda;
- reordenação;
- duplicatas.

## 17.3 sample_index

Número cumulativo de samples desde início da sessão.

Ajuda:

- sincronização;
- drift;
- reconstrução de timeline.

---

# 18. Control channel

Inicialmente TCP.

Mensagens:

```text
uint32 length
UTF-8 JSON
```

Control traffic é pequeno; JSON simplifica Android + Rust e debugging.

Exemplo hello:

```json
{
  "type": "hello",
  "protocol": 1,
  "client": {
    "name": "Galaxy A23",
    "app_version": "0.2.0",
    "android_api": 34
  },
  "audio": {
    "codecs": ["opus"],
    "rates": [48000],
    "channels": [1],
    "frames_ms": [5, 10, 20]
  }
}
```

Server response:

```json
{
  "type": "hello_ack",
  "protocol": 1,
  "server": {
    "name": "CachyOS-PC",
    "version": "0.2.0"
  },
  "session_id": 123456789,
  "udp_port": 39821
}
```

## 18.1 Mensagens obrigatórias

```text
hello
hello_ack
pair_request
pair_challenge
pair_submit
pair_success
pair_error

start_stream
start_stream_ack
stop_stream

mute
unmute
set_gain
set_profile

ping
pong

stats_client
stats_server

disconnect

error
```

## 18.2 Versionamento

Control message sempre inclui ou herda protocol version.

Se incompatível:

```json
{
  "type": "error",
  "code": "PROTOCOL_UNSUPPORTED",
  "supported": [1]
}
```

---

# 19. Discovery mDNS

Linux anuncia:

```text
Service type:
_linmic._tcp.local.
```

TXT:

```text
version=1
name=CachyOS-PC
control_port=39820
requires_pairing=1
```

Android usa NSD/DNS-SD.

A descoberta é conveniência, não pré-requisito.

Sempre manter entrada manual:

```text
IP / hostname
port
```

---

# 20. Pairing

## v0.2

Primeiro pairing:

1. Android pede conexão.
2. Linux gera 6 dígitos.
3. Código válido por 120 s.
4. Usuário digita.
5. PC gera `device_token` aleatório.
6. Android guarda token.
7. Conexões futuras apresentam token.

Isso impede conexão acidental, mas **não deve ser descrito como criptografia**.

## v1.0

Adicionar troca autenticada de chaves e AEAD.

Meta:

- X25519;
- HKDF-SHA256;
- ChaCha20-Poly1305 por datagram;
- nonce derivado de session + sequence;
- replay protection.

Antes de implementar criptografia própria, revisar desenho com bibliotecas maduras.

---

# 21. Medição de latência

## 21.1 Não mentir para o usuário

Sem loopback físico, não é possível medir exatamente a latência acústica total microfone→aplicação.

Exibir:

```text
Latência estimada
```

## 21.2 Sync estilo NTP

Usar quatro timestamps:

```text
t0 = Android envia ping
t1 = Linux recebe
t2 = Linux envia pong
t3 = Android recebe
```

Calcular:

```text
RTT ≈ (t3 - t0) - (t2 - t1)

clock_offset ≈ ((t1 - t0) + (t2 - t3)) / 2
```

Manter média/mediana de várias medições.

Descartar outliers.

## 21.3 Estimativa pipeline

```text
estimated_latency =
    capture_frame_ms
  + encoder_queue_ms
  + encode_time_ms
  + estimated_network_one_way_ms
  + jitter_buffer_delay_ms
  + decode_time_ms
  + output_ring_delay_ms
  + pipewire_quantum_estimate_ms
```

Mostrar valor suavizado por EMA.

---

# 22. Jitter

Calcular diferença de transit time entre packets.

Manter:

```text
current jitter
p50
p95 opcional
```

UI mostra valor suavizado.

---

# 23. Jitter buffer Linux

Estrutura ordenada por sequence/sample index.

Não usar container que aloque por packet em steady state.

Preferir ring circular prealocado.

Exemplo:

```text
slots[512]
```

Cada slot:

```text
sequence
present
payload_len
payload[max_opus_packet]
arrival_timestamp
```

## 23.1 Target

Balanced inicial:

```text
20 ms
```

Adaptive:

```text
min 10 ms
max 80 ms
```

Aumentar se:

- packets chegam tarde;
- jitter sobe;
- perda aparente por late arrival.

Diminuir lentamente quando estável.

Nunca variar agressivamente e causar pumping.

---

# 24. Receiver Linux

Thread de rede:

```text
recv UDP
   ↓
validate
   ↓
timestamp arrival
   ↓
stats
   ↓
jitter ring
```

Thread decode:

```text
jitter scheduler
   ↓
Opus decode / PLC
   ↓
drift correction
   ↓
gain/mute/limiter
   ↓
output SPSC ring
```

PipeWire callback:

```text
output SPSC ring
   ↓
PipeWire buffer
```

Nenhuma rede no callback PipeWire.

---

# 25. Clock drift

Problema:

O clock efetivo do ADC Android e o clock do PipeWire não serão exatamente iguais.

Mesmo que ambos reportem 48 kHz:

```text
Android real: 48002 Hz
PC real:      47998 Hz
```

Ao longo de minutos o buffer pode crescer ou secar.

## Solução

Monitorar ocupação do output/jitter buffer.

Implementar resampler de correção muito pequena:

```text
ratio 0.995 ... 1.005
```

Controlador PI simples tenta manter target occupancy.

Primeira implementação permitida:

- linear interpolation para voz;
- sem dependência extra.

Depois pode migrar para SpeexDSP/soxr se necessário.

Evitar drop/duplicate brusco de frames, exceto como recuperação emergencial.

---

# 26. Pipeline DSP Linux

Ordem:

```text
Opus decode
   ↓
drift resample
   ↓
mute
   ↓
gain
   ↓
optional noise processing
   ↓
limiter
   ↓
metering
   ↓
PipeWire
```

Limiter simples:

- ceiling ~ -1 dBFS;
- attack rápido;
- release moderado.

MVP pode usar hard clamp somente para proteção, substituído depois.

---

# 27. Semântica de mute

Há três conceitos diferentes.

## Phone mute

Usuário toca mute no Android.

Fonte de verdade global passa a `muted=true`.

Linux recebe evento imediatamente.

Audio packets seguintes têm flag MUTED e/ou payload silencioso.

## PC mute

Usuário toca mute na GUI Linux.

Daemon silencia localmente e envia estado ao telefone.

Telefone exibe mutado.

## Hardware capture stop

Não é o mute normal.

É "Stop streaming".

Ao parar:

- captura encerra;
- UDP encerra;
- foreground service encerra;
- daemon permanece pronto.

A state machine deve evitar conflito entre mute local e remoto.

Usar estado sincronizado com revision number:

```json
{
  "type": "mute",
  "muted": true,
  "revision": 42
}
```

Maior revision vence.

---

# 28. PipeWire

Objetivo: criar uma fonte virtual persistente enquanto `linmicd` estiver rodando.

Nome técnico:

```text
linmic_input
```

Descrição:

```text
LinMic — Galaxy A23
```

Propriedades desejadas:

```text
media.type=Audio
media.category=Capture
media.role=Communication
media.class=Audio/Source
node.virtual=true
audio.channels=1
audio.rate=48000
```

Formato inicial:

```text
S16LE mono 48 kHz
```

ou F32LE internamente se API simplificar DSP.

## 28.1 Requisito importante

O source não deve desaparecer quando Wi-Fi cair momentaneamente.

Enquanto daemon existir:

```text
no phone => silence
phone connected => audio
```

Isso evita que Discord/OBS perca o device.

## 28.2 Callback PipeWire

O process callback:

- dequeue buffer;
- tentar ler SPSC;
- se houver PCM, copiar;
- se faltar, preencher silêncio;
- atualizar underrun counter;
- devolver buffer.

Não:

- esperar UDP;
- esperar decoder;
- lock de mutex;
- fazer heap allocation.

---

# 29. IPC local Linux

Socket:

```text
$XDG_RUNTIME_DIR/linmic/linmic.sock
```

Permissão:

```text
0600
```

Protocolo:

```text
length-prefixed JSON
```

Clientes:

- `linmic`;
- `linmic-gui`.

Eventos assíncronos:

```json
{
  "event": "stats",
  "latency_ms": 31.2,
  "rtt_ms": 8.0,
  "jitter_ms": 1.7,
  "packet_loss": 0.001,
  "rms_db": -18.1,
  "peak_db": -5.4
}
```

Frequência GUI:

```text
10–30 updates/s
```

Stats que mudam pouco podem ir a 2 Hz.

Waveform:

- enviar envelope já reduzido;
- não enviar PCM bruto pela IPC para desenhar GUI.

---

# 30. Configuração Linux

Arquivo:

```text
~/.config/linmic/config.toml
```

Exemplo:

```toml
[network]
control_port = 39820
audio_port = 39821
bind = "0.0.0.0"

[audio]
sample_rate = 48000
channels = 1
default_profile = "balanced"
gain_db = 0.0

[jitter]
mode = "adaptive"
target_ms = 20
min_ms = 10
max_ms = 80

[pipewire]
node_name = "linmic_input"
node_description = "LinMic"

[discovery]
enabled = true

[logging]
level = "info"
```

Segredos não ficam aqui.

Estado:

```text
~/.local/state/linmic/
```

Paired devices:

```text
~/.local/share/linmic/devices.json
```

Arquivo com segredo deve ser `0600`.

---

# 31. Android settings persistentes

Usar `SharedPreferences` inicialmente.

Preferências:

```text
last_pc
auto_reconnect
profile
bitrate
gain
preferred_input_device
noise_suppression
agc
wifi_low_latency
show_advanced_stats
keep_screen_on
theme
```

Token de pairing deve ser armazenado usando Android Keystore ou proteção equivalente antes de release público.

---

# 32. Reconnect

Heartbeat control:

```text
ping interval: 1 s
dead after: 3–5 missed responses
```

Audio timeout:

```text
if no packet > 500 ms:
state = stalled
output silence
```

Reconexão não deve destruir PipeWire node.

Quando client retorna:

1. control reconnect;
2. session resume ou new session;
3. reset jitter buffer;
4. reset Opus decoder;
5. marcar discontinuity;
6. fade in de ~5–10 ms para evitar click.

---

# 33. Handling de troca de rede

Android deve escutar ConnectivityManager.

Casos:

- Wi-Fi troca IP;
- sai do Wi-Fi;
- entra USB tethering;
- VPN altera rota;
- rede perde default.

Em mudança:

```text
pause sender
re-resolve PC
reconnect control
open new UDP path
resume
```

Não manter socket morto indefinidamente.

---

# 34. USB

MVP USB será via **USB tethering**.

Vantagem:

- mesma stack TCP/UDP;
- nenhuma dependência de ADB;
- nenhuma API USB custom;
- geralmente menor variabilidade que Wi-Fi congestionado.

O app detecta que PC está acessível pelo endereço da interface e continua igual.

Futuro opcional:

- ADB transport;
- Android Open Accessory;
- USB custom.

Não implementar antes de existir necessidade.

---

# 35. Métricas

## Android

Counters:

```text
capture_callbacks
capture_frames
capture_ring_overruns
capture_xruns
encoder_frames
encoder_errors
udp_packets_sent
udp_bytes_sent
udp_send_errors
control_reconnects
rms
peak
```

## Linux

```text
udp_packets_received
udp_bytes_received
invalid_packets
duplicate_packets
out_of_order_packets
lost_packets
late_packets
plc_frames
decoder_errors
jitter_buffer_ms
output_ring_ms
pipewire_underruns
resample_ratio
rtt_ms
jitter_ms
estimated_latency_ms
rms_db
peak_db
```

---

# 36. Visualizer internals

## Android

Encoder worker calcula para cada frame:

```text
peak = max(abs(sample))
rms = sqrt(mean(sample^2))
```

Waveform:

Dividir frame/janela em buckets.

Cada bucket produz:

```text
min
max
```

Guardar últimos ~128–256 buckets.

Kotlin consulta snapshot no máximo 30 Hz.

JNI:

```text
nativeGetMeterSnapshot()
```

retorna estrutura pequena.

Não transferir arrays gigantes a 60 FPS.

## Linux

Daemon calcula métricas pós-DSP.

GUI recebe ~128 valores de envelope a 20–30 Hz.

---

# 37. UI settings Android detalhada

```text
Connection
 ├─ Auto reconnect
 ├─ Prefer last PC
 ├─ Manual host
 └─ Discovery

Audio
 ├─ Input device
 ├─ Profile
 ├─ Gain
 ├─ Noise suppression
 ├─ AGC
 ├─ Limiter
 └─ Advanced codec

Latency
 ├─ Automatic
 ├─ Ultra
 ├─ Balanced
 ├─ Stable
 ├─ Jitter target
 └─ Wi-Fi low latency

Interface
 ├─ Show waveform
 ├─ Show advanced statistics
 ├─ Keep screen on
 └─ Theme

Privacy
 ├─ Paired computers
 └─ Remove pairing

About
 ├─ App version
 ├─ Protocol version
 ├─ Licenses
 └─ Open source
```

---

# 38. GUI Linux settings

```text
General
 ├─ Start daemon on login
 ├─ Show tray
 ├─ Notifications
 └─ Default name

Audio
 ├─ Gain
 ├─ Limiter
 ├─ PipeWire node name
 └─ Keep source alive

Network
 ├─ Discovery
 ├─ Control port
 ├─ UDP port
 └─ Allowed interfaces

Pairing
 ├─ paired phones
 ├─ forget device
 └─ require pairing

Advanced
 ├─ logging
 ├─ jitter
 ├─ stats
 └─ diagnostics
```

---

# 39. Logging

Linux:

```text
tracing
```

Levels:

```text
error
warn
info
debug
trace
```

Default release:

```text
info
```

Não logar cada packet.

Exemplos bons:

```text
INFO stream started session=...
INFO android device connected name=...
WARN jitter buffer underrun
WARN audio packet loss 2.3%
ERROR pipewire connection lost
```

Android:

usar Logcat em debug.

Release deve reduzir logging.

Nenhum PCM deve ser salvo por padrão.

---

# 40. Diagnostics

Botão:

```text
Export diagnostics
```

Gera texto/JSON com:

- versions;
- Android API/model genérico;
- codec config;
- PipeWire version;
- latency stats;
- counters;
- recent errors.

Não incluir áudio.

No Linux:

```bash
linmic diagnostics > linmic-diagnostics.txt
```

---

# 41. Erros de produto

Códigos estáveis:

```text
NETWORK_UNREACHABLE
CONTROL_TIMEOUT
PROTOCOL_UNSUPPORTED
PAIRING_REQUIRED
PAIRING_INVALID_CODE
PAIRING_EXPIRED
AUDIO_PERMISSION_DENIED
AUDIO_DEVICE_UNAVAILABLE
AUDIO_STREAM_OPEN_FAILED
OPUS_INIT_FAILED
UDP_BIND_FAILED
PIPEWIRE_UNAVAILABLE
PIPEWIRE_STREAM_FAILED
IPC_FAILED
UNKNOWN
```

UI converte em mensagem amigável.

Nunca mostrar apenas stack trace ao usuário.

---

# 42. systemd user service

Arquivo:

```ini
[Unit]
Description=LinMic Android microphone receiver
After=pipewire.service
Wants=pipewire.service

[Service]
Type=simple
ExecStart=/usr/bin/linmicd
Restart=on-failure
RestartSec=1

[Install]
WantedBy=default.target
```

Instalado em:

```text
/usr/lib/systemd/user/linmic.service
```

Ativação:

```bash
systemctl --user enable --now linmic.service
```

Pacote não deve forçar enable silenciosamente se guidelines da distro desaconselharem.

GUI pode oferecer botão que executa ação apropriada via systemd user.

---

# 43. CachyOS / Arch desenvolvimento

Dependências de build previstas:

```text
base-devel
rustup ou rust
clang
cmake
ninja
pkgconf
pipewire
pipewire-audio
opus
gtk4      # apenas GUI
```

Comando inicial orientativo:

```bash
sudo pacman -S --needed base-devel rustup clang cmake ninja pkgconf pipewire pipewire-audio opus gtk4
rustup default stable
```

Android:

- Android Studio ou CLI SDK;
- Android SDK;
- NDK;
- CMake;
- Ninja.

A IA deve confirmar nomes atuais de pacotes antes de automatizar instalação.

---

# 44. PKGBUILD

Criar primeiro pacote:

```text
linmic-git
```

Depois release:

```text
linmic
```

Split packages idealmente:

```text
linmic
linmic-gui
```

`linmic` contém:

```text
/usr/bin/linmicd
/usr/bin/linmic
/usr/lib/systemd/user/linmic.service
/usr/share/licenses/linmic/
```

`linmic-gui`:

```text
/usr/bin/linmic-gui
/usr/share/applications/linmic.desktop
/usr/share/icons/...
```

Não instalar em `/usr/local` via PKGBUILD.

---

# 45. Firewall

Discovery e streaming precisam LAN.

A aplicação deve detectar provável bloqueio e mostrar:

```text
PC encontrado, mas a conexão foi recusada.
Verifique firewall para TCP 39820 e UDP 39821.
```

Não editar firewall automaticamente sem permissão.

Documentar:

- firewalld;
- ufw;
- nftables.

---

# 46. Segurança

## MVP interno

Pode operar sem criptografia somente para desenvolvimento em LAN confiável.

A UI debug deve indicar:

```text
Development build — unencrypted LAN audio
```

## Release pública

Requer:

- pairing autenticado;
- encrypted control;
- encrypted audio;
- replay protection;
- device revocation;
- random session IDs;
- bound packet sizes;
- fuzz tests do parser;
- no remote arbitrary file access;
- no shell command exposure.

O daemon deve rodar como usuário, nunca root.

---

# 47. Parser defensivo

Limites:

```text
max control frame: 64 KiB
max UDP datagram expected: 1500 bytes
max Opus payload: definir limite seguro
max device name: 128 UTF-8 bytes
max paired devices: 100
```

Rejeitar valores absurdos.

Nunca usar tamanho fornecido pela rede para alocação ilimitada.

---

# 48. Threads Linux

Modelo inicial:

```text
Main/control thread
UDP receive thread
Decode/jitter thread
PipeWire thread/mainloop
IPC accept thread
Stats aggregator
```

Comunicação por:

- atomics;
- SPSC rings;
- channels somente fora realtime.

Evitar thread por packet/conexão.

---

# 49. Threads Android

```text
Main/UI thread
Foreground service
Oboe realtime callback thread
Encoder/network worker
Control TCP worker
Stats/UI sampling
```

Nenhuma UI no thread nativo realtime.

---

# 50. Performance targets

Android Balanced:

```text
48 kHz mono
10 ms
48 kbps Opus
CPU encoder target: < 5% em telefone moderno médio
network: < 10 KB/s típico total
```

Linux:

```text
daemon CPU streaming target: < 3% de um core moderno
idle: ~0%
```

Essas metas devem ser medidas, não presumidas.

---

# 51. Benchmark command

Adicionar futuramente:

```bash
linmic benchmark
```

Pode testar:

- Opus encode/decode speed;
- buffer performance;
- local synthetic packets;
- PipeWire feed.

Android debug screen:

```text
Audio Engine Diagnostics
```

com:

- frames/burst;
- callback interval;
- xrun count;
- encoder avg/p95 time.

---

# 52. Testes unitários

## Protocol

- valid header;
- invalid magic;
- unsupported version;
- truncated payload;
- huge payload;
- sequence wraparound;
- duplicate;
- endian.

## Jitter

- ordered;
- out of order;
- missing;
- delayed;
- duplicate;
- burst loss;
- wraparound.

## Stats

- packet loss;
- jitter;
- RTT;
- EMA;
- clock offset.

## DSP

- gain;
- mute;
- limiter;
- RMS;
- peak.

---

# 53. Integration tests

Criar synthetic sender Linux para não depender de Android durante desenvolvimento.

```bash
linmic-test-sender --tone 440
```

Ele envia Opus para `linmicd`.

Isso permite desenvolver Linux primeiro.

Também criar receiver debug:

```bash
linmic-test-receiver
```

para validar Android antes do PipeWire.

Essa separação é altamente recomendada.

---

# 54. Teste ponta a ponta

Procedimento:

1. iniciar daemon;
2. verificar source via `wpctl status`;
3. Android conecta;
4. falar;
5. gravar source no Linux;
6. confirmar áudio;
7. mutar no Android;
8. confirmar silêncio;
9. desmutar;
10. desligar Wi-Fi;
11. source permanece;
12. ligar Wi-Fi;
13. reconecta;
14. aplicação consumidora não precisou trocar input.

---

# 55. Soak test

Executar:

```text
1 hora
8 horas
24 horas
```

Observar:

- memória;
- CPU;
- packet counters;
- drift;
- buffer occupancy;
- reconnect;
- leaks;
- crashes;
- latency growth.

Critério:

latência não pode crescer continuamente.

Se crescer, há bug de clock/buffer.

---

# 56. Testes de rede

Simular:

```text
0% loss
1% loss
3% loss
5% loss

jitter:
5 ms
20 ms
50 ms

reorder
burst loss
```

No Linux pode usar `tc netem` em ambiente de teste.

Nunca exigir root no app normal.

---

# 57. Android compatibility

Meta inicial:

```text
minSdk: 26 ou 27
targetSdk: versão atual no momento do build
```

Decisão final de minSdk deve considerar Oboe e esforço de manutenção.

Como foco é app novo e baixa latência, não sacrificar arquitetura para Android muito antigo.

Testar pelo menos:

- Android 10;
- Android 12;
- Android 14;
- Android 15/16 ou versões disponíveis no momento.

---

# 58. PipeWire compatibility

CachyOS atual é prioridade.

Também testar:

- Arch;
- Fedora;
- Ubuntu moderno;
- Debian moderno, quando versão de PipeWire for suficiente.

PulseAudio puro não é requisito inicial.

PipeWire-Pulse compatibility naturalmente permite que apps Pulse vejam a source.

---

# 59. Flatpak

Aplicações Flatpak podem depender de portal/permissões próprias.

Não tratar falha específica de sandbox como falha de LinMic.

Documentar troubleshooting.

---

# 60. Audio device naming

Quando desconectado:

```text
LinMic
```

Quando conectado:

```text
LinMic — Galaxy A23
```

Porém mudar `node.name` dinamicamente pode confundir apps.

Recomendação:

```text
node.name = linmic_input
node.description = LinMic — Galaxy A23
```

Manter technical node name estável.

---

# 61. Volume / gain ownership

Evitar múltiplos sliders conflitantes.

Definir:

## Android gain

Pré-encode.

Útil quando mic está baixo.

## Linux gain

Pós-decode.

Útil para controlar nível recebido sem alterar telefone.

A UI padrão pode sincronizar ambos como um "Ganho" lógico no futuro, mas internamente manter separados.

MVP:

- Android input gain;
- Linux receive gain.

---

# 62. Waveform privacy

Visualizer é processado em memória.

Não salvar áudio.

Ao mutar:

- waveform deve cair para zero se mute ocorre antes do meter "transmitted".
- opcionalmente mostrar um segundo meter "raw mic" apenas em tela de diagnóstico.

Tela normal deve representar áudio efetivamente transmitido.

---

# 63. Notificações

Android:

- streaming started;
- disconnected;
- reconnect failed por muito tempo.

Linux desktop:

- phone connected;
- phone disconnected;
- severe quality issue opcional.

Evitar spam.

---

# 64. Quality health indicator

Criar status derivado:

```text
Excellent
Good
Unstable
Poor
```

Exemplo heurística:

Excellent:
- RTT < 20 ms
- jitter < 5 ms
- loss < 0.5%

Good:
- RTT < 50
- jitter < 15
- loss < 2%

Unstable:
- loss/jitter maior

Não usar como métrica científica; apenas UX.

---

# 65. First-run Linux GUI

```text
Welcome to LinMic

✓ PipeWire detected
✓ LinMic daemon running
✓ Virtual microphone created
✓ Discovery enabled

Install the Android app and connect.

[ Pair a phone ]
```

Se PipeWire não estiver:

```text
PipeWire is required.
```

Mostrar diagnóstico.

---

# 66. First-run Android

```text
Use your phone as a Linux microphone

1. Allow microphone access
2. Make sure phone and PC are on the same network
3. Select your computer

[ Continue ]
```

Depois permission.

Não pedir permissões irrelevantes.

---

# 67. Themes

Android:

- System;
- Dark;
- Light.

Linux:

seguir GTK/system theme.

Não gastar tempo com custom theming no MVP.

---

# 68. Accessibility

- botões com content descriptions;
- cores não podem ser único indicador;
- tamanho de toque >= recomendado;
- waveform não essencial para compreender estado;
- status textual sempre presente.

---

# 69. Localization

Arquitetura preparada para:

```text
pt-BR
en-US
```

pt-BR pode ser primeira língua.

Nunca hardcodar todas as strings na Activity.

Android: `strings.xml`.

Linux GTK: inicialmente arquivo/localization layer simples; gettext depois.

---

# 70. Versionamento

App:

```text
0.1.0
```

Protocol:

```text
1
```

Separar app version de protocol version.

Não quebrar wire protocol silenciosamente.

---

# 71. CI Linux

Em push/PR:

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

Adicionar sanitizers/fuzz posteriormente.

---

# 72. CI Android

```text
./gradlew lint
./gradlew test
./gradlew assembleDebug
```

Native:

- CMake compile;
- clang warnings;
- unit tests quando possível.

---

# 73. Release artifacts

Linux:

```text
linmic-x86_64.tar.zst
Arch package
PKGBUILD/AUR
```

Android:

```text
linmic-vX.Y.Z.apk
```

Posteriormente F-Droid pode ser avaliado.

---

# 74. Licença

Sugestão inicial:

```text
GPL-3.0-or-later
```

se a intenção for garantir que forks distribuídos permaneçam open source.

Alternativa se quiser máxima permissividade:

```text
Apache-2.0
```

Não iniciar repositório público sem escolher explicitamente.

Para desenvolvimento local inicial, usar placeholder documentado até decisão.

---

# 75. Dependências proibidas sem justificativa

Evitar:

- Electron;
- embedded Chromium;
- Flutter;
- Node runtime no desktop;
- Python runtime para daemon;
- Java desktop runtime;
- PulseAudio-only driver hacks;
- kernel modules;
- ALSA loopback obrigatório;
- root daemon;
- cloud relay.

---

# 76. Architecture Decision Records

Criar ADR para decisões importantes.

Exemplo:

```text
docs/decisions/
ADR-0001-rust-linux-daemon.md
ADR-0002-oboe-android.md
ADR-0003-opus-over-udp.md
ADR-0004-pipewire-source.md
ADR-0005-gtk4-optional-ui.md
```

Formato:

```text
Context
Decision
Consequences
Alternatives considered
```

---

# 77. Primeira sequência de implementação

## Milestone 0 — Bootstrap

A IA deve:

1. criar repo;
2. Cargo workspace;
3. Android Gradle project;
4. CMake native Android;
5. CI básico;
6. docs;
7. protocol version file.

Nenhuma GUI avançada.

## Milestone 1 — Linux virtual microphone

Criar `linmicd` que:

- conecta PipeWire;
- cria `Audio/Source`;
- gera um tom de teste;
- source aparece no sistema.

Critério:

```bash
wpctl status
```

mostra LinMic e Audacity consegue gravar tom.

Depois substituir tom por ring buffer.

## Milestone 2 — Local synthetic Opus

Criar `linmic-test-sender` Linux.

Fluxo:

```text
WAV/tone
→ Opus encode
→ UDP
→ linmicd
→ Opus decode
→ PipeWire
```

Critério:

tom chega via rede localhost.

## Milestone 3 — Android raw capture

Criar app Android:

- permission;
- Oboe opens;
- meter funciona;
- waveform funciona;
- sem rede ainda.

Critério:

falar move meter.

## Milestone 4 — Android Opus/UDP

Android envia para IP manual.

Critério:

fala aparece no PipeWire.

Esse é o primeiro MVP real.

## Milestone 5 — Controls

Adicionar:

- connect/disconnect;
- mute;
- gain;
- selected preset;
- stats.

## Milestone 6 — Foreground/background

- foreground service;
- screen off;
- notification actions;
- reconnect.

## Milestone 7 — Discovery/pairing

- mDNS;
- PC list;
- six-digit code;
- remembered device.

## Milestone 8 — Linux GUI

Implementar janela simples via IPC.

## Milestone 9 — Adaptive networking

- jitter;
- loss;
- adaptive buffer;
- drift compensation;
- FEC.

## Milestone 10 — Hardening

- security;
- fuzz;
- soak;
- packaging;
- release.

---

# 78. Definition of Done do MVP

O MVP só é considerado pronto quando:

- [ ] Android captura com Oboe.
- [ ] App exibe meter em tempo real.
- [ ] Opus funciona.
- [ ] UDP funciona.
- [ ] Linux recebe e decodifica.
- [ ] PipeWire source aparece.
- [ ] Discord/OBS consegue selecionar LinMic.
- [ ] Mute Android resulta em silêncio imediato.
- [ ] Unmute é instantâneo.
- [ ] PC mostra stats básicas.
- [ ] App mostra RTT e latência estimada.
- [ ] Queda de rede gera silêncio, não ruído.
- [ ] Reconnect funciona.
- [ ] Fechar GUI Linux não encerra daemon.
- [ ] Tela Android apagada não encerra stream.
- [ ] Daemon não precisa root.
- [ ] Não existe cloud.

---

# 79. Critérios de aceitação de qualidade

## Audio

Fala inteligível e sem clipping por padrão.

## Stability

30 min sem:

- crash;
- memory growth evidente;
- latency creep;
- PipeWire disappearance.

## Mute

Tempo perceptível de mute < 100 ms em LAN boa.

## Reconnect

Wi-Fi off/on deve recuperar sessão automaticamente.

## Resource usage

Daemon não deve permanecer usando CPU alta quando mutado/idle.

---

# 80. Tarefas concretas para a IA começar AGORA

Executar nesta ordem.

### Task 1

Criar workspace Rust:

```text
linmic-common
linmic-protocol
linmic-audio
linmic-pipewire
linmic-network
linmic-ipc
linmicd
linmic
```

### Task 2

Implementar uma source PipeWire que gera seno 440 Hz.

### Task 3

Transformar source em consumidor de SPSC ring.

### Task 4

Adicionar libopus e decoder.

### Task 5

Definir e implementar `LinMicAudioHeaderV1`.

### Task 6

Adicionar receiver UDP.

### Task 7

Criar synthetic sender.

### Task 8

Adicionar stats internas.

### Task 9

Só então criar Android project.

### Task 10

No Android, primeiro criar Oboe capture + meter, sem rede.

### Task 11

Adicionar Opus nativo.

### Task 12

Adicionar packetizer + UDP.

### Task 13

Validar end-to-end.

### Task 14

Adicionar UI detalhada.

---

# 81. APIs internas sugeridas Linux

```rust
pub trait AudioSource {
    fn read(&mut self, output: &mut [i16]) -> usize;
}

pub trait Decoder {
    fn decode(&mut self, packet: Option<&[u8]>, output: &mut [i16])
        -> Result<usize, DecodeError>;
}

pub trait JitterBuffer {
    fn push(&mut self, packet: AudioPacket, arrival: Instant);
    fn pop_due(&mut self, now: Instant) -> JitterResult;
}

pub trait AudioOutput {
    fn start(&mut self) -> Result<(), AudioOutputError>;
    fn stop(&mut self);
}

pub struct StreamStats {
    pub packets_received: AtomicU64,
    pub packets_lost: AtomicU64,
    pub packets_late: AtomicU64,
    pub pipewire_underruns: AtomicU64,
}
```

Não precisa seguir nomes literalmente, mas manter separação de responsabilidades.

---

# 82. JNI Android sugerida

Interface mínima:

```kotlin
external fun nativeCreateEngine(): Long
external fun nativeDestroyEngine(handle: Long)

external fun nativeConfigure(
    handle: Long,
    host: String,
    port: Int,
    bitrate: Int,
    frameMs: Int,
    gainDb: Float
): Int

external fun nativeStart(handle: Long): Int
external fun nativeStop(handle: Long)

external fun nativeSetMuted(handle: Long, muted: Boolean)
external fun nativeSetGain(handle: Long, gainDb: Float)

external fun nativeGetStats(handle: Long): NativeStats
external fun nativeGetWaveform(handle: Long): FloatArray
```

Otimização posterior:

não criar objetos JNI continuamente.

Pode usar buffer direto ou preencher objeto reutilizado.

---

# 83. Native Android classes sugeridas

```text
AudioEngine
OboeInput
PcmRingBuffer
OpusEncoder
AudioPacketizer
UdpSender
AudioMeter
EngineStats
```

`AudioEngine` coordena.

Evitar classe gigante de 3000 linhas.

---

# 84. Kotlin classes sugeridas

```text
MainActivity
StreamingForegroundService

ConnectionRepository
SettingsRepository
DeviceDiscoveryManager
ControlClient
PairingManager
NativeAudioController

MainViewModel
SettingsViewModel

WaveformView
VuMeterView
```

Pode não usar ViewModel no primeiro spike, mas arquitetura final deve separar UI de service/engine.

---

# 85. Linux classes/modules sugeridos

```text
daemon/
  app.rs
  state.rs
  config.rs

network/
  control.rs
  udp.rs
  discovery.rs

audio/
  opus_decoder.rs
  jitter.rs
  drift.rs
  dsp.rs
  meter.rs
  ring.rs

pipewire/
  source.rs

ipc/
  server.rs
  messages.rs

pairing/
  store.rs
  challenge.rs
```

---

# 86. GUI update strategy

Nunca redesenhar com update por sample.

Stats UI:

```text
10 Hz
```

Waveform:

```text
30 Hz máximo
```

Text stats rápidas:

```text
2–5 Hz
```

Isso é suficiente e economiza recursos.

---

# 87. Networking constraints

Não fragmentar UDP.

Manter datagrama confortavelmente abaixo do MTU.

Opus 48 kbps / 10 ms:

payload é pequeno.

Definir hard cap:

```text
<= 1200 bytes
```

por audio datagram se possível.

---

# 88. Packet sequence wrap

`uint32 sequence` vai wrap.

Comparação deve usar aritmética modular.

Não escrever:

```text
if new_seq < old_seq => old
```

de forma ingênua.

Criar helper testado.

---

# 89. Session reset

Ao trocar session:

- flush jitter;
- reset decoder;
- reset drift controller;
- reset sample index expectation;
- preservar PipeWire node;
- stats de lifetime e session separados.

---

# 90. Fade

Para evitar click:

Ao:

- conectar;
- reconectar;
- unmute;

aplicar fade curto:

```text
5–10 ms
```

Ao mute:

fade out opcional 3–5 ms.

Se mute precisa ser instantâneo, priorizar resposta.

---

# 91. Silence handling

Quando receiver não tem áudio:

PipeWire recebe zeros.

Nunca deixar buffer não inicializado.

Quando muted:

zeros.

Quando decode error:

PLC ou zero.

---

# 92. User-visible latency explanation

Tooltip:

```text
Latência estimada combina tamanho dos frames,
tempo de rede, jitter buffer e fila de áudio no PC.
Não é uma medição acústica exata.
```

---

# 93. Advanced diagnostics Android

Tela escondida/avançada:

```text
Actual sample rate
Frames per burst
Sharing mode
Performance mode
Input preset
Device id
AAudio/OpenSL backend
XRun count
Capture callback p50/p95
Encoder p50/p95
UDP send p50/p95
```

Muito útil para comparar aparelhos.

---

# 94. PipeWire diagnostics

CLI:

```bash
linmic pipewire-info
```

Retorna:

```text
PipeWire connected: yes
Node id: ...
Node name: linmic_input
Rate: 48000
Quantum: ...
Format: S16LE
Underruns: ...
```

---

# 95. Default user experience

O usuário comum não deve escolher 20 parâmetros.

Default:

```text
Auto-discovery: on
Auto reconnect: on
Profile: Balanced
Gain: 0 dB
Noise suppression: system/off based on support
Adaptive jitter: on
Visualizer: on
Advanced stats: off
```

Abrir app → selecionar PC → conectar.

---

# 96. Developer toggles

Debug builds podem mostrar:

```text
Force 5 ms frame
Force 20 ms frame
Inject packet loss
Inject jitter
Disable PLC
Disable drift correction
Dump packet headers
Synthetic mic
```

Não expor tudo em release normal.

---

# 97. Não implementar cedo demais

Postergar:

- Windows;
- iOS;
- macOS;
- audio playback PC→phone;
- multi-phone mixer;
- remote internet relay;
- recording;
- VST;
- AI voice enhancement;
- Bluetooth custom transport;
- kernel modules.

Primeiro tornar Android→Linux impecável.

---

# 98. Possíveis features pós-1.0

- stereo mode;
- 96 kHz mode, se houver caso real;
- RNNoise;
- WebRTC audio processing;
- EQ;
- compressor configurável;
- push-to-talk;
- Steam Deck mode;
- QR-code pairing;
- multiple PCs;
- audio monitor on phone;
- direct USB transport;
- LAN encryption improvements;
- Flatpak GUI;
- AppImage GUI;
- DEB/RPM;
- KDE native integration.

---

# 99. README inicial sugerido

```text
# LinMic

LinMic turns an Android phone into a low-latency microphone
for PipeWire-based Linux desktops.

Primary target:
- CachyOS / Arch Linux
- Android

Principles:
- lightweight
- native
- open source
- no cloud
- low latency
- PipeWire first

Project status: early development.
```

---

# 100. Regra final para o coding agent

A prioridade do projeto é:

```text
CORREÇÃO DE ÁUDIO
>
ESTABILIDADE
>
LATÊNCIA
>
BAIXO CONSUMO
>
USABILIDADE
>
FEATURES
>
ESTÉTICA
```

Uma UI perfeita não compensa áudio instável.

O primeiro checkpoint real é ouvir a própria voz do Android entrando no dispositivo PipeWire do CachyOS com estabilidade.

Depois disso, evoluir incrementalmente e medir cada mudança.

---

# 101. Checklist de início imediato

Antes de escrever features, a IA deve verificar no CachyOS:

```bash
pipewire --version
wpctl status
rustc --version
cargo --version
cmake --version
clang --version
```

Criar branch:

```text
feat/mvp-audio-path
```

E produzir primeiro:

```text
1. daemon compila
2. source PipeWire existe
3. tom sintético funciona
4. ring buffer funciona
5. UDP local funciona
6. Opus local funciona
7. Android captura
8. Android envia
9. voz chega ao PipeWire
```

Somente após 9:

```text
10. descoberta
11. pairing
12. visual refinado
13. packaging
```

Essa ordem deve ser mantida salvo bloqueio técnico documentado.
