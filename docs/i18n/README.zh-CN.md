# LinMic

[Português (Brasil)](../../README.md) · [English](README.en.md) · [Español](README.es.md) · [Русский](README.ru.md) · **简体中文** · [日本語](README.ja.md)

将 Android 手机用作 Linux 麦克风。原生应用，无需账号，无需云端。**0.3.0 (Android) / 0.3.0 (Linux) · Android 8 及以上 · 使用 PipeWire 的 Linux**，主要面向 CachyOS/Arch。

## 安装与连接

1. 下载[最新版本](https://github.com/goszc/LinMic/releases/latest)。
2. 解压 Linux 压缩包，以普通用户运行 `./scripts/install-linux.sh`，不要使用 sudo。在应用菜单中打开 **LinMic**。
3. 安装 Android APK。将手机和电脑连接到同一私有网络，或启用 USB 网络共享。
4. 在桌面端点击**配对手机**。在 Android 上选择发现的电脑或输入 IP，允许麦克风权限，并输入电脑显示的**六位配对码**。无需手动复制证书指纹。
5. 在 Discord、OBS 或其他应用中选择 **LinMic** 作为输入设备。

配对码两分钟内有效，仅可使用一次，最多允许五次尝试。后续连接自动验证身份。安装 release APK 前，需要先卸载一次最初的 0.1.0 调试 APK，因为签名密钥已更换。后续版本将保留 release 签名身份。

## 音频与界面

48 kHz 单声道 Opus；Ultra（5 ms）、Balanced（10 ms）、Stable（20 ms）。增益最高 +24 dB，支持平滑调整与 -1 dBFS 限幅。手机输入增益与桌面接收增益彼此独立。建议从 0 dB 开始：数字增益也会放大已有噪声。

输入名称结合 Android 提供的设备类型和产品名称，并随设备插拔更新。Android 不一定会单独显示每个物理麦克风。蓝牙可能增加明显延迟。界面显示的是估计延迟，并非声学测量结果。

包含静音、自动重连、波形、音量表、网络统计、托盘和可选的桌面门户全局快捷键。关闭界面不会停止后台服务。两个应用均支持葡萄牙语、英语、西班牙语、俄语、简体中文和日语。Android 语言位于“设置 → 界面”；桌面端更改语言后需重新启动界面。

## 编译

```sh
sudo pacman -S --needed rust cmake ninja clang pkgconf pipewire opus gtk4
cargo build --locked --release --workspace
cargo test --workspace
```

Android 需要 JDK 17、SDK 36、NDK 28.2.13676358、CMake 3.22.1、Python，以及 Rust 目标 `aarch64-linux-android`、`armv7-linux-androideabi`、`x86_64-linux-android`。在 `android/` 目录运行 `./gradlew assembleDebug testDebugUnitTest lintDebug`。

本地签名：`python3 scripts/sign-android.py`。请私密备份 `~/.local/share/linmic-signing`，切勿提交或上传。局域网端口：TCP 39820、UDP 39821、mDNS UDP 5353。

[安全政策](../../SECURITY.md) · [安全设计](../security.md) · [验证记录](../validation.md) · [参与贡献](../../CONTRIBUTING.md)。功能测试不等同于独立安全审计；设备和发行版覆盖仍有限。

**GPL-3.0-or-later**。参阅[许可证](../../LICENSE)及[第三方声明](../../THIRD_PARTY_NOTICES.md)。分发二进制文件时，须同时提供对应源代码和许可证声明。

桌面版 0.3.0 新增麦克风、连接和诊断选项卡、增益预设，以及带独立音量的**监听麦克风**。请使用耳机。监听使用电脑默认输出，默认关闭，并在断开连接或退出界面时停止。需要 PipeWire（`pw-loopback`、`pw-dump`、`pw-link`）和 WirePlumber（`wpctl`）。
