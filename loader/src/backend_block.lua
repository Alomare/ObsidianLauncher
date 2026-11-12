-- No Galactic War or leaderboard reporting, and no ranked Proving Grounds, while session content is
-- deployed (spec 6.3, layer 3), or while an expansion's content runs, and for the results of a mission
-- it ran in. That's all Obsidian keeps from the shared game: what a player earns in their own save is
-- theirs.
-- The hooks sit on the public CampaignManager entry points the game calls. The begin_* functions
-- behind them are reached through locals inside campaign_manager, so hooking those never fires.
-- A blocked call is never sent, and every table passed in is emptied: StateMenu saves the same
-- table back as the upload queue, so the results can't be sent later, not even after a purge.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local game_text = require("obsidian/loader/game_text")

-- Results wait in these save keys until the ship menu is online, so a mission finished in a session
-- and then a quit or an offline exit would leave them for the next run without session mods to send.
local COMMON_QUEUES = { "mission_stats", "mission_stats_to_upload", "completed_planets" }
local USER_QUEUES = { "leaderboard_stats", "leaderboard_stats_to_upload", "leaderboard_provinggrounds_stats", "leaderboard_provinggrounds_stats_to_upload" }

local METHODS = {
	"upload_mission_stats", "upload_leaderboard_stats", "upload_leaderboard_provinggrounds_stats",
	"complete_planets", "complete_defend_event_mission", "complete_attack_event_mission",
}

local M = {}

-- A player with no session mods of their own, who lets others' in, is in a modded game too when
-- someone here runs a different set. So is a game while an expansion's content runs in it.
local function modded(core)
	if core.deploy.has_session or core.sandboxed() then return true end
	local x = core.expansion
	if x and x.run then return true end
	if not core.consent() then return false end
	for _, m in B.ipairs(core.session.members or {}) do
		if m.obsidian and m.modset ~= core.deploy.modset then return true end
	end
	return false
end

-- The results of a mission that an expansion's content ran in wait in the save after the content
-- ends, until the ship sends them. They stay home until a mission starts without it.
local function reporting_blocked(core)
	-- A custom war: the requests can only reach the relay, so the results are the war's.
	if core.war_reporting and core.war_reporting() then return false end
	if modded(core) then return true end
	local x = core.expansion
	return x ~= nil and x.taint == true
end

local function drop_queues()
	local save, users = B.rawget(B.G, "SaveManager"), B.rawget(B.G, "UserManager")
	if B.type(save) ~= "table" then return end
	B.pcall(function()
		for _, key in B.ipairs(COMMON_QUEUES) do save:set(key, nil) end
	end)
	B.pcall(function()
		local user = users:get_user_id()
		for _, key in B.ipairs(USER_QUEUES) do save:user_set(key, nil, user) end
	end)
end

-- Proving Grounds is ranked, so a modded game doesn't open it. The galaxy map opens it through
-- show_state, the region screen first through a camera move that ends in a fade to black, and a
-- trial picked earlier waits in the save, where the ship and the campaign screen read it.
local PG_STATE = "provinggrounds_selection"

local function refuse_proving_grounds(core)
	local lm = B.rawget(B.G, "LocalizationManager")
	local ok, name = B.pcall(function() return lm:string("menu_provinggrounds") end)
	name = ok and B.type(name) == "string" and name or "Proving Grounds"
	local x = core.expansion
	local run = x and x.run
	local state = run and not core.deploy.has_session and core.mods_by_id[run.id]
	if core.sandboxed() then
		core.notice("proving_grounds_sandbox", name)
	elseif state then
		core.notice("expansion_proving_grounds", name, core.name_of(state))
	else
		core.notice("proving_grounds_blocked", name)
	end
	log.limited("proving grounds", "proving grounds: closed, the game is modded")
end

local function forget_trial(core)
	local save = B.rawget(B.G, "SaveManager")
	if not modded(core) or core.war_reporting() or B.type(save) ~= "table" or not save:get("selected_trial") then return end
	save:set("selected_trial", nil)
	log.line("proving grounds: dropped the saved trial")
end

local function empty(t)
	local keys = {}
	for k in B.pairs(t) do keys[#keys + 1] = k end
	for _, k in B.ipairs(keys) do t[k] = nil end
end

function M.setup(core, hooks)
	hooks.loader_hook("StateMenu", "on_enter", function(original, self, ...)
		forget_trial(core)
		return original(self, ...)
	end)
	hooks.loader_hook("MenuScreenCampaign", "on_enter", function(original, self, ...)
		forget_trial(core)
		return original(self, ...)
	end)
	hooks.loader_hook("MenuScreenCampaign", "show_state", function(original, self, name, ...)
		if name == PG_STATE and modded(core) and not core.war_reporting() then return refuse_proving_grounds(core) end
		return original(self, name, ...)
	end)
	hooks.loader_hook("MenuScreenCampaignRegion", "enter_pg_menu_selection", function(original, self, ...)
		if modded(core) and not core.war_reporting() then return refuse_proving_grounds(core) end
		return original(self, ...)
	end)
	for _, name in B.ipairs(METHODS) do
		hooks.loader_hook("CampaignManager", name, function(original, self, ...)
			if not reporting_blocked(core) then return original(self, ...) end
			for i = 1, B.select("#", ...) do
				local v = (B.select(i, ...))
				if B.type(v) == "table" then empty(v) end
			end
			log.limited("upload " .. name, "blocked upload: " .. name)
		end)
	end
	hooks.loader_hook("SaveManager", "save", function(original, self, ...)
		if reporting_blocked(core) then
			for _, key in B.ipairs(COMMON_QUEUES) do self:set(key, nil) end
		end
		return original(self, ...)
	end)
	hooks.loader_hook("SaveManager", "user_save", function(original, self, done, fail, subtitle, details, user, ...)
		if reporting_blocked(core) then
			for _, key in B.ipairs(USER_QUEUES) do self:user_set(key, nil, user) end
		end
		return original(self, done, fail, subtitle, details, user, ...)
	end)
	-- The mission end screen picks one of these two for the Galactic Campaign line.
	local LINES = { "mission_end_galactic_campaign_progress", "mission_end_galactic_campaign_no_progress" }
	if core.war then
		log.line("galactic war reporting: to the custom war through the relay")
		return
	end
	if core.sandboxed() then
		for _, key in B.ipairs(LINES) do game_text.replace(core, hooks, key, "campaign_not_uploaded_sandbox") end
		log.line("galactic war reporting blocked: Sandbox save")
		return
	end
	if core.deploy.has_session then
		for _, key in B.ipairs(LINES) do game_text.replace(core, hooks, key, "campaign_not_uploaded") end
		log.line("galactic war reporting blocked: session content is deployed")
		return
	end
	local x = core.expansion
	if not (x and B.next(x.mine) ~= nil) then return end
	hooks.loader_hook("StateMissionStart", "on_enter", function(original, self, ...)
		if x.mission_started() then
			-- What the content left in the save can't go out with the next results.
			drop_queues()
			log.line("expansion: missions count for the Galactic War again")
		end
		return original(self, ...)
	end)
	local function line()
		if not x.taint then return nil end
		local state = x.last and core.mods_by_id[x.last]
		return { state and core.name_of(state) or B.tostring(x.last) }
	end
	for _, key in B.ipairs(LINES) do game_text.replace_when(core, hooks, key, "campaign_not_uploaded_expansion", line) end
end

return M
