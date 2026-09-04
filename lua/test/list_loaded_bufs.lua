local names = {}
for _, b in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_loaded(b) then
        table.insert(names, vim.api.nvim_buf_get_name(b))
    end
end
return names
