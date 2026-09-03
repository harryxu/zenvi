(function()
    local text = {
        "",
        "   ⚡ Zenvi - Standalone Neovim GUI Shell",
        "   ──────────────────────────────────────",
        "   Version : 0.1.0",
        "   Engine  : Rust + GPUI",
        "   Backend : Embedded Neovim (RPC)",
        "",
        "   A lightweight, GPU-accelerated desktop",
        "   frontend for Neovim built with GPUI.",
        "",
        "   https://github.com/harryxu/zenvi",
        "",
        "   [ Press 'q' or <Esc> to close ]",
        "",
    }
    local buf = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, text)
    vim.bo[buf].modifiable = false
    vim.bo[buf].bufhidden = "wipe"
    vim.bo[buf].buftype = "nofile"
    vim.bo[buf].filetype = "zenvi_about"
    vim.b[buf].scrollbar_disabled = true
    vim.b[buf].satellite = false
    vim.b[buf].minianimate_disable = true
    vim.b[buf].miniindentscope_disable = true

    local max_w = 0
    for _, line in ipairs(text) do
        max_w = math.max(max_w, vim.fn.strdisplaywidth(line))
    end
    local width = math.max(48, max_w + 4)
    local height = #text
    local ui = vim.api.nvim_list_uis()[1]
    local screen_w = ui and ui.width or vim.o.columns
    local screen_h = ui and ui.height or vim.o.lines

    local row = math.max(1, math.floor((screen_h - height) / 2))
    local col = math.max(1, math.floor((screen_w - width) / 2))

    local win_opts = {
        relative = "editor",
        width = width,
        height = height,
        row = row,
        col = col,
        style = "minimal",
        border = "rounded",
        noautocmd = true,
    }
    pcall(function()
        win_opts.title = " About Zenvi "
        win_opts.title_pos = "center"
    end)

    local ok, win = pcall(vim.api.nvim_open_win, buf, true, win_opts)
    if not ok then
        vim.notify(table.concat(text, "\n"), vim.log.levels.INFO)
        return
    end

    pcall(function()
        vim.wo[win].wrap = false
        vim.wo[win].cursorline = false
        vim.wo[win].cursorcolumn = false
        vim.wo[win].number = false
        vim.wo[win].relativenumber = false
        vim.wo[win].signcolumn = "no"
        vim.wo[win].foldcolumn = "0"
        vim.wo[win].statuscolumn = ""
        vim.wo[win].winblend = 0
    end)

    local close = function()
        if vim.api.nvim_win_is_valid(win) then
            pcall(vim.api.nvim_win_close, win, true)
        end
    end

    vim.keymap.set("n", "q", close, { buffer = buf, nowait = true, silent = true })
    vim.keymap.set("n", "<Esc>", close, { buffer = buf, nowait = true, silent = true })
    vim.keymap.set("n", "<CR>", close, { buffer = buf, nowait = true, silent = true })
end)()
