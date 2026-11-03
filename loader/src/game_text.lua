-- Game texts the loader replaces: LocalizationManager.string returns a loader text (in the game
-- language) for the chosen keys. That function runs for every text on screen, every frame, so the
-- wrapper stays lean instead of going through the protected hook path.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { map = {}, args = {}, when = {}, installed = false, waiting = false }

local function install()
	local lm = B.rawget(B.G, "LocalizationManager")
	if B.type(lm) ~= "table" or B.type(lm.string) ~= "function" then return false end
	local original, map, args, when, core = lm.string, M.map, M.args, M.when, M.core
	lm.string = function(self, key, ...)
		local own = map[key]
		if own then
			local a = args[key]
			local w = when[key]
			if w then
				local ok, got = B.pcall(w)
				if not (ok and got) then return original(self, key, ...) end
				a = B.type(got) == "table" and got or nil
			end
			if a then return core.text(own, B.unpack(a)) end
			return core.text(own)
		end
		return original(self, key, ...)
	end
	M.installed = true
	log.line("hook installed: LocalizationManager.string")
	return true
end

-- From then on the game key shows the loader text `own` from the shared catalog, filled with `args`.
function M.replace(core, hooks, key, own, args)
	M.core = core
	M.map[key], M.args[key], M.when[key] = own, args, nil
	if M.installed or M.waiting or install() then return end
	M.waiting = true
	hooks.on_frame(function()
		if not M.installed then install() end
	end)
end

-- The same, only while when() answers: true, or the args as a list. Otherwise the game's own text.
function M.replace_when(core, hooks, key, own, when)
	M.replace(core, hooks, key, own, nil)
	M.when[key] = when
end

return M
