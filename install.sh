#!/bin/bash
set -e

# Surch2 One-Line Installer
REPO="rishuishind/surch2"
APP_NAME="surch2"
INSTALL_DIR="$HOME/.local/bin"
DESKTOP_ENTRY_DIR="$HOME/.local/share/applications"
AUTOSTART_DIR="$HOME/.config/autostart"

echo "====================================="
echo "   🚀 Installing Surch2 Launcher   "
echo "====================================="

# 1. Fetch the latest release API
echo "[1/4] Fetching latest release info..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest")
APPIMAGE_URL=$(echo "$LATEST_RELEASE" | grep "browser_download_url.*AppImage\"" | cut -d '"' -f 4 | head -n 1)

if [ -z "$APPIMAGE_URL" ]; then
    echo "❌ Error: Could not find an AppImage in the latest release."
    echo "Ensure the GitHub Actions build has successfully published the binary."
    exit 1
fi

echo "Found latest AppImage: $APPIMAGE_URL"

# 2. Download AppImage
echo "[2/4] Downloading $APP_NAME..."
mkdir -p "$INSTALL_DIR"
curl -L -o "$INSTALL_DIR/$APP_NAME" "$APPIMAGE_URL"
chmod +x "$INSTALL_DIR/$APP_NAME"
echo "✅ Installed binary to $INSTALL_DIR/$APP_NAME"

# 3. Create Desktop Entry
echo "[3/4] Creating application menu shortcut..."
mkdir -p "$DESKTOP_ENTRY_DIR"

cat <<EOF > "$DESKTOP_ENTRY_DIR/$APP_NAME.desktop"
[Desktop Entry]
Name=Surch2
Comment=Raycast alternative for Linux (i3 compatible)
Exec=$INSTALL_DIR/$APP_NAME
Icon=utilities-terminal
Terminal=false
Type=Application
Categories=Utility;System;
EOF
echo "✅ Created shortcut at $DESKTOP_ENTRY_DIR/$APP_NAME.desktop"

# 4. Create Autostart Entry
echo "[4/4] Setting up background autostart..."
mkdir -p "$AUTOSTART_DIR"

cat <<EOF > "$AUTOSTART_DIR/$APP_NAME.desktop"
[Desktop Entry]
Type=Application
Exec=$INSTALL_DIR/$APP_NAME
Hidden=false
NoDisplay=false
X-GNOME-Autostart-enabled=true
Name=Surch2
Comment=Launch Surch2 in background for global hotkey support
EOF
# 5. Fix GNOME Alt+Space conflict
if command -v gsettings &> /dev/null; then
    echo "[5/5] Releasing Alt+Space shortcut from GNOME (if applicable)..."
    gsettings set org.gnome.desktop.wm.keybindings activate-window-menu "['']" || true
fi

echo "====================================="
echo " 🎉 Installation Complete! "
echo "====================================="
echo "Important: Make sure $INSTALL_DIR is in your system's PATH."
echo ""
echo "To start Surch2 right now, run:"
echo "  $INSTALL_DIR/$APP_NAME &"
echo ""
echo "Or find it in your application menu. Once running in the background, press Alt+Space to summon!"
