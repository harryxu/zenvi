local win = vim.api.nvim_get_current_win()
local buf = vim.api.nvim_get_current_buf()
local ft = vim.bo[buf].filetype
local name = vim.api.nvim_buf_get_name(buf)
local lines = vim.api.nvim_buf_line_count(buf)
return {
    win = win,
    buf = buf,
    filetype = ft,
    name = name,
    lines = lines,
}
