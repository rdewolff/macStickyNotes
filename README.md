# macStickyNotes

A lightweight sticky notes app for macOS with markdown support, window anchoring, and smart snapping.

> Forked from [md-sticky](https://github.com/andrewyur/md-sticky) by [Andrew Yurovchak](https://github.com/andrewyur). Built on top of his original work with additional features like window anchoring, note pinning, automatic backups, and more.

## What it does

macStickyNotes lets you create small, frameless sticky notes that float on your desktop. Notes support markdown syntax (powered by Quill), auto-save their content and position, and can anchor themselves to other application windows so they follow them around as you rearrange your workspace.

## Features

### Markdown editor
Type using standard markdown syntax and it converts to rich text automatically. Supports bold, italic, lists, checkboxes (`[ ]`), and more. The toolbar appears only when you select text, keeping the interface minimal.

### Window anchoring
Click the anchor button on any note to attach it to the nearest application window on your screen. The note will track that window's position and move with it. If the target window is closed, the anchor releases automatically. A small badge shows the name of the app you're anchored to.

### Pin notes (always on top)
Toggle the pin button to keep a note floating above all other windows. The pin state is saved and restored when you relaunch the app.

### Smart snapping
Snap notes to each other or to screen edges using keyboard shortcuts. Full snap aligns with overlapping window edges, partial snap aligns with any nearby edge. Notes maintain a 20px gap when snapped.

### Color palette
7 preset colors available from the palette button or via `Cmd+1` through `Cmd+7`:

| Key | Color |
|-----|-------|
| `Cmd+1` | Yellow |
| `Cmd+2` | Blue |
| `Cmd+3` | Green |
| `Cmd+4` | Teal |
| `Cmd+5` | Light green |
| `Cmd+6` | Pink |
| `Cmd+7` | Purple |

### Auto-save & backups
Notes automatically save their content, position, size, color, and pin state. On each app launch, a backup is created in the app data directory. Backups older than 30 days are cleaned up automatically.

### Additional features
- **Resize to fit** - Automatically resize a note to match its content (`Cmd+F`)
- **Cycle focus** - Navigate between notes with `Cmd+/` and `Cmd+Alt+/`
- **Bring all to front** - Option to bring all notes forward when the app is focused
- **Launch on startup** - Configurable via the app menu
- **Auto-updates** - Install once, get updates automatically

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+N` | New note |
| `Cmd+W` | Close note |
| `Cmd+F` | Resize note to fit text |
| `Cmd+/` | Focus next note |
| `Cmd+Alt+/` | Focus previous note |
| `Cmd+1` - `Cmd+7` | Set note color |
| `Cmd+Alt+Arrow` | Snap note in direction |
| `Cmd+Alt+Shift+Arrow` | Partial snap in direction |

Standard editor shortcuts (`Cmd+C`, `Cmd+V`, `Cmd+X`, `Cmd+Z`) work as expected.

## Installation

### macOS

Download the `.dmg` from the [releases page](https://github.com/rdewolff/macStickyNotes/releases). The installer is not signed/notarized, so you may need to run:

```bash
xattr -d com.apple.quarantine /path/to/macStickyNotes.dmg
```

### Build from source

Requires [Rust](https://www.rust-lang.org/tools/install) and [Node.js](https://nodejs.org/).

```bash
npm install
npm run tauri build
```

## Tech stack

- **Frontend**: Svelte 5 + Quill editor
- **Backend**: Tauri 2 (Rust)
- **Platform APIs**: Core Graphics, AppKit (via objc2)
