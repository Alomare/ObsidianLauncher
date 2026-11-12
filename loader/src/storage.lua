-- obsidian.storage: values a mod keeps between launches, one small text file per mod beside the
-- logs. The game can't run Lua source, so the file is lines of "key<TAB>type<TAB>value".
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { dir = nil, loaded = {} }

local ESCAPES = { ["\\"] = "\\\\", ["\t"] = "\\t", ["\n"] = "\\n", ["\r"] = "\\r" }
local UNESCAPES = { ["\\\\"] = "\\", ["\\t"] = "\t", ["\\n"] = "\n", ["\\r"] = "\r" }

local function escape(s)
	return (B.gsub(s, "[\\\t\n\r]", ESCAPES))
end

local function unescape(s)
	return (B.gsub(s, "\\.", UNESCAPES))
end

M.escape, M.unescape = escape, unescape

local function path(id)
	return M.dir .. "/" .. log.file_name(id) .. ".store"
end

local function read(id)
	local values = {}
	-- A crash between the two steps of `write` leaves the new file under its other name.
	local f = M.dir and (B.open(path(id), "rb") or B.open(path(id) .. ".tmp", "rb"))
	if not f then return values end
	local text = f:read("*a") or ""
	f:close()
	for line in B.gmatch(text, "[^\n]+") do
		local key, kind, value = B.match(line, "^([^\t]*)\t([sbn])\t(.*)$")
		if key then
			local k = unescape(key)
			if kind == "s" then
				values[k] = unescape(value)
			elseif kind == "b" then
				values[k] = value == "1"
			else
				values[k] = B.tonumber(value)
			end
		end
	end
	return values
end

local function values_of(id)
	local v = M.loaded[id]
	if not v then
		v = read(id)
		M.loaded[id] = v
	end
	return v
end

local function write(id)
	local tmp = M.dir and (path(id) .. ".tmp")
	local f = tmp and B.open(tmp, "wb")
	if not f then
		log.limited("storage " .. id, "storage: can't write the file of " .. id)
		return false
	end
	local keys = {}
	local values = M.loaded[id]
	for k in B.pairs(values) do B.insert(keys, k) end
	B.sort(keys)
	for _, k in B.ipairs(keys) do
		local v = values[k]
		local kind = B.type(v)
		if kind == "string" then
			f:write(escape(k), "\ts\t", escape(v), "\n")
		elseif kind == "boolean" then
			f:write(escape(k), "\tb\t", v and "1" or "0", "\n")
		else
			f:write(escape(k), "\tn\t", B.format("%.14g", v), "\n")
		end
	end
	f:close()
	-- Written beside the real file, so a crash can't cost a mod what it had. Windows won't rename
	-- over a file, so the old one goes first.
	B.delete(path(id))
	return B.rename(tmp, path(id)) ~= nil
end

function M.setup(core, dir)
	M.core, M.dir = core, dir
end

local function caller()
	local state = M.core and M.core.current
	if not state then B.error("obsidian.storage needs a running mod", 3) end
	return state.id
end

-- The loader's own values, kept the same way under an id no mod uses.
function M.own(id)
	return {
		get = function(key) return values_of(id)[key] end,
		put = function(values)
			local all = values_of(id)
			for k, v in B.pairs(values) do all[k] = v end
			return write(id)
		end,
	}
end

function M.api()
	return {
		get = function(key, default)
			local v = values_of(caller())[key]
			if v == nil then return default end
			return v
		end,
		-- A string, number or boolean; nil removes the key. The file is written at once.
		set = function(key, value)
			local id = caller()
			local kind = B.type(value)
			if kind ~= "string" and kind ~= "number" and kind ~= "boolean" and value ~= nil then
				B.error("obsidian.storage holds strings, numbers and booleans", 2)
			end
			values_of(id)[B.tostring(key)] = value
			return write(id)
		end,
		-- A copy of every key and value of this mod.
		all = function()
			local copy = {}
			for k, v in B.pairs(values_of(caller())) do copy[k] = v end
			return copy
		end,
	}
end

return M
