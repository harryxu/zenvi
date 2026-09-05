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
- **Collapsible Left Panel** — Dedicated titlebar toggle button to open and close Neovim's left panel (defaults to [neo-tree.nvim](https://github.com/nvim-neo-tree/neo-tree.nvim) or your own custom panel function), with bidirectional state synchronization.
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

### 🗂️ Left Panel Toggle (`toggle_left_panel`)

Zenvi provides a left panel toggle button on the right side of the titlebar. The button dynamically switches icons (`panel-left` vs `panel-left-open`) based on whether the panel is currently open.

#### Default Behavior
If no custom function is specified, Zenvi defaults to toggling **[neo-tree.nvim](https://github.com/nvim-neo-tree/neo-tree.nvim)** (`neo-tree.command.execute({ toggle = true, position = "left" })`). If `neo-tree` is not installed, clicking the button has no effect.

#### Built-in API & Commands
- **Lua function**: `zenvi.toggle_left_panel()`
- **Ex command**: `:ZenviToggleLeftPanel`
- **State query**: `zenvi.is_left_panel_open()`

#### Custom Panel Function
You can customize the toggle function and state detection in your `init.lua`:

```lua
if vim.g.zenvi then
  -- Define custom toggle logic (Lua function or Ex command string)
  vim.g.zenvi_toggle_left_panel = function()
    require("nvim-tree.api").tree.toggle()
  end
  -- Or as an Ex command string:
  -- vim.g.zenvi_toggle_left_panel = "NvimTreeToggle"

  -- Optional: Define custom state detection (returns boolean)
  vim.g.zenvi_is_left_panel_open = function()
    return require("nvim-tree.api").tree.is_visible()
  end
end
```

### Other options

#### `vim.opt.zenvi_prewarm_max_lines`

Pre-warms all off-screen lines into Zenvi's 64-bit FNV-1a content_cache, once per buffer when opening files <= zenvi_prewarm_max_lines (default 1000), set `0` to disable pre-warming.

