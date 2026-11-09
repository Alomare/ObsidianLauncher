-- Sandbox saves: while a Sandbox is chosen, the game's save system opens helldivers_sandbox_<id> in
-- the saves folder through the default system, and the backup system is dropped, so nothing in this
-- launch reaches Steam Cloud or the Official files.
-- Saving starts locked and only a confirmed redirect opens it: a hook that didn't apply, a Sandbox
-- line that can't be used or a game build the loader doesn't know all leave saving off for the launch.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { id = nil, confirmed = false, active = false, told = false, dropped = 0 }

function M.file_name(id)
	return "helldivers_sandbox_" .. id
end

-- Whether this launch has a Sandbox chosen, usable or not.
function M.wanted(core)
	return core.sandbox ~= nil or core.sandbox_bad == true
end

-- A save the Sandbox rules refuse. The game's flow waits for the done callback, so that still runs.
local function refuse(done)
	M.dropped = M.dropped + 1
	if M.dropped <= 3 then log.line("sandbox: a save was refused, the Sandbox save isn't open") end
	if B.type(done) == "function" then B.pcall(done) end
end

-- Runs once per frame. The game calls SaveManager.setup during init, well before these frames, so a
-- Sandbox still unconfirmed after a few of them didn't open.
local WAIT_FRAMES = 5

function M.tick()
	if M.told or M.confirmed or not M.active then return end
	M.frames = (M.frames or 0) + 1
	if M.frames < WAIT_FRAMES then return end
	M.told = true
	log.line("sandbox: the Sandbox save didn't open, saving is off for this launch")
	M.core.notice("sandbox_failed")
end

-- locked: a launch that can't run a Sandbox at all (an unknown build, no deploy): nothing redirects and
-- saving stays off.
function M.setup(core, hooks, locked)
	M.core = core
	if not M.wanted(core) then return end
	M.active = true
	M.id = not locked and core.sandbox or nil
	hooks.loader_hook("SaveManager", "save", function(original, self, done, ...)
		if not M.confirmed then return refuse(done) end
		return original(self, done, ...)
	end)
	hooks.loader_hook("SaveManager", "user_save", function(original, self, done, ...)
		if not M.confirmed then return refuse(done) end
		return original(self, done, ...)
	end)
	hooks.on_frame(M.tick)
	if not M.id then return end
	local file = M.file_name(M.id)
	hooks.loader_hook("SaveManager", "setup", function(original, self, _, _, ...)
		original(self, "default", file, ...)
		M.confirmed = self.filename == file and self.system_type == "default"
		log.line("sandbox: " .. (M.confirmed and "save redirected to " or "the redirect didn't take, wanted ") .. file)
		return
	end)
	hooks.loader_hook("SaveManager", "set_backup_system", function() end)
end

return M
