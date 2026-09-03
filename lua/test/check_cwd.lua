return {
    cwd = vim.fn.getcwd(),
    is_dir = vim.fn.isdirectory(vim.fn.expand("%:p")) == 1,
    bufname = vim.api.nvim_buf_get_name(0),
}
