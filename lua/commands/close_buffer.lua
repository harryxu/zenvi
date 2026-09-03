(function()
    local cur = vim.api.nvim_get_current_buf()
    local bufs = vim.tbl_filter(function(b)
        return vim.api.nvim_buf_is_valid(b) and vim.bo[b].buflisted
    end, vim.api.nvim_list_bufs())

    if #bufs <= 1 then
        vim.cmd("confirm quit")
        return
    end

    local alt = vim.fn.bufnr("#")
    local next_buf = nil
    if alt > 0 and alt ~= cur and vim.api.nvim_buf_is_valid(alt) and vim.bo[alt].buflisted then
        next_buf = alt
    else
        for _, b in ipairs(bufs) do
            if b ~= cur then
                next_buf = b
                break
            end
        end
    end

    if next_buf then
        for _, w in ipairs(vim.api.nvim_list_wins()) do
            if vim.api.nvim_win_is_valid(w) and vim.api.nvim_win_get_buf(w) == cur then
                vim.api.nvim_win_set_buf(w, next_buf)
            end
        end
    end

    vim.cmd("confirm bdelete " .. cur)
end)()
