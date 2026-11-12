local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

-- A loader started with this settings file, beside a session mod, with the game's save and user manager.
local function engine(settings)
	local e = E.new({ deploy = E.deploy({ has_session = true, modset = string.rep("a", 64) }), settings = settings, modules = {} })
	e:boot()
	local save = { user = { player_xp = 100, research_samples = 50, item_unlocks = { "owned" }, trophy_unlock_progress = { kills = 3 }, customization = "helmet" }, saves = 0 }
	rawset(_G, "SaveManager", {
		set = function() end,
		user_get = function(_, k) return save.user[k] end,
		user_set = function(_, k, v) save.user[k] = v end,
		save = function() save.saves = save.saves + 1 end,
		user_save = function() save.saves = save.saves + 1 end,
	})
	e.opts.modules["foundation/lua/managers/save_manager"] = true
	rawget(_G, "require")("foundation/lua/managers/save_manager")
	local trophies = {}
	rawset(_G, "UserManager", { get_user_id = function() return "user" end, unlock_trophy = function(_, name) trophies[#trophies + 1] = name end })
	e.opts.modules["lua/managers/user_manager"] = true
	rawget(_G, "require")("lua/managers/user_manager")
	e:start_frames()
	e:frames(2)
	return e, save, trophies
end

local function keys_shown(e)
	local out = {}
	for _, n in ipairs(e:core().notices_shown) do out[#out + 1] = n.key end
	return table.concat(out, ",")
end

test("without_guardrails_the_game_saves_and_unlocks_as_usual", function()
	local e, save, trophies = engine("consent\t0\n")
	local sm = rawget(_G, "SaveManager")
	sm:user_set("player_xp", 900, "user")
	rawget(_G, "UserManager"):unlock_trophy("first")
	E.eq(save.user.player_xp, 900)
	E.eq(trophies[1], "first")
	local g = rawget(_G, "obsidian").guardrails()
	E.eq(g.consent, false) E.eq(g.block_achievements, false) E.eq(g.freeze_progress, false)
	E.eq(keys_shown(e):find("progress_frozen"), nil, "nothing to tell")
	E.eq(keys_shown(e):find("achievements_blocked"), nil)
end)

test("a_frozen_account_keeps_its_progression_from_launch", function()
	local e, save = engine("freeze_progress\t1\n")
	local sm = rawget(_G, "SaveManager")
	sm:user_set("player_xp", 900, "user")
	E.eq(save.user.player_xp, 100, "a write is dropped")
	sm:user_set("customization", "cape", "user")
	E.eq(save.user.customization, "cape", "what isn't progression saves")
	-- As the mission end does: read the live table, change it in place, then save.
	local unlocks = sm:user_get("item_unlocks")
	unlocks[#unlocks + 1] = "reward"
	save.user.research_samples = 9
	sm:user_save()
	E.eq(table.concat(save.user.item_unlocks, ","), "owned", "the change in place goes back before the save")
	E.eq(save.user.research_samples, 50)
	E.eq(save.saves, 1, "the save still happens")
	E.has(e:log(), "guardrails: kept the account as it was at launch")
	E.eq(keys_shown(e):find("progress_frozen") ~= nil, true, "the player is told once")
	E.eq(rawget(_G, "obsidian").guardrails().freeze_progress, true)
end)

test("a_mod_cant_write_past_the_freeze", function()
	local _, save = engine("freeze_progress\t1\n")
	-- A mod's own hook on the save still ends in the loader's.
	local sm = rawget(_G, "SaveManager")
	local game = sm.user_set
	sm.user_set = function(self, k, v, ...) return game(self, k, v, ...) end
	sm:user_set("research_samples", 999, "user")
	E.eq(save.user.research_samples, 50)
end)

test("achievements_off_keeps_trophies_and_their_progress_but_not_the_rest", function()
	local e, save, trophies = engine("block_achievements\t1\n")
	local sm = rawget(_G, "SaveManager")
	rawget(_G, "UserManager"):unlock_trophy("first")
	E.eq(#trophies, 0)
	sm:user_set("trophy_unlock_progress", { kills = 900 }, "user")
	E.eq(save.user.trophy_unlock_progress.kills, 3, "no progress toward one either")
	sm:user_set("player_xp", 900, "user")
	E.eq(save.user.player_xp, 900, "XP still counts")
	E.eq(keys_shown(e):find("achievements_blocked") ~= nil, true)
	E.eq(keys_shown(e):find("progress_frozen"), nil)
end)

local SANDBOX = "sandbox\tabcd1234\n"
local WAR = "war\tk3x9q2\thttp://127.0.0.1:40123/0f1e2d3c4b5a69788796a5b4c3d2e1f0/1.0/\ta1b2c3d4e5f6\n"

test("a_sandbox_blocks_achievements_without_turning_the_guardrail_on", function()
	local e, _, trophies = engine("block_achievements\t0\n" .. SANDBOX)
	rawget(_G, "UserManager"):unlock_trophy("first")
	E.eq(#trophies, 0, "a Sandbox never earns one")
	E.eq(rawget(_G, "obsidian").guardrails().block_achievements, false, "the player's own choice")
	E.eq(keys_shown(e):find("achievements_blocked"), nil, "no notice for the Sandbox block")
end)

test("a_custom_war_that_allows_achievements_lifts_the_sandbox_block", function()
	local e, save, trophies = engine("block_achievements\t0\n" .. SANDBOX .. WAR .. "war_achievements\t1\n")
	rawget(_G, "UserManager"):unlock_trophy("first")
	E.eq(trophies[1], "first")
	rawget(_G, "SaveManager"):user_set("trophy_unlock_progress", { kills = 9 }, "user")
	E.eq(save.user.trophy_unlock_progress.kills, 9)
	E.eq(e:core().war_achievements, true)
end)

test("the_player_guardrail_still_blocks_in_a_war_that_allows_achievements", function()
	local e, _, trophies = engine("block_achievements\t1\n" .. SANDBOX .. WAR .. "war_achievements\t1\n")
	rawget(_G, "UserManager"):unlock_trophy("first")
	E.eq(#trophies, 0)
	E.eq(keys_shown(e):find("achievements_blocked") ~= nil, true)
end)

-- One boot per test: each runs in a process of its own.
local function trophies_with(text)
	local _, _, trophies = engine(text)
	rawget(_G, "UserManager"):unlock_trophy("first")
	return #trophies
end

test("war_achievements_without_a_war_line_counts_for_nothing", function()
	E.eq(trophies_with(SANDBOX .. "war_achievements\t1\n"), 0)
end)

test("a_war_that_does_not_allow_achievements_keeps_the_sandbox_block", function()
	E.eq(trophies_with(SANDBOX .. WAR .. "war_achievements\t0\n"), 0)
end)

test("a_war_line_from_an_older_obsidian_keeps_the_sandbox_block", function()
	E.eq(trophies_with(SANDBOX .. WAR), 0)
end)

test("a_war_line_without_a_sandbox_changes_nothing", function()
	E.eq(trophies_with("war_achievements\t1\n" .. WAR), 1, "the official war on the Official save, as usual")
end)

test("what_a_mod_reads_of_the_guardrails_cant_change_them", function()
	local e = engine("consent\t1\nfreeze_progress\t1\n")
	local g = rawget(_G, "obsidian").guardrails()
	E.eq(g.consent, true)
	g.freeze_progress = false
	E.eq(rawget(_G, "obsidian").guardrails().freeze_progress, true)
	E.eq(e:core().freeze_progress, true)
end)

return T
