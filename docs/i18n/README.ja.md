# LinMic

[Português (Brasil)](../../README.md) · [English](README.en.md) · [Español](README.es.md) · [Русский](README.ru.md) · [简体中文](README.zh-CN.md) · **日本語**

Android スマートフォンを Linux のマイクとして使用できます。ネイティブアプリ、アカウント不要、クラウド不要。**0.3.0 (Android) / 0.2.0 (Linux)・Android 8 以降・PipeWire を使用する Linux**。主な対象は CachyOS/Arch です。

## インストールと接続

1. [最新リリース](https://github.com/goszc/LinMic/releases/latest)をダウンロードします。
2. Linux アーカイブを展開し、一般ユーザーで `./scripts/install-linux.sh` を実行します。sudo は不要です。アプリメニューから **LinMic** を開きます。
3. Android APK をインストールします。両方のデバイスを同じプライベートネットワークに接続するか、USB テザリングを有効にします。
4. デスクトップで**スマートフォンをペアリング**を押します。Android で検出された PC を選ぶか IP を入力し、マイクへのアクセスを許可して、PC に表示される **6 桁のコード**を入力します。証明書の指紋を手動でコピーする必要はありません。
5. Discord、OBS などの入力デバイスに **LinMic** を選択します。

コードは 2 分間有効で、1 回限り、最大 5 回まで試行できます。以降の接続は自動認証されます。release APK の署名が異なるため、初期の 0.1.0 デバッグ APK は一度アンインストールしてください。今後のリリースでは同じ release 署名を維持します。

## 音声とインターフェース

48 kHz モノラル Opus。Ultra（5 ms）、Balanced（10 ms）、Stable（20 ms）。最大 +24 dB のゲイン、滑らかな変更、-1 dBFS リミッターを搭載しています。スマートフォンの入力ゲインと PC の受信ゲインは独立しています。まず 0 dB から調整してください。デジタルゲインは既存のノイズも増幅します。

入力名は Android のデバイス種別と製品名を組み合わせ、接続・取り外しに応じて更新されます。Android が個別のマイクカプセルを公開するとは限りません。Bluetooth は遅延を大きくする場合があります。表示される遅延は推定値であり、音響測定値ではありません。

ミュート、自動再接続、波形、レベルメーター、統計、トレイ、デスクトップポータルによる任意のグローバルショートカットを備えています。画面を閉じてもサービスは動作します。両アプリはポルトガル語、英語、スペイン語、ロシア語、簡体字中国語、日本語に対応します。Android は「設定 → インターフェース」で言語を選択できます。デスクトップの言語変更後はインターフェースを再起動してください。

## ビルド

```sh
sudo pacman -S --needed rust cmake ninja clang pkgconf pipewire opus gtk4
cargo build --locked --release --workspace
cargo test --workspace
```

Android には JDK 17、SDK 36、NDK 28.2.13676358、CMake 3.22.1、Python、および Rust ターゲット `aarch64-linux-android`、`armv7-linux-androideabi`、`x86_64-linux-android` が必要です。`android/` で `./gradlew assembleDebug testDebugUnitTest lintDebug` を実行します。

ローカル署名：`python3 scripts/sign-android.py`。`~/.local/share/linmic-signing` は安全にバックアップし、公開しないでください。LAN ポート：TCP 39820、UDP 39821、mDNS UDP 5353。

[セキュリティ方針](../../SECURITY.md)・[設計](../security.md)・[検証](../validation.md)・[貢献](../../CONTRIBUTING.md)。機能テストは独立したセキュリティ監査ではなく、検証したデバイスとディストリビューションには限りがあります。

**GPL-3.0-or-later**。[ライセンス](../../LICENSE)と[第三者の通知](../../THIRD_PARTY_NOTICES.md)を参照してください。バイナリの配布時には、対応するソースコードと通知も提供してください。
