(function()
    local function restore_buffer_filetypes()
        local function detect()
            for _, buf in ipairs(vim.api.nvim_list_bufs()) do
                if vim.api.nvim_buf_is_loaded(buf) and vim.api.nvim_buf_get_name(buf) ~= "" then
                    if vim.bo[buf].filetype == "" then
                        vim.api.nvim_buf_call(buf, function()
                            vim.cmd("filetype detect")
                            pcall(vim.treesitter.start, buf)
                        end)
                    end
                end
            end
        end
        detect()
        vim.schedule(detect)
    end

    local group = vim.api.nvim_create_augroup("ZenviSessionAutoRestore", { clear = true })
    vim.api.nvim_create_autocmd("SessionLoadPost", {
        group = group,
        desc = "Restore filetype and syntax highlighting on session load",
        callback = restore_buffer_filetypes,
    })
    vim.api.nvim_create_autocmd("User", {
        group = group,
        pattern = { "PersistenceLoadPost", "AutoSessionRestorePost", "PossessionPostLoad", "ResessionLoadPost" },
        desc = "Restore filetype and syntax highlighting on plugin session load",
        callback = restore_buffer_filetypes,
    })

    -- ==============================================================================
    -- Idle Pre-warming Mechanism & Future Re-Prewarming Design Plan
    -- ==============================================================================
    --
    -- 1. CURRENT BEHAVIOR (BufEnter / BufReadPost):
    --    Pre-warms all off-screen lines into Zenvi's 64-bit FNV-1a content_cache
    --    once per buffer when opening files <= zenvi_prewarm_max_lines (default 1000).
    --    Operates silently after an idle pause (default 600ms) with zero screen flicker.
    --
    -- 2. FUTURE IMPLEMENTATION BLUEPRINT (Re-Prewarming on Paste / Batch Edits):
    --    When a user pastes or modifies large blocks of code, newly inserted lines
    --    beyond the viewport start as cold lines. While cold shaping is already very
    --    fast (0.08ms per line with RowSegment whitespace skipping), full prewarming
    --    can be re-triggered using the following architecture:
    --
    --    a) Long Inactivity Cooldown (Debounce):
    --       Never re-prewarm during active editing. Instead, set a 2.5s - 3.0s idle
    --       cooldown timer after TextChanged (`vim.g.zenvi_reprewarm_idle_delay_ms`).
    --    b) Mutation Threshold Gating:
    --       Track `vim.b[buf].zenvi_last_line_count`. Only schedule re-prewarm if:
    --       `math.abs(new_line_count - last_line_count) >= 10` (or after paste/undo),
    --       completely ignoring single-keystroke edits.
    --    c) Re-Arming:
    --       When the 2.5s timer expires without subsequent input, clear
    --       `vim.b[buf].zenvi_prewarmed = nil` and invoke `run_idle_prewarm()`.
    -- ==============================================================================
    local prewarm_group = vim.api.nvim_create_augroup("ZenviPrewarmGroup", { clear = true })
    local prewarm_timer = nil

    local function cancel_prewarm_timer()
        if prewarm_timer then
            pcall(function()
                prewarm_timer:stop()
                prewarm_timer:close()
            end)
            prewarm_timer = nil
        end
    end

    local function run_idle_prewarm()
        prewarm_timer = nil
        local max_lines = vim.g.zenvi_prewarm_max_lines
        if max_lines == nil then max_lines = 1000 end
        if max_lines <= 0 then return end

        local buf = vim.api.nvim_get_current_buf()
        if not vim.api.nvim_buf_is_valid(buf) then return end
        if vim.bo[buf].buftype ~= "" then return end
        if vim.b[buf].zenvi_prewarmed then return end

        local total = vim.api.nvim_buf_line_count(buf)
        if total <= 0 or total > max_lines then return end

        vim.b[buf].zenvi_prewarmed = true

        -- Notify Zenvi UI to freeze visual painting
        pcall(vim.rpcnotify, 1, "zenvi_prewarm_start")

        local save_view = vim.fn.winsaveview()
        local height = vim.api.nvim_win_get_height(0)
        if height <= 0 then height = 30 end

        -- Sweep through the buffer in viewport chunks to trigger linegrid redraw
        for l = 1, total, height do
            vim.api.nvim_win_set_cursor(0, { l, 0 })
            vim.cmd("redraw")
        end

        -- Restore original view seamlessly
        vim.fn.winrestview(save_view)
        vim.cmd("redraw")

        -- Notify Zenvi UI to unfreeze visual painting
        pcall(vim.rpcnotify, 1, "zenvi_prewarm_end")
    end

    local function schedule_idle_prewarm()
        cancel_prewarm_timer()
        local max_lines = vim.g.zenvi_prewarm_max_lines
        if max_lines == nil then max_lines = 1000 end
        if max_lines <= 0 then return end

        local buf = vim.api.nvim_get_current_buf()
        if not vim.api.nvim_buf_is_valid(buf) then return end
        if vim.bo[buf].buftype ~= "" then return end
        if vim.b[buf].zenvi_prewarmed then return end

        local total = vim.api.nvim_buf_line_count(buf)
        if total <= 0 or total > max_lines then return end

        local delay = vim.g.zenvi_prewarm_idle_delay_ms or 600
        prewarm_timer = vim.defer_fn(run_idle_prewarm, delay)
    end

    vim.api.nvim_create_autocmd({ "BufEnter", "BufReadPost" }, {
        group = prewarm_group,
        callback = schedule_idle_prewarm,
    })

    vim.api.nvim_create_autocmd({ "CursorMoved", "InsertEnter", "TextChanged" }, {
        group = prewarm_group,
        callback = cancel_prewarm_timer,
    })

    -- ==============================================================================
    -- Zenvi Panel Integration (Left Panel / Neo-tree / Custom Panel)
    -- ==============================================================================
    _G.zenvi = _G.zenvi or {}

    local function is_neotree_open()
        for _, win in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
            if vim.api.nvim_win_is_valid(win) then
                local buf = vim.api.nvim_win_get_buf(win)
                if vim.api.nvim_buf_is_valid(buf) and vim.bo[buf].filetype == "neo-tree" then
                    return true
                end
            end
        end
        return false
    end

    local function is_left_panel_open()
        if type(vim.g.zenvi_is_left_panel_open) == "function" then
            local ok, res = pcall(vim.g.zenvi_is_left_panel_open)
            if ok then return not not res end
        elseif type(vim.g.zenvi_is_left_panel_open) == "boolean" then
            return vim.g.zenvi_is_left_panel_open
        end

        if type(_G.zenvi_is_left_panel_open) == "function" then
            local ok, res = pcall(_G.zenvi_is_left_panel_open)
            if ok then return not not res end
        end

        if type(zenvi.custom_is_left_panel_open) == "function" then
            local ok, res = pcall(zenvi.custom_is_left_panel_open)
            if ok then return not not res end
        end

        return is_neotree_open()
    end

    local function notify_left_panel_state()
        local open = is_left_panel_open()
        pcall(vim.rpcnotify, 1, "zenvi_left_panel_state", open)
    end

    local function toggle_left_panel()
        local custom_fn = nil
        if type(vim.g.zenvi_toggle_left_panel) == "function" then
            custom_fn = vim.g.zenvi_toggle_left_panel
        elseif type(vim.g.zenvi_toggle_left_panel) == "string" then
            local cmd = vim.g.zenvi_toggle_left_panel
            custom_fn = function() vim.cmd(cmd) end
        elseif type(_G.zenvi_toggle_left_panel) == "function" then
            custom_fn = _G.zenvi_toggle_left_panel
        elseif type(zenvi.custom_toggle_left_panel) == "function" then
            custom_fn = zenvi.custom_toggle_left_panel
        elseif type(vim.g.zenvi_toggle_panel) == "function" then
            custom_fn = vim.g.zenvi_toggle_panel
        elseif type(vim.g.zenvi_toggle_panel) == "string" then
            local cmd = vim.g.zenvi_toggle_panel
            custom_fn = function() vim.cmd(cmd) end
        elseif type(_G.zenvi_toggle_panel) == "function" then
            custom_fn = _G.zenvi_toggle_panel
        end

        if custom_fn then
            pcall(custom_fn)
        else
            local ok, neotree = pcall(require, "neo-tree.command")
            if ok and neotree and type(neotree.execute) == "function" then
                pcall(neotree.execute, { toggle = true, position = "left" })
            elseif vim.fn.exists(":Neotree") == 2 then
                pcall(vim.cmd, "Neotree toggle left")
            else
                vim.notify(
                    "Zenvi: neo-tree is not installed and no vim.g.zenvi_toggle_left_panel is configured.",
                    vim.log.levels.WARN
                )
            end
        end

        vim.schedule(notify_left_panel_state)
    end

    local function is_right_panel_open()
        if type(vim.g.zenvi_is_right_panel_open) == "function" then
            local ok, res = pcall(vim.g.zenvi_is_right_panel_open)
            if ok then return not not res end
        elseif type(vim.g.zenvi_is_right_panel_open) == "boolean" then
            return vim.g.zenvi_is_right_panel_open
        end

        if type(_G.zenvi_is_right_panel_open) == "function" then
            local ok, res = pcall(_G.zenvi_is_right_panel_open)
            if ok then return not not res end
        end

        if type(zenvi.custom_is_right_panel_open) == "function" then
            local ok, res = pcall(zenvi.custom_is_right_panel_open)
            if ok then return not not res end
        end

        return false
    end

    local function notify_right_panel_state()
        local open = is_right_panel_open()
        pcall(vim.rpcnotify, 1, "zenvi_right_panel_state", open)
    end

    local function toggle_right_panel()
        local custom_fn = nil
        if type(vim.g.zenvi_toggle_right_panel) == "function" then
            custom_fn = vim.g.zenvi_toggle_right_panel
        elseif type(vim.g.zenvi_toggle_right_panel) == "string" then
            local cmd = vim.g.zenvi_toggle_right_panel
            custom_fn = function() vim.cmd(cmd) end
        elseif type(_G.zenvi_toggle_right_panel) == "function" then
            custom_fn = _G.zenvi_toggle_right_panel
        elseif type(zenvi.custom_toggle_right_panel) == "function" then
            custom_fn = zenvi.custom_toggle_right_panel
        end

        if custom_fn then
            pcall(custom_fn)
        else
            vim.notify(
                "Zenvi: No right panel toggle function configured. Please set vim.g.zenvi_toggle_right_panel in your config.",
                vim.log.levels.WARN
            )
        end

        vim.schedule(notify_right_panel_state)
    end

    local function is_bottom_panel_open()
        if type(vim.g.zenvi_is_bottom_panel_open) == "function" then
            local ok, res = pcall(vim.g.zenvi_is_bottom_panel_open)
            if ok then return not not res end
        elseif type(vim.g.zenvi_is_bottom_panel_open) == "boolean" then
            return vim.g.zenvi_is_bottom_panel_open
        end

        if type(_G.zenvi_is_bottom_panel_open) == "function" then
            local ok, res = pcall(_G.zenvi_is_bottom_panel_open)
            if ok then return not not res end
        end

        if type(zenvi.custom_is_bottom_panel_open) == "function" then
            local ok, res = pcall(zenvi.custom_is_bottom_panel_open)
            if ok then return not not res end
        end

        for _, win in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
            if vim.api.nvim_win_is_valid(win) then
                local buf = vim.api.nvim_win_get_buf(win)
                if vim.api.nvim_buf_is_valid(buf) then
                    local bt = vim.bo[buf].buftype
                    local ft = vim.bo[buf].filetype
                    if bt == "terminal" or ft == "toggleterm" then
                        return true
                    end
                end
            end
        end
        return false
    end

    local function notify_bottom_panel_state()
        local open = is_bottom_panel_open()
        pcall(vim.rpcnotify, 1, "zenvi_bottom_panel_state", open)
    end

    local function default_toggle_bottom_panel()
        local ok_toggleterm, toggleterm = pcall(require, "toggleterm")
        if ok_toggleterm and toggleterm and type(toggleterm.toggle) == "function" then
            pcall(toggleterm.toggle, 1, 12, nil, "horizontal")
            return
        end
        if vim.fn.exists(":ToggleTerm") == 2 then
            pcall(vim.cmd, "ToggleTerm direction=horizontal")
            return
        end

        local term_win = nil
        for _, win in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
            if vim.api.nvim_win_is_valid(win) then
                local buf = vim.api.nvim_win_get_buf(win)
                if vim.api.nvim_buf_is_valid(buf) then
                    local bt = vim.bo[buf].buftype
                    local ft = vim.bo[buf].filetype
                    if bt == "terminal" or ft == "toggleterm" then
                        term_win = win
                        break
                    end
                end
            end
        end

        if term_win then
            vim.api.nvim_win_close(term_win, false)
        else
            local existing_buf = nil
            for _, buf in ipairs(vim.api.nvim_list_bufs()) do
                if vim.api.nvim_buf_is_valid(buf) and vim.bo[buf].buftype == "terminal" then
                    existing_buf = buf
                    break
                end
            end

            vim.cmd("botright 12split")
            if existing_buf then
                vim.api.nvim_win_set_buf(0, existing_buf)
            else
                vim.cmd("terminal")
            end
            pcall(vim.cmd, "startinsert")
        end
    end

    local function toggle_bottom_panel()
        local custom_fn = nil
        if type(vim.g.zenvi_toggle_bottom_panel) == "function" then
            custom_fn = vim.g.zenvi_toggle_bottom_panel
        elseif type(vim.g.zenvi_toggle_bottom_panel) == "string" then
            local cmd = vim.g.zenvi_toggle_bottom_panel
            custom_fn = function() vim.cmd(cmd) end
        elseif type(_G.zenvi_toggle_bottom_panel) == "function" then
            custom_fn = _G.zenvi_toggle_bottom_panel
        elseif type(zenvi.custom_toggle_bottom_panel) == "function" then
            custom_fn = zenvi.custom_toggle_bottom_panel
        end

        if custom_fn then
            pcall(custom_fn)
        else
            default_toggle_bottom_panel()
        end

        vim.schedule(notify_bottom_panel_state)
    end

    zenvi.is_left_panel_open = is_left_panel_open
    zenvi.toggle_left_panel = toggle_left_panel
    zenvi.toggle_panel = toggle_left_panel
    zenvi.notify_left_panel_state = notify_left_panel_state

    zenvi.is_bottom_panel_open = is_bottom_panel_open
    zenvi.toggle_bottom_panel = toggle_bottom_panel
    zenvi.notify_bottom_panel_state = notify_bottom_panel_state

    zenvi.is_right_panel_open = is_right_panel_open
    zenvi.toggle_right_panel = toggle_right_panel
    zenvi.notify_right_panel_state = notify_right_panel_state

    pcall(vim.api.nvim_create_user_command, "ZenviToggleLeftPanel", function()
        zenvi.toggle_left_panel()
    end, { desc = "Toggle Zenvi left panel" })

    pcall(vim.api.nvim_create_user_command, "ZenviToggleBottomPanel", function()
        zenvi.toggle_bottom_panel()
    end, { desc = "Toggle Zenvi bottom panel (terminal)" })

    pcall(vim.api.nvim_create_user_command, "ZenviToggleRightPanel", function()
        zenvi.toggle_right_panel()
    end, { desc = "Toggle Zenvi right panel" })

    local panel_group = vim.api.nvim_create_augroup("ZenviPanelGroup", { clear = true })
    vim.api.nvim_create_autocmd({ "BufWinEnter", "BufWinLeave", "WinClosed", "TabEnter" }, {
        group = panel_group,
        callback = function()
            vim.schedule(function()
                notify_left_panel_state()
                notify_bottom_panel_state()
                notify_right_panel_state()
            end)
        end,
    })

    vim.defer_fn(function()
        notify_left_panel_state()
        notify_bottom_panel_state()
        notify_right_panel_state()
    end, 100)

    -- ==============================================================================
    -- Delicate Statusline Subsystem
    -- ==============================================================================
    local delicate_group = vim.api.nvim_create_augroup("ZenviDelicateStatusline", { clear = true })
    local delicate_timer = nil

    local function parse_hl_attrs(group)
        if not group or group == "" then return {} end
        local ok, h = pcall(vim.api.nvim_get_hl, 0, { name = group, link = false })
        if not ok or type(h) ~= "table" then return {} end
        local fg = h.fg
        local bg = h.bg
        if h.reverse then
            fg, bg = bg, fg
        end
        return {
            fg = fg,
            bg = bg,
            bold = h.bold or false,
            italic = h.italic or false,
            underline = h.underline or false,
        }
    end

    local function split_statusline_by_divider(stl)
        local parts = {}
        local last_idx = 1
        local len = #stl
        local i = 1
        while i <= len do
            if string.sub(stl, i, i + 1) == "%=" then
                local p = i - 1
                local percent_count = 0
                while p >= 1 and string.sub(stl, p, p) == "%" do
                    percent_count = percent_count + 1
                    p = p - 1
                end
                if percent_count % 2 == 0 then
                    table.insert(parts, string.sub(stl, last_idx, i - 1))
                    last_idx = i + 2
                    i = i + 2
                else
                    i = i + 1
                end
            else
                i = i + 1
            end
        end
        table.insert(parts, string.sub(stl, last_idx))
        return parts
    end

    local function parse_part_spans(part, win)
        if not part or part == "" then return {} end
        local ok, res = pcall(vim.api.nvim_eval_statusline, part, { winid = win, highlights = true, maxwidth = 999 })
        if not ok or not res then return {} end
        local spans = {}
        local hls = res.highlights or {}
        local str = res.str or ""
        local len = #str

        if #hls == 0 then
            if len > 0 then
                table.insert(spans, { text = str })
            end
        else
            if hls[1].start > 0 then
                table.insert(spans, { text = string.sub(str, 1, hls[1].start) })
            end
            for i = 1, #hls do
                local start_pos = hls[i].start + 1
                local end_pos = (i < #hls) and hls[i+1].start or len
                if start_pos <= end_pos and start_pos <= len then
                    local text = string.sub(str, start_pos, end_pos)
                    local attrs = parse_hl_attrs(hls[i].group)
                    attrs.text = text
                    table.insert(spans, attrs)
                end
            end
        end
        return spans
    end

    local function build_delicate_payload()
        local win = vim.api.nvim_get_current_win()
        if not vim.api.nvim_win_is_valid(win) then return nil end
        local stl = nil
        if package.loaded["lualine"] then
            local ok, res = pcall(require("lualine").statusline, true)
            if ok and type(res) == "string" and res ~= "" then
                stl = res
            end
        end
        if stl == nil or stl == "" then
            stl = vim.wo[win].statusline
        end
        if stl == nil or stl == "" then
            stl = vim.go.statusline
        end
        if stl == nil or stl == "" then
            stl = "%<%f %h%m%r%=%-14.(%l,%c%V%) %P"
        end

        local parts = split_statusline_by_divider(stl)
        local left_spans = {}
        local center_spans = {}
        local right_spans = {}

        if #parts == 1 then
            left_spans = parse_part_spans(parts[1], win)
        elseif #parts == 2 then
            left_spans = parse_part_spans(parts[1], win)
            right_spans = parse_part_spans(parts[2], win)
        elseif #parts >= 3 then
            left_spans = parse_part_spans(parts[1], win)
            center_spans = parse_part_spans(parts[2], win)
            right_spans = parse_part_spans(parts[3], win)
        end

        local bar_bg = nil
        local hl_c = parse_hl_attrs("lualine_c_normal")
        if hl_c.bg then
            bar_bg = hl_c.bg
        else
            local hl_stl = parse_hl_attrs("StatusLine")
            if hl_stl.bg then
                bar_bg = hl_stl.bg
            end
        end

        return {
            left_spans = left_spans,
            center_spans = center_spans,
            right_spans = right_spans,
            bg = bar_bg,
        }
    end

    local function send_statusline_update()
        delicate_timer = nil
        local enabled = vim.g.zenvi_delicate_statusline
        if enabled == false or enabled == 0 then
            return
        end
        local ok, payload = pcall(build_delicate_payload)
        if ok and payload then
            pcall(vim.rpcnotify, 1, "zenvi_statusline_update", payload)
        end
    end

    local function trigger_statusline_update(immediate)
        local enabled = vim.g.zenvi_delicate_statusline
        if enabled == false or enabled == 0 then
            return
        end
        if immediate then
            if delicate_timer then
                pcall(function()
                    delicate_timer:stop()
                    delicate_timer:close()
                end)
                delicate_timer = nil
            end
            send_statusline_update()
        else
            if not delicate_timer then
                delicate_timer = vim.defer_fn(send_statusline_update, 16)
            end
        end
    end

    local function enforce_laststatus()
        local enabled = vim.g.zenvi_delicate_statusline
        if enabled == nil or enabled == true or enabled == 1 then
            if vim.o.laststatus ~= 0 then
                vim.o.laststatus = 0
            end
        end
    end

    local function notify_delicate_config()
        local enabled = vim.g.zenvi_delicate_statusline
        if enabled == nil then
            enabled = true
        elseif enabled == 0 or enabled == false then
            enabled = false
        else
            enabled = true
        end

        local font = vim.g.zenvi_delicate_statusline_font or ""

        if enabled then
            enforce_laststatus()
        end

        pcall(vim.rpcnotify, 1, "zenvi_statusline_config", {
            enabled = enabled,
            font = font,
        })

        if enabled then
            trigger_statusline_update(true)
        end
    end

    -- Hook statusline autocmds
    vim.api.nvim_create_autocmd({
        "UIEnter", "VimEnter", "BufEnter", "WinEnter", "BufWritePost", "ModeChanged",
        "DiagnosticChanged", "VimResized", "ColorScheme", "TermEnter", "TermLeave",
        "SessionLoadPost", "FileType"
    }, {
        group = delicate_group,
        callback = function()
            enforce_laststatus()
            trigger_statusline_update(true)
        end,
    })

    vim.api.nvim_create_autocmd({ "CursorMoved", "CursorMovedI" }, {
        group = delicate_group,
        callback = function()
            trigger_statusline_update(false)
        end,
    })

    vim.api.nvim_create_autocmd("OptionSet", {
        group = delicate_group,
        pattern = "laststatus",
        callback = function()
            local enabled = vim.g.zenvi_delicate_statusline
            if (enabled == nil or enabled == true or enabled == 1) and vim.o.laststatus ~= 0 then
                vim.schedule(function()
                    enforce_laststatus()
                end)
            end
            trigger_statusline_update(true)
        end,
    })

    vim.api.nvim_create_autocmd("OptionSet", {
        group = delicate_group,
        pattern = "statusline",
        callback = function()
            trigger_statusline_update(true)
        end,
    })

    zenvi.notify_delicate_config = notify_delicate_config
    zenvi.trigger_statusline_update = trigger_statusline_update
    zenvi.enforce_laststatus = enforce_laststatus

    -- Run initial enforcement across startup intervals to override lazy-loaded plugins (like lualine)
    for _, delay in ipairs({ 10, 50, 150, 300, 600, 1200 }) do
        vim.defer_fn(function()
            enforce_laststatus()
            notify_delicate_config()
        end, delay)
    end
end)()
