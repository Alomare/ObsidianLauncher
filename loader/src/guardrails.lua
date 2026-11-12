-- The player's guardrails, chosen in the app for each profile and read from the settings file at launch:
-- no Steam achievements, and a frozen account. They sit under every mod, so no mod's ask or action gets
-- past them, and they last until the game closes. Galactic War and leaderboard reports are another
-- matter, blocked for every player with session mods (backend_block).
-- Freezing keeps the account's progression as it was at launch. Writes to it are dropped, and because the
-- game also changes the save's live tables in place, the values from launch go back before every save.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

-- What a mission, the Armory or a mod can change about an account's progression.
local PROGRESS = { "player_xp", "research_samples", "item_unlocks", "new_unlocks", "item_upgrades", "boss_influence", "lifetime_stats", "trophy_unlock_progress", "seen_enemies" }
-- The counters the game unlocks achievements from later: kept too, or an achievement would wait for the
-- next launch without the guardrail.
local ACHIEVEMENT_PROGRESS = { "trophy_unlock_progress" }

local M = { held = {}, kept = nil, restoring = false, told = false }

function M.holds(key)
	return M.held[key] == true and not M.restoring
end

local function copy(value)
	if B.type(value) ~= "table" then return value end
	local out = {}
	for k, v in B.pairs(value) do out[k] = copy(v) end
	return out
end

-- The values from launch, once the save has loaded.
local function keep()
	local save, users = B.rawget(B.G, "SaveManager"), B.rawget(B.G, "UserManager")
	if B.type(save) ~= "table" or B.type(users) ~= "table" then return end
	local user = users:get_user_id()
	if save:user_get("player_xp", user) == nil then return end
	local values = {}
	for key in B.pairs(M.held) do values[key] = copy(save:user_get(key, user)) end
	M.kept = { save = save, user = user, values = values }
	log.line("guardrails: kept the account as it was at launch")
end

function M.restore()
	local kept = M.kept
	if not kept then return end
	M.restoring = true
	local ok, err = B.pcall(function()
		for key in B.pairs(M.held) do kept.save:user_set(key, copy(kept.values[key]), kept.user) end
	end)
	M.restoring = false
	if not ok then log.limited("guardrails restore", "guardrails: could not put the account back: " .. B.tostring(err)) end
end

function M.tick()
	if not M.told then
		M.told = true
		if M.core.freeze_progress then M.core.notice("progress_frozen") end
		if M.core.block_achievements then M.core.notice("achievements_blocked") end
	end
	if not M.kept and next(M.held) ~= nil then
		local ok, err = B.pcall(keep)
		if not ok then log.limited("guardrails keep", "guardrails: could not keep the account: " .. B.tostring(err)) end
	end
end

-- Whether this launch plays a custom war that allows achievements: a war line in war mode, with the
-- war_achievements line. war.setup has dropped a war line it refused by then.
function M.war_allows(core)
	return core.war ~= nil and core.war_achievements == true and core.sandbox ~= nil
end

function M.setup(core, hooks)
	M.core = core
	M.held, M.kept, M.told = {}, nil, false
	-- A Sandbox save never earns achievements, unless the custom war played on it allows them. Unlike
	-- the guardrail, that needs no notice, and the player's own guardrail applies either way.
	local no_achievements = core.block_achievements or (core.sandboxed() and not M.war_allows(core))
	if core.freeze_progress then
		for _, key in B.ipairs(PROGRESS) do M.held[key] = true end
	elseif no_achievements then
		for _, key in B.ipairs(ACHIEVEMENT_PROGRESS) do M.held[key] = true end
	end
	if not (core.freeze_progress or no_achievements) then return end
	log.line("guardrails:" .. (core.freeze_progress and " progression frozen" or "") .. (no_achievements and " achievements off" or ""))
	hooks.loader_hook("UserManager", "unlock_trophy", function(original, self, ...)
		if no_achievements then
			log.limited("achievement", "guardrails: kept an achievement from unlocking")
			return
		end
		return original(self, ...)
	end)
	-- Every way the game, or a mod, changes the account ends in this one write.
	hooks.loader_hook("SaveManager", "user_set", function(original, self, key, ...)
		if M.holds(key) then return end
		return original(self, key, ...)
	end)
	hooks.loader_hook("SaveManager", "save", function(original, self, ...)
		M.restore()
		return original(self, ...)
	end)
	hooks.loader_hook("SaveManager", "user_save", function(original, self, ...)
		M.restore()
		return original(self, ...)
	end)
	hooks.on_frame(M.tick)
end

-- obsidian.guardrails(): what the player chose, for a mod to read. Changing them is the player's alone.
function M.api(core)
	return function()
		return { consent = core.consent(), block_achievements = core.block_achievements == true, freeze_progress = core.freeze_progress == true }
	end
end

return M
