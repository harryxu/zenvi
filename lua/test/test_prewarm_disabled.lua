local max_lines = vim.g.zenvi_prewarm_max_lines
if max_lines == nil then max_lines = 1000 end
if max_lines <= 0 then return { ok = false, reason = "disabled" } end
return { ok = true }
