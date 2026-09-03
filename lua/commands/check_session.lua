local ok, auto_session = pcall(require, "auto-session")
if not ok or not auto_session then
    return { has_auto_session = false, should_restore = false, cwd = vim.fn.getcwd() }
end

local is_session_active = false
local this_session = vim.v.this_session
if this_session and this_session ~= "" then
    is_session_active = true
else
    local lib_ok, lib = pcall(require, "auto-session.lib")
    if lib_ok and lib and lib.get_session_file_name then
        local sfile = lib.get_session_file_name()
        if sfile and vim.fn.filereadable(sfile) == 1 then
            is_session_active = true
        end
    end
end

-- Check if any named file buffers are currently open
local bufs = vim.fn.getbufinfo({ buflisted = 1 })
local has_valid_buffers = false
for _, b in ipairs(bufs) do
    if b.name and b.name ~= "" then
        has_valid_buffers = true
        break
    end
end

local should_restore = false
if is_session_active or has_valid_buffers then
    pcall(auto_session.save_session)
    should_restore = true
end

return {
    has_auto_session = true,
    should_restore = should_restore,
    cwd = vim.fn.getcwd(),
}
