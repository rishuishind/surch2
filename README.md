# Surch2 — The Lightning-Fast Raycast Alternative for Linux

Surch2 is a blazing-fast, keyboard-driven productivity launcher for Linux. It is built natively for Linux window managers (like i3, Sway, and GNOME) using Rust, Tauri v2, and SolidJS. Surch2 stays out of your way until you need it, and executes your commands instantly.

![Surch2 Demo](https://via.placeholder.com/800x400?text=Surch2+Screenshot)

## Features 🚀

- **Universal App Launcher:** Fuzzy searches all installed `.desktop`, Snap, and Flatpak applications.
- **Clipboard History:** Everything you copy is indexed into a local, private SQLite database. Hit `Enter` to instantly paste previous copies.
- **Snippets & Text Expansion:** Create reusable text blocks (like email signatures or links) that expand into your clipboard.
- **System Controls:** Command your PC (Sleep, Shutdown, Restart, Volume Up/Down/Mute, Lock Screen) without touching a mouse.
- **i3 Window Manager IPC:** Talk natively to i3! Instantly search and pull focus to any active window, or jump to specific workspaces via keyboard.
- **Inline Calculator:** Type math directly into the search bar (e.g., `sqrt(144) * pi`) and get instant answers.
- **Always-on-top, Borderless UI:** Designed specifically for tiling environments to look beautiful and unobtrusive.

## ⚡ 1-Click Installation (Recommended)

Surch2 can be installed via a single line in your terminal. This script automatically:
1. Downloads the latest `.AppImage` build from GitHub Releases.
2. Places the executable in `~/.local/bin`.
3. Creates a `.desktop` shortcut so it appears in standard app menus.
4. Adds a background autostart entry so the `Alt+Space` hotkey works globally immediately after booting.

```bash
curl -sSL https://raw.githubusercontent.com/rishuishind/surch2/main/install.sh | bash
```

*Note: Surch2 is designed to run silently in the background. The `Alt+Space` hotkey will summon and dismiss the window instantly.*

### Dependencies

Because Surch2 uses Tauri, it relies on your system's built-in `WebKit2GTK` to run the frontend at near-zero memory cost. E.g on Debian/Ubuntu:
```bash
sudo apt install libwebkit2gtk-4.1-0
```

## 🛠️ Building from Source

If you prefer to compile the application yourself (requires Rust & Node.js 20+):

```bash
# Clone the repo
git clone https://github.com/rishuishind/surch2.git
cd surch2

# Install frontend dependencies
npm install

# Start development server
npm run tauri dev

# Build production binary
npm run tauri build
```

## ⌨️ Architecture

- **Backend:** Rust (`tauri`, `rusqlite`, `i3ipc`, `arboard`)
- **Frontend:** SolidJS, TypeScript, Vanilla CSS
- **Data Store:** SQLite (stored at `~/.local/share/surch2/data.db`)

Surch2 does **not** rely on Chromium/Electron, keeping the resting RAM footprint ~30MB!
