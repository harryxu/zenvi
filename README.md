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

### Other options

#### `vim.opt.zenvi_prewarm_max_lines`

Pre-warms all off-screen lines into Zenvi's 64-bit FNV-1a content_cache, once per buffer when opening files <= zenvi_prewarm_max_lines (default 1000), set `0` to disable pre-warming.

