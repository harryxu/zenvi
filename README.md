# Zenvi ⚡️

[![Rust](https://img.shields.io/badge/rust-1.98.0%2B-orange.svg)](https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/)

A modern, standalone, GPU-accelerated **Neovim GUI frontend** built in Rust with [GPUI](https://gpui.rs/) (the UI framework powering the Zed editor).

---

## ✨ Features

- **🚀 Embedded Neovim**: Runs true Neovim (`nvim --embed`) in the background via MessagePack-RPC. All your existing `init.lua`, Lua plugins, Treesitter, and LSP configurations work out of the box.
- **⚡️ GPU Accelerated**: Native rendering powered by GPUI with Metal / Vulkan.
- **🔄 Neovim Hot Reload**: Reload the embedded Neovim session on the fly via macOS menu (`Zenvi -> Reload Neovim` / `File -> Reload Neovim`) or shortcut (`Cmd+Shift+R`) whenever your `init.lua` changes.
- **📂 External Drag & Drop**: Drag any file or folder from Finder / file manager into the Zenvi window to immediately open and edit it (`:edit <path>`).
- **⌨️ Rich Keyboard Mapping**: Full support for Neovim key sequences (`<Esc>`, `<CR>`, `<C-w>`, `<M-x>`, `<D-s>`, arrow keys, function keys, and more).
- **📐 Dynamic Resizing**: Automatically computes optimal cols/rows based on window dimensions and updates Neovim grid size in real time.
- **🔤 Native `guifont` Support**: Configure font family and font size natively in `init.lua` via `vim.opt.guifont = "JetBrainsMono Nerd Font:h15"`. Zenvi also injects `vim.g.zenvi = true` and `vim.g.gui_running = 1` before `init.lua` loads.

---

## ⚙️ Configuration (in `init.lua`)

Zenvi sets `vim.g.zenvi = true` and `vim.g.gui_running = 1` upon startup. You can configure your GUI font and line spacing directly in your Neovim configuration (`~/.config/nvim/init.lua`):

```lua
if vim.g.zenvi then
  -- Set font family and font size
  vim.opt.guifont = "JetBrainsMono Nerd Font:h15"
  
  -- Optional: add extra pixel line spacing
  vim.opt.linespace = 2
end
```

You can also change fonts dynamically at runtime inside Neovim:
```vim
:set guifont=Fira_Code:h16
```

---

## 🛠️ Requirements

- **Rust**: 1.98.0+ (`cargo`, `rustc`)
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

### 📦 Packaging macOS Desktop App (`Zenvi.app`)

Zenvi includes an automated packaging pipeline with high-resolution Retina icons (`AppIcon.icns`):

```bash
# Package Zenvi.app into target/Zenvi.app
bash scripts/bundle_macos.sh

# Open the packaged standalone app
open target/Zenvi.app
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
    ├── font.rs      # Neovim guifont & linespace parser
    └── grid.rs      # High-performance grid text & cell span renderer
```
