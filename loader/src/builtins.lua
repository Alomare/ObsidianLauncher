-- Private copies of every builtin the loader uses, taken before any mod runs, so a mod that
-- replaces a global can't break the loader or the guardrails.
local B = {
	pcall = pcall, xpcall = xpcall, error = error, type = type, select = select, next = next,
	pairs = pairs, ipairs = ipairs, tostring = tostring, tonumber = tonumber,
	rawget = rawget, rawset = rawset, rawequal = rawequal,
	setmetatable = setmetatable, getmetatable = getmetatable,
	require = require, unpack = unpack,
	format = string.format, sub = string.sub, gsub = string.gsub, match = string.match,
	find = string.find, gmatch = string.gmatch, len = string.len, lower = string.lower, rep = string.rep,
	abs = math.abs, floor = math.floor, min = math.min, max = math.max,
	insert = table.insert, remove = table.remove, concat = table.concat, sort = table.sort,
	open = io.open, getenv = os.getenv, date = os.date, clock = os.clock, time = os.time, rename = os.rename, delete = os.remove,
	traceback = debug and debug.traceback,
	G = _G,
}

-- Every value converted with B.tostring, joined by spaces, like print.
function B.join(...)
	local parts = {}
	for i = 1, B.select("#", ...) do
		parts[i] = B.tostring((B.select(i, ...)))
	end
	return B.concat(parts, " ")
end

return B
