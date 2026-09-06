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
end)()
