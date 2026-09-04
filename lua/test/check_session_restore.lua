local current_buf = vim.api.nvim_get_current_buf()
local ft = vim.bo[current_buf].filetype
local syn = vim.bo[current_buf].syntax
return {
    buf = current_buf,
    name = vim.api.nvim_buf_get_name(current_buf),
    ft = ft,
    syn = syn,
}
