<div align="center">

  # Zenvi 

  <img src="assets/zenvi-icon.svg" width="96" alt="Zenvi icon" />

  **A Neovim GUI frontend built with [GPUI](https://gpui.rs/).**

</div>

---

## Features

- **Embedded Neovim** — Runs `nvim --embed` via MessagePack-RPC. Your `init.lua`, plugins, Treesitter, and LSP configs work as-is.
- **GPU Rendering** — Uses GPUI with Metal (macOS) / Vulkan.
- **Theme Synchronization** — Titlebar, borders, and menus dynamically derive colors from Neovim's active colorscheme.
- **CLI Integration** — Install `zenvi` command via menu (`Zenvi -> Install Shell Command`), then open files or directories from terminal (`zenvi .`, `zenvi file.rs`).
- **Neovim Hot Reload** — Reload Neovim session via `Zenvi -> Reload Neovim` or `Cmd+Shift+R`. Supports [auto-session](https://github.com/rmagatti/auto-session) state save/restore.
- **`guifont` Support** — Set font via `vim.opt.guifont` in `init.lua`. Injects `vim.g.zenvi = true` and `vim.g.gui_running = 1` on startup.

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
