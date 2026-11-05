-- What multiplayer looks like for this deploy, told once, the first time the player loads into the
-- ship: free to play with anyone, session mods that switch off in some public lobbies, content that
-- can't switch off and keeps vanilla players out of public lobbies, or an expansion, whose running
-- content turns away players without it.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = {}

-- The notice key for a deploy.
function M.key(deploy)
	if deploy.has_static_session then return "status_isolated" end
	if deploy.has_session then return "status_session" end
	if B.type(deploy.expansions) == "table" and B.next(deploy.expansions) ~= nil then return "status_expansion" end
	return "status_clear"
end

function M.setup(core, hooks)
	local told = false
	local function tell(...)
		if not told then
			told = true
			local key = M.key(core.deploy)
			log.line("status: " .. key)
			core.notice(key)
		end
		return ...
	end
	-- After the game's own on_enter, which makes the ship's hint box.
	hooks.loader_hook("StateMenu", "on_enter", function(original, self, ...)
		return tell(original(self, ...))
	end)
end

return M
