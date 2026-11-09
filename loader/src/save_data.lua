-- obsidian.save_data: a mod's values kept with the current save, in a text file beside it that the
-- app copies with the save. Lines are "<mod id><TAB><key><TAB><s|b|n><TAB><value>", with the escapes
-- of storage.lua. Sandbox: helldivers_sandbox_<id>.obsidian. Official: helldivers_official.obsidian.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local storage = require("obsidian/loader/storage")

local M = { path = nil, values = nil, other = nil }

local function caller()
	local state = M.core and M.core.current
	if not state then B.error("obsidian.save_data needs a running mod", 3) end
	return state.id
end

-- One line of the file, or nil when the format doesn't match (kept as it was, for a newer app).
local function parse(line)
	local mod, key, kind, value = B.match(line, "^([^\t]*)\t([^\t]*)\t([sbn])\t(.*)$")
	if not mod then return nil end
	if kind == "s" then
		value = storage.unescape(value)
	elseif kind == "b" then
		value = value == "1"
	else
		value = B.tonumber(value)
		if value == nil then return nil end
	end
	return storage.unescape(mod), storage.unescape(key), value
end

local function load()
	if M.values then return M.values end
	local values, other = {}, {}
	M.values, M.other = values, other
	-- A crash between the two steps of `write` leaves the new file under the other name.
	local f = M.path and (B.open(M.path, "rb") or B.open(M.path .. ".tmp", "rb"))
	if not f then return values end
	local text = f:read("*a") or ""
	f:close()
	for line in B.gmatch(text, "[^\n]+") do
		local mod, key, value = parse((B.gsub(line, "\r$", "")))
		if mod then
			values[mod] = values[mod] or {}
			values[mod][key] = value
		else
			B.insert(other, line)
		end
	end
	return values
end

local function write()
	local tmp = M.path and (M.path .. ".tmp")
	local f = tmp and B.open(tmp, "wb")
	if not f then
		log.limited("save_data", "save_data: can't write beside the save")
		return false
	end
	local mods = {}
	for id in B.pairs(M.values) do B.insert(mods, id) end
	B.sort(mods)
	for _, id in B.ipairs(mods) do
		local keys = {}
		for k in B.pairs(M.values[id]) do B.insert(keys, k) end
		B.sort(keys)
		for _, k in B.ipairs(keys) do
			local v = M.values[id][k]
			local kind = B.type(v)
			local head = storage.escape(id) .. "\t" .. storage.escape(k)
			if kind == "string" then
				f:write(head, "\ts\t", storage.escape(v), "\n")
			elseif kind == "boolean" then
				f:write(head, "\tb\t", v and "1" or "0", "\n")
			else
				f:write(head, "\tn\t", B.format("%.14g", v), "\n")
			end
		end
	end
	for _, line in B.ipairs(M.other) do f:write(line, "\n") end
	f:close()
	B.delete(M.path)
	return B.rename(tmp, M.path) ~= nil
end

-- A Sandbox that couldn't be read has no file to hold values: nothing is read or written.
function M.setup(core, saves_dir)
	M.core, M.values, M.other = core, nil, nil
	M.path = nil
	if saves_dir and not core.sandbox_bad then
		M.path = saves_dir .. "/" .. (core.sandbox and ("helldivers_sandbox_" .. core.sandbox) or "helldivers_official") .. ".obsidian"
	end
end

function M.api()
	return {
		get = function(key, default)
			local mod = load()[caller()]
			local v = mod and mod[B.tostring(key)]
			if v == nil then return default end
			return v
		end,
		-- A string, number or boolean; nil removes the key. The file is written at once.
		set = function(key, value)
			local id = caller()
			local kind = B.type(value)
			if kind ~= "string" and kind ~= "number" and kind ~= "boolean" and value ~= nil then
				B.error("obsidian.save_data holds strings, numbers and booleans", 2)
			end
			local all = load()
			all[id] = all[id] or {}
			all[id][B.tostring(key)] = value
			if B.next(all[id]) == nil then all[id] = nil end
			return write()
		end,
		-- The Sandbox id of this launch, nil on an Official save.
		sandbox = function()
			return M.core and M.core.sandbox or nil
		end,
	}
end

return M
