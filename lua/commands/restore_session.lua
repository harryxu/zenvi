(function()
    local ok, auto_session = pcall(require, "auto-session")
    if ok and auto_session then
        pcall(auto_session.restore_session)
    end
end)()
