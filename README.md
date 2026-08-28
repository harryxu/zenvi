# Zenvi ⚡️

A modern, standalone, GPU-accelerated **Neovim GUI frontend** built in Rust with [GPUI](https://gpui.rs/) (the UI framework powering the Zed editor).

---

## ✨ Features

- **🚀 Embedded Neovim**: Runs true Neovim (`nvim --embed`) in the background via MessagePack-RPC. All your existing `init.lua`, Lua plugins, Treesitter, and LSP configurations work out of the box.
- **⚡️ GPU Accelerated**: Native rendering powered by GPUI with Metal / Vulkan.
- **📂 External Drag & Drop**: Drag any file or folder from Finder / file manager into the Zenvi window to immediately open and edit it (`:edit <path>`).
- **⌨️ Rich Keyboard Mapping**: Full support for Neovim key sequences (`<Esc>`, `<CR>`, `<C-w>`, `<M-x>`, `<D-s>`, arrow keys, function keys, and more).
- **📐 Dynamic Resizing**: Automatically computes optimal cols/rows based on window dimensions and updates Neovim grid size in real time.
- **🎨 Native Status Bar**: Displays current mode (NORMAL, INSERT, VISUAL, etc.) and cursor position.

---

## 🛠️ Requirements

- **Rust**: 1.80+ (`cargo`, `rustc`)
- **Neovim**: `nvim` installed and available in `$PATH`
- **macOS**: Xcode Metal Toolchain (if building on macOS)

---

## 🏃 Running Zenvi

```bash
cd /Users/harry/dev/zenvi
cargo run
```

To build a release build:
```bash
cargo build --release
./target/release/zenvi
```

---

## 🏗️ Project Architecture

```
src/
├── main.rs          # Application entry point & GPUI window management
├── input.rs         # GPUI Keystroke -> Neovim key notation converter
├── nvim/
│   ├── mod.rs       # Module exports
│   ├── process.rs   # nvim --embed process lifecycle & stdio RPC channels
│   ├── protocol.rs  # Msgpack-RPC encoder/decoder
│   ├── events.rs    # Neovim 'redraw' protocol dispatcher
│   └── state.rs     # Screen grid buffer, highlights & mode state
└── ui/
    ├── mod.rs       # ZenviView root view, status bar & drag-and-drop handler
    └── grid.rs      # High-performance grid text & cell span renderer
```
