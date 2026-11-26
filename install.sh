#!/usr/bin/env bash
# Build and install the applet into ~/.local (or $PREFIX). No sudo needed.
# Usage: ./install.sh            build + install
#        ./install.sh --uninstall
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
prefix="${PREFIX:-$HOME/.local}"
name="cosmic-applet-batteries"
appid="dev.darthsalad.CosmicAppletBatteries"

bin="$prefix/bin/$name"
desktop="$prefix/share/applications/$appid.desktop"

if [[ "${1:-}" == "--uninstall" ]]; then
    rm -f "$bin" "$desktop"
    pkill -x "$name" 2>/dev/null || true
    echo "Removed. Also remove it from Settings > Desktop > Panel > Applets."
    exit 0
fi

cargo build --release --manifest-path "$repo/Cargo.toml"
install -Dm0755 "$repo/target/release/$name" "$bin"
# cosmic-panel's PATH lacks ~/.local/bin, so point Exec at the absolute path.
install -Dm0644 "$repo/resources/app.desktop" "$desktop"
sed -i "s|^Exec=.*|Exec=$bin|" "$desktop"

echo "Installed to $prefix."
echo "Add it via Settings > Desktop > Panel > Applets > Device Batteries."
