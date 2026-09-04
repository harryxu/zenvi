local max_lines = vim.g.zenvi_prewarm_max_lines
if max_lines == nil then max_lines = 1000 end
if max_lines <= 0 then return { ok = false, reason = "disabled" } end

local count = vim.fn.line("$")
if count > max_lines or count <= 0 then
    return { ok = false, reason = "too_large", count = count }
end

local save = vim.fn.winsaveview()
local h = vim.api.nvim_win_get_height(0)
for l = 1, count, h do
    vim.api.nvim_win_set_cursor(0, { l, 0 })
    vim.cmd("redraw")
end
vim.fn.winrestview(save)
vim.cmd("redraw")
return { ok = true, count = count }
