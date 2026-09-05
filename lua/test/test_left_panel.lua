-- Test Zenvi left panel Lua integration
assert(type(_G.zenvi) == "table", "zenvi table must be defined")
assert(type(_G.zenvi.toggle_left_panel) == "function", "toggle_left_panel must be a function")
assert(type(_G.zenvi.is_left_panel_open) == "function", "is_left_panel_open must be a function")

-- 1. In default clean nvim, left panel should not be open
assert(_G.zenvi.is_left_panel_open() == false, "default left panel should be closed")

-- 2. Toggle in clean environment without neo-tree should safely do nothing
local ok, err = pcall(_G.zenvi.toggle_left_panel)
assert(ok, "toggle_left_panel without neo-tree must not error: " .. tostring(err))
assert(_G.zenvi.is_left_panel_open() == false, "left panel should remain closed when neo-tree is absent")

-- 3. Custom toggle function override
local custom_called = false
vim.g.zenvi_toggle_left_panel = function()
    custom_called = true
end
_G.zenvi.toggle_left_panel()
assert(custom_called == true, "custom vim.g.zenvi_toggle_left_panel must be called")
vim.g.zenvi_toggle_left_panel = nil

-- 4. Custom is_open check override
vim.g.zenvi_is_left_panel_open = function()
    return true
end
assert(_G.zenvi.is_left_panel_open() == true, "custom is_open function must return true")
vim.g.zenvi_is_left_panel_open = nil

-- 5. Neo-tree buffer detection
local scratch_buf = vim.api.nvim_create_buf(false, true)
vim.bo[scratch_buf].filetype = "neo-tree"
local cur_win = vim.api.nvim_get_current_win()
vim.api.nvim_win_set_buf(cur_win, scratch_buf)
assert(_G.zenvi.is_left_panel_open() == true, "neo-tree buffer should be recognized as open left panel")

-- Clean up scratch buffer
vim.api.nvim_buf_delete(scratch_buf, { force = true })
assert(_G.zenvi.is_left_panel_open() == false, "left panel should be closed after neo-tree buffer deleted")

print("All left panel Lua tests passed!")
