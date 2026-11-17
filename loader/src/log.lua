-- The loader log (closed after every line, so a crash leaves the last line on disk) and per-mod logs.
local B = require("obsidian/loader/builtins")

-- A mod that logs every frame stops here instead of filling the disk.
local MOD_LINES = 20000

local M = { dir = nil, lines = {}, written = 0, opened = {}, counts = {}, mod_lines = {} }

function M.setup(dir)
	M.dir = dir
end

-- Lines wait in M.lines until the log folder is known and the write works, then leave memory.
function M.flush()
	if not M.dir or #M.lines == 0 then return end
	local f = B.open(M.dir .. "/loader.log", M.written > 0 and "a" or "w")
	if f then
		for i = 1, #M.lines do f:write(M.lines[i], "\n") end
		M.written = M.written + #M.lines
		M.lines = {}
		f:close()
	end
end

function M.line(...)
	B.insert(M.lines, B.date("%H:%M:%S ") .. B.join(...))
	M.flush()
end

-- Logs at most 3 times per site, so an error repeated every frame can't flood the log.
function M.limited(site, ...)
	local n = (M.counts[site] or 0) + 1
	M.counts[site] = n
	if n <= 3 then M.line(...) end
end

-- Only [%w_-] characters from the mod id, so no mod can write outside the logs folder.
function M.file_name(id)
	local name = B.gsub(B.tostring(id), "[^%w_%-]", "")
	if name == "" then name = "unknown" end
	return name
end

function M.mod(id, ...)
	if not M.dir then return end
	local name = M.file_name(id)
	local count = (M.mod_lines[name] or 0) + 1
	if count > MOD_LINES + 1 then return end
	M.mod_lines[name] = count
	local f = B.open(M.dir .. "/" .. name .. ".log", M.opened[name] and "a" or "w")
	if f then
		M.opened[name] = true
		if count > MOD_LINES then
			f:write(B.date("%H:%M:%S "), "the log is full, later lines are dropped\n")
		else
			f:write(B.date("%H:%M:%S "), B.join(...), "\n")
		end
		f:close()
	end
end

return M
