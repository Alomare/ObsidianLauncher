-- The game's tamper check, told in the log. On PC, components keep a hashed copy of some state values
-- (BaseComponent.set_protected: cooldown_timer, uses_left, rounds, hitpoints_real...) and StateGame keeps
-- one of each player's extra lives. A value written directly instead of through the setter trips the
-- check the next time the game sets it: StateGame.on_fusk_detected marks the player, stops taking
-- network messages and leaves the lobby. Interactions stop working, and the mission ends in a black
-- screen and "connection lost". Nothing here stops the check: it says which value tripped it.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { told = false }

local TRACE_LINES = 12

-- The value the game's copy holds and whether the copy is broken, or nil when it can't be read.
local function kept(copy)
	if copy == nil then return nil end
	local ok, value, broken = B.pcall(function() return copy:get() end)
	if not ok then return nil end
	return true, value, broken
end

-- What BaseComponent.check_protected(component, state, key, hashed_key) is about to find wrong, or nil.
function M.mismatch(component, state, key, hashed)
	if B.type(state) ~= "table" then return nil end
	local h = hashed
	if h == nil and B.type(component) == "table" and B.type(component.protected_keys) == "table" then h = component.protected_keys[key] end
	if h == nil then
		local unit = B.rawget(B.G, "Unit")
		local ok, id = B.pcall(function() return unit.material_id(key) end)
		if not ok then return nil end
		h = id
	end
	local read, value, broken = kept(state[h])
	if not read then return nil end
	if value ~= state[key] or broken then return { value = state[key], kept = value, broken = broken } end
	return nil
end

-- The stack below the hook, on one line.
local function trace()
	if not B.traceback then return nil end
	local ok, text = B.pcall(B.traceback, "", 3)
	if not ok or B.type(text) ~= "string" then return nil end
	local out = {}
	for line in B.gmatch(text, "[^\n]+") do
		line = B.match(line, "^%s*(.-)%s*$")
		if line ~= "" and line ~= "stack traceback:" then
			B.insert(out, line)
			if #out >= TRACE_LINES then break end
		end
	end
	return B.concat(out, " | ")
end

function M.report(where, key, found)
	local site = "tamper " .. B.tostring(where) .. " " .. B.tostring(key)
	local running = M.core.current and M.core.current.id
	log.limited(site, "tamper: " .. B.tostring(where) .. " " .. B.tostring(key) .. " is " .. B.tostring(found.value)
		.. ", the game's copy " .. B.tostring(found.kept) .. (found.broken and " (copy broken)" or "")
		.. ". Some mod wrote it directly instead of through the game's setter"
		.. (running and (", " .. running .. " running now") or ""))
	local stack = trace()
	if stack then log.limited(site .. " trace", "tamper: stack: " .. stack) end
end

function M.setup(core, hooks)
	M.core = core
	hooks.loader_hook("BaseComponent", "check_protected", function(original, self, state, key, hashed, ...)
		local ok, found = B.pcall(M.mismatch, self, state, key, hashed)
		if ok and found then M.report(B.type(self) == "table" and self.name or "component", key, found) end
		return original(self, state, key, hashed, ...)
	end)
	hooks.loader_hook("StateGame", "set_extra_lives", function(original, self, player, ...)
		if B.type(player) == "table" then
			local read, value, broken = kept(player.extra_lives_protected)
			if read and (value ~= player.extra_lives or broken) then
				M.report("StateGame", "extra_lives", { value = player.extra_lives, kept = value, broken = broken })
			end
		end
		return original(self, player, ...)
	end)
	hooks.loader_hook("StateGame", "on_fusk_detected", function(original, self, ...)
		log.line("tamper: the game's tamper check went off: it leaves the lobby and stops taking network messages")
		if not M.told then
			M.told = true
			core.notice("tamper_tripped")
		end
		return original(self, ...)
	end)
end

return M
