return {
    buf = vim.api.nvim_get_current_buf(),
    name = vim.api.nvim_buf_get_name(0),
    ft = vim.bo.filetype,
}
