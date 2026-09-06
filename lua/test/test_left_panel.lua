-- Test Zenvi panel Lua integration (left & right panels)
assert(type(_G.zenvi) == "table", "zenvi table must be defined")
assert(type(_G.zenvi.toggle_left_panel) == "function", "toggle_left_panel must be a function")
assert(type(_G.zenvi.is_left_panel_open) == "function", "is_left_panel_open must be a function")
assert(type(_G.zenvi.toggle_right_panel) == "function", "toggle_right_panel must be a function")
assert(type(_G.zenvi.is_right_panel_open) == "function", "is_right_panel_open must be a function")

-- Intercept vim.notify to test warning messages
local notifications = {}
vim.notify = function(msg, level)
    table.insert(notifications, { msg = msg, level = level })
end

-- 1. In default clean nvim, panels should not be open
assert(_G.zenvi.is_left_panel_open() == false, "default left panel should be closed")
assert(_G.zenvi.is_right_panel_open() == false, "default right panel should be closed")

-- 2. Toggle left panel in clean environment without neo-tree should trigger warning notification
notifications = {}
local ok, err = pcall(_G.zenvi.toggle_left_panel)
assert(ok, "toggle_left_panel without neo-tree must not crash: " .. tostring(err))
assert(_G.zenvi.is_left_panel_open() == false, "left panel should remain closed when neo-tree is absent")
assert(#notifications > 0, "should notify user when neo-tree is absent")
assert(string.find(notifications[#notifications].msg, "neo%-tree"), "notification should mention neo-tree")

-- 3. Custom toggle function override for left panel
local left_custom_called = false
vim.g.zenvi_toggle_left_panel = function()
    left_custom_called = true
end
_G.zenvi.toggle_left_panel()
assert(left_custom_called == true, "custom vim.g.zenvi_toggle_left_panel must be called")
vim.g.zenvi_toggle_left_panel = nil

-- 4. Custom is_open check override for left panel
vim.g.zenvi_is_left_panel_open = function()
    return true
end
assert(_G.zenvi.is_left_panel_open() == true, "custom is_open function must return true")
vim.g.zenvi_is_left_panel_open = nil

-- 5. Neo-tree buffer detection for left panel
local scratch_buf = vim.api.nvim_create_buf(false, true)
vim.bo[scratch_buf].filetype = "neo-tree"
local cur_win = vim.api.nvim_get_current_win()
vim.api.nvim_win_set_buf(cur_win, scratch_buf)
assert(_G.zenvi.is_left_panel_open() == true, "neo-tree buffer should be recognized as open left panel")

-- Clean up scratch buffer
vim.api.nvim_buf_delete(scratch_buf, { force = true })
assert(_G.zenvi.is_left_panel_open() == false, "left panel should be closed after neo-tree buffer deleted")

-- 6. Toggle right panel without configured function should trigger warning notification
notifications = {}
local r_ok, r_err = pcall(_G.zenvi.toggle_right_panel)
assert(r_ok, "toggle_right_panel without config must not crash: " .. tostring(r_err))
assert(_G.zenvi.is_right_panel_open() == false, "right panel should remain closed when not configured")
assert(#notifications > 0, "should notify user when right panel toggle is not configured")
assert(string.find(notifications[#notifications].msg, "right panel"), "notification should mention right panel")

-- 7. Custom toggle function for right panel
local right_custom_called = false
vim.g.zenvi_toggle_right_panel = function()
    right_custom_called = true
end
_G.zenvi.toggle_right_panel()
assert(right_custom_called == true, "custom vim.g.zenvi_toggle_right_panel must be called")
vim.g.zenvi_toggle_right_panel = nil

-- 8. Custom is_open check for right panel
vim.g.zenvi_is_right_panel_open = function()
    return true
end
assert(_G.zenvi.is_right_panel_open() == true, "custom right is_open function must return true")
vim.g.zenvi_is_right_panel_open = nil
assert(_G.zenvi.is_right_panel_open() == false, "right panel should be closed after resetting check function")

print("All left & right panel Lua tests passed!")
