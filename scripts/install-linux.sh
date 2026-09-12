#!/bin/sh
set -eu
base=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
bin="$HOME/.local/bin"
mkdir -p "$bin" "$HOME/.local/share/applications" "$HOME/.local/share/icons/hicolor/scalable/apps" "$HOME/.config/systemd/user"
if [ -d "$base/bin" ]; then source_dir="$base/bin"; else source_dir="$base/target/release"; fi
for name in linmic linmicd linmic-gui linmic-hotkeys; do
    test -x "$source_dir/$name" || { printf 'Missing binary: %s. Build with cargo build --release --workspace\n' "$name" >&2; exit 1; }
    install -m755 "$source_dir/$name" "$bin/$name.new"
    mv "$bin/$name.new" "$bin/$name"
done
sed "s|Exec=linmic-gui|Exec=$bin/linmic-gui|" "$base/linux/linmic.desktop" > "$HOME/.local/share/applications/org.linmic.LinMic.desktop"
install -m644 "$base/linux/linmic.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/org.linmic.LinMic.svg"
sed 's|ExecStart=/usr/bin/linmicd|ExecStart=%h/.local/bin/linmicd|' "$base/linux/systemd/linmic.service" > "$HOME/.config/systemd/user/linmic.service"
systemctl --user daemon-reload
systemctl --user enable linmic.service
systemctl --user restart linmic.service
printf 'LinMic installed. Open LinMic from the application menu, or run %s/linmic-gui\n' "$bin"
