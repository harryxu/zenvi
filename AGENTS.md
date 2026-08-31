# AGENTS.md - Developer & Agent Guide for Zenvi

> **Project:** Zenvi (GPU-accelerated Standalone Neovim GUI Shell)  
> **Tech Stack:** Rust (`1.98.0+`), GPUI (`gpui 0.2.2`), Neovim RPC (`nvim --embed`), Tokio, MessagePack (`rmpv`)

---

## 1. Project Overview & Mission

**Zenvi** is a modern, lightweight, standalone desktop GUI frontend for **Neovim**, built entirely in **Rust** using **GPUI** (the GPU-accelerated UI framework developed by Zed Industries).

### Core Philosophy
* **Pure Neovim Underneath:** Zenvi does *not* emulate Vim. It spawns and embeds real `nvim --embed` via MessagePack-RPC over `stdio`. User configs (`init.lua`), plugins (Lazy.nvim, Treesitter, LSP), and keymaps run 100% natively.
* **Native Desktop Capabilities:** Provides features traditional terminal emulators struggle with:
  * External file and folder drag-and-drop (`on_drop` -> `:cd` / `:edit`)
  * Native macOS menu integration (`File -> Open File / Open Folder`, `Cmd+Q`, `Cmd+O`)
  * Smooth GPU rendering (Metal on macOS, Vulkan on Linux/Windows)
  * Dedicated titlebar with macOS traffic light clearance
  * Full mouse & trackpad gesture support (click, double/triple click, drag selection, scroll wheel)

---

## 2. Directory Structure & Architecture

```
/Users/harry/dev/zenvi/
├── Cargo.toml               # Dependencies (gpui, tokio, rmpv, unicode-width, resvg, usvg)
├── README.md                # End-user introduction and usage guide
├── AGENTS.md                # This developer/agent reference document
├── zenvi-icon.svg           # Master vector icon source
├── assets/                  # Application asset directory
│   ├── icon.svg             # Standard icon SVG asset
│   ├── icon_1024x1024.png   # Master high-res raster icon
│   ├── AppIcon.icns         # Compiled macOS multi-resolution icon bundle
│   ├── AppIcon.iconset/     # macOS iconset (16x16 to 512x512@2x)
│   └── Info.plist           # macOS bundle metadata
├── scripts/
│   └── bundle_macos.sh      # macOS Zenvi.app standalone packaging script
└── src/
    ├── main.rs              # App entry point, GPUI Application lifecycle & Tokio runtime initialization
    ├── actions.rs           # Centralized GPUI Action definitions (Quit, OpenFile, OpenFolder, OpenConfig, etc.)
    ├── keymap.rs            # Keyboard shortcuts mapping and registration
    ├── menu.rs              # macOS Application menus hierarchy and action bindings
    ├── window.rs            # Window lifecycle management, cascade offset calculation & config path resolution
    ├── input.rs             # Keyboard event translation (GPUI KeyDownEvent -> Neovim key notation)
    ├── bin/
    │   └── generate_icon.rs # Asset & icon rasterizer / icns compiler tool
    ├── nvim/
    │   ├── mod.rs           # Submodule definitions
    │   ├── process.rs       # Background nvim child process, stdio pipes, NvimSession API, NvimEvent channel
    │   ├── protocol.rs      # MessagePack-RPC encoder & parser (Request, Response, Notification)
    │   ├── events.rs        # Neovim `redraw` event dispatcher (grid_line, hl_attr_define, cursor_goto, etc.)
    │   └── state.rs         # In-memory screen grid buffers, highlight lookup table, cursor & mode state
    └── ui/
        ├── mod.rs           # ZenviView (Root view, custom macOS titlebar, status bar, mouse events, drag-and-drop)
        ├── font.rs          # Neovim guifont & linespace parser and font metrics calculator
        └── grid.rs          # High-performance grid text & cell span renderer
```

---

## 3. Key Components & Mechanics

### A. Neovim RPC & Redraw Protocol (`src/nvim/`)
* **Spawning:** `NvimSession::spawn(event_tx)` launches `nvim --embed` with `Stdio::piped()`.
* **Attachment:** Calls `nvim_ui_attach(width, height, {"ext_linegrid": true, "rgb": true})`.
* **Redraw Pipeline:**
  1. Neovim sends `redraw` notifications containing micro-events (`grid_line`, `grid_resize`, `grid_scroll`, `hl_attr_define`, `default_colors_set`, `mode_change`, `flush`).
  2. `events::handle_redraw_event` updates `Arc<RwLock<NvimState>>`.
  3. `NvimEvent::Redraw` is emitted through `tokio::sync::mpsc`.
  4. GPUI async task receives `NvimEvent::Redraw` and triggers `cx.notify()`.
* **Exit Lifecycle:** When `stdout` reaches EOF (user typed `:q` / `:qa` or nvim closed), `NvimEvent::Exit` is sent, triggering `cx.quit()`.

### B. View & Grid Rendering (`src/ui/`)
* **Span Batching (`src/ui/grid.rs`):** Rather than rendering 80+ separate DOM elements per line, adjacent cells sharing identical highlight attributes (foreground, background, bold, italic, underline) are merged into single `CellSpan` elements for high GPU throughput.
* **Layout Geometry:**
  * Top titlebar: `32px` with `pl(px(78.0))` for macOS traffic light buttons.
  * Grid area: Fills all remaining window height down to the bottom. Automatically recalculates `(cols, rows)` on window resize and notifies Neovim via `nvim_ui_try_resize`.

### C. Input & Mouse Management (`src/input.rs`, `src/ui/mod.rs`)
* **Keyboard (`src/input.rs`):** Converts GPUI `KeyDownEvent` into Neovim-compatible strings (e.g. `<CR>`, `<Esc>`, `<C-w>`, `<M-x>`, `<D-s>`, `<lt>`). System-level shortcuts (`Cmd+Q`, `Cmd+O`, `Cmd+Shift+O`) are bypassed to let GPUI native actions handle them.
* **Mouse (`src/ui/mod.rs`):** Converts pixel coordinates `Point<Pixels>` into Neovim `(col, row)` and calls `nvim_input_mouse`:
  * Left/Right/Middle press, release, and drag.
  * Trackpad / mouse wheel scrolling with fractional accumulator.

---

## 4. Development & Build Commands

All commands should be run from `/Users/harry/dev/zenvi`:

```bash
# Check compilation without linking
cargo check

# Run debug build
cargo run

# Build release binary
cargo build --release

# Inspect generated binary
ls -lh target/debug/zenvi
```

---

## 5. Guidelines for Future AI Agents & Contributors

When extending Zenvi, follow these best practices:

 - **Do Not Block the UI Thread:** All RPC I/O and process management must remain on Tokio background tasks. Only pass lightweight notify signals (`NvimEvent`) to GPUI views.
 - **Preserve LineGrid Alignment:** Monospace alignment is critical. When modifying `grid.rs` or font metrics, ensure `char_width` and `line_height` calculations stay synchronized with `pos_to_grid` and `try_resize`.
 - **Handle macOS Window Titlebar Gracefully:** When adjusting top padding or adding toolbar elements, never remove the `78px` left margin required by macOS traffic light buttons unless custom frame rendering is explicitly configured.
 - **Prefer Direct Neovim RPC Commands:** When implementing UI actions (e.g., file open, buffer close, cd), invoke `session.send_command(...)` or `session.send_input(...)`.
 - Do not make git commits on your own without being asked.
