local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local OWN = string.rep("a", 64)
local OTHER = string.rep("c", 64)

local function engine(lobby, deploy_over, mods)
	local d = { has_session = true, modset = OWN, mods = {} }
	for k, v in pairs(deploy_over or {}) do d[k] = v end
	local e = E.new({ deploy = E.deploy(d), lobby = lobby, mods = mods })
	e:boot()
	e:start_frames()
	return e
end

local function cleared()
	return rawget(_G, "obsidian").session.cleared()
end

test("a_session_check_that_fails_doesnt_stay_cleared", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	E.eq(cleared(), true)
	rawget(_G, "NetworkGlobal").lobby.members = function() error("no lobby data") end
	e:frames(31)
	E.eq(cleared(), false, "not knowing who's here isn't cleared")
end)

test("a_hook_that_fails_after_the_game_ran_doesnt_run_it_twice", function()
	local spawned = 0
	rawset(_G, "Target", { spawn = function() spawned = spawned + 1; return "unit" end })
	engine(nil, { mods = { E.mod("c", { scope = "client" }) } }, {
		["obsidian/mods/c/main"] = "obsidian.hook(Target, 'spawn', function(original, self) original(self); error('after the call') end)",
	})
	E.eq(rawget(_G, "Target"):spawn(), "unit", "the game's result still comes back")
	E.eq(spawned, 1, "spawned once")
end)

test("a_game_function_that_throws_inside_a_hook_runs_once", function()
	local calls = 0
	rawset(_G, "Target", { spawn = function() calls = calls + 1; error("the game's own error") end })
	engine(nil, { mods = { E.mod("c", { scope = "client" }) } }, {
		["obsidian/mods/c/main"] = "obsidian.hook(Target, 'spawn', function(original, self) return original(self) end)",
	})
	E.eq(pcall(rawget(_G, "Target").spawn, rawget(_G, "Target")), true, "the hook takes the error")
	E.eq(calls, 1)
end)

test("a_hook_that_fails_before_the_game_ran_still_runs_it", function()
	local spawned = 0
	rawset(_G, "Target", { spawn = function() spawned = spawned + 1; return "unit" end })
	engine(nil, { mods = { E.mod("c", { scope = "client" }) } }, {
		["obsidian/mods/c/main"] = "obsidian.hook(Target, 'spawn', function() error('first thing') end)",
	})
	E.eq(rawget(_G, "Target"):spawn(), "unit")
	E.eq(spawned, 1)
end)

test("solo_is_cleared", function()
	engine({ my_id = "me", members = { "me" }, data = {} })
	E.eq(cleared(), true)
end)

test("no_lobby_is_cleared", function()
	local e = engine(nil)
	e:frames(31)
	E.eq(cleared(), true)
end)

test("identical_hashes_are_cleared", function()
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OWN .. ";0" } })
	e:frames(31)
	E.eq(cleared(), true)
end)

test("one_different_hash_is_not_cleared", function()
	local e = engine({ my_id = "me", members = { "me", "p2", "p3" }, data = { p2 = "1;" .. OWN .. ";0", p3 = "1;" .. OTHER .. ";0" } })
	e:frames(31)
	E.eq(cleared(), false)
end)

test("others_consenting_is_cleared_without_own_consent", function()
	-- p2 lets anyone change the game; mine stays closed. My mods run; theirs would not.
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OTHER .. ";1" } })
	e:frames(31)
	E.eq(cleared(), true)
end)

test("identical_or_consenting_mix_is_cleared", function()
	local e = engine({ my_id = "me", members = { "me", "p2", "p3" }, data = { p2 = "1;" .. OWN .. ";0", p3 = "1;" .. OTHER .. ";1" } })
	e:frames(31)
	E.eq(cleared(), true)
end)

test("own_consent_does_not_clear_my_mods", function()
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OTHER .. ";0" } })
	e:core().consent_on = true
	e:frames(31)
	E.eq(cleared(), false, "p2 runs another set and does not consent")
end)

test("one_vanilla_member_is_not_cleared", function()
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = {} })
	e:frames(31)
	E.eq(cleared(), false)
end)

test("malformed_member_data_is_not_cleared", function()
	for _, bad in ipairs({ "1;zz;0", "2;" .. OWN .. ";0", "", "1;" .. OWN .. ";2", "1;" .. OWN }) do
		local e = engine({ my_id = "me", members = { "me", "p2" }, data = { p2 = bad } })
		e:frames(31)
		E.eq(cleared(), false, "data " .. bad)
	end
end)

test("a_forfeit_reaches_everyone_in_the_mission", function()
	local lobby = { my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OWN .. ";1" }, extra = {} }
	local e = engine(lobby)
	local session = e.loaded["obsidian/loader/session"]
	local state = "game"
	rawset(_G, "Boot", { state_machine = { get_current_state_name = function() return state end } })
	e:frames(31)
	E.eq(lobby.published.obsidian_forfeit, "0")
	E.eq(session.forfeited, false)
	lobby.extra.p2 = { obsidian_forfeit = "1" }
	e:frames(31)
	E.eq(session.forfeited, true, "the host's mod changed this player's mission")
	e:frames(31)
	E.eq(lobby.published.obsidian_forfeit, "1", "and the word goes on")
end)

test("a_holding_mod_tells_the_lobby_while_it_runs", function()
	local lobby = { my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OWN .. ";1" } }
	local e = engine(lobby)
	local core = e:core()
	local mod = { id = "holder" }
	core.current = mod
	rawget(_G, "obsidian").session.hold_progress()
	core.current = nil
	e:frames(31)
	E.eq(lobby.published.obsidian_forfeit, "1", "before any mission")
	mod.off = true
	e:frames(31)
	E.eq(lobby.published.obsidian_forfeit, "0", "a mod that failed holds nothing")
end)

test("a_holder_stops_holding_beside_players_who_havent_agreed", function()
	local lobby = { my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. string.rep("b", 64) .. ";0" } }
	local e = engine(lobby)
	local core = e:core()
	core.current = { id = "holder" }
	rawget(_G, "obsidian").session.hold_progress()
	core.current = nil
	e:frames(31)
	local session = e.loaded["obsidian/loader/session"]
	E.eq(session.mission_held(), false, "the mod is off, so the mission counts")
end)

test("a_peers_forfeit_doesnt_hold_the_ship", function()
	local lobby = { my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OWN .. ";1" }, extra = { p2 = { obsidian_forfeit = "1" } } }
	local e = engine(lobby)
	rawset(_G, "Boot", { state_machine = { get_current_state_name = function() return "menu" end } })
	e:frames(31)
	E.eq(e.loaded["obsidian/loader/session"].forfeited, false, "their mission is over, and the Armory is in use")
end)

-- The ship and a mission have the game's hint box. The main menu doesn't.
local function with_hint_box(e)
	local hints = {}
	rawset(_G, "HintSettings", {})
	rawset(_G, "LocalizationManager", { string = function(_, key) return "game:" .. key end })
	local box = { add_hint = function(_, key, override, params, color, seconds) hints[#hints + 1] = { key = key, override = override, seconds = seconds } end }
	rawget(_G, "Boot").state_machine = { get_current_state = function() return { hint_manager = box } end, get_current_state_name = function() return "menu" end }
	local toasts, menu = {}, e.loaded["obsidian/loader/menu"]
	menu.toast = function(text) toasts[#toasts + 1] = text end
	return hints, toasts
end

local NOTICE_TEXTS = { texts = { en = { every_player_off = "Obsidian: %s is off." } } }

test("a_notice_shows_in_the_games_hint_box", function()
	local e = engine(nil, NOTICE_TEXTS)
	local hints, toasts = with_hint_box(e)
	e:core().notice("every_player_off", "Big")
	e:frames(1)
	E.eq(#hints, 1)
	E.eq(#toasts, 0, "no toast as well")
	E.eq(hints[1].override, true, "shown with the game's hints turned off")
	local entry = rawget(_G, "HintSettings")[hints[1].key]
	E.eq(entry.icon, "icon_hint")
	E.eq(entry.audio, "ui_select")
	E.eq(rawget(_G, "LocalizationManager"):string(entry.text), "Obsidian: Big is off.")
	E.eq(rawget(_G, "LocalizationManager"):string("other"), "game:other", "other texts")
end)

-- The hint box leaves out a text it already has, so a repeat can't queue up, but another mod's can.
test("only_the_same_notice_shares_a_hint", function()
	local e = engine(nil, NOTICE_TEXTS)
	local hints = with_hint_box(e)
	e:core().notice("every_player_off", "Big")
	e:core().notice("every_player_off", "Big")
	e:core().notice("every_player_off", "Small")
	e:frames(1)
	E.eq(hints[1].key, hints[2].key)
	E.eq(hints[1].key == hints[3].key, false)
	local settings, lm = rawget(_G, "HintSettings"), rawget(_G, "LocalizationManager")
	E.eq(lm:string(settings[hints[1].key].text), "Obsidian: Big is off.")
	E.eq(lm:string(settings[hints[3].key].text), "Obsidian: Small is off.")
end)

test("a_notice_is_a_toast_where_there_is_no_hint_box", function()
	local e = engine(nil, NOTICE_TEXTS)
	local hints, toasts = with_hint_box(e)
	rawget(_G, "Boot").state_machine.get_current_state = function() return {} end
	e:core().notice("every_player_off", "Big")
	e:frames(1)
	E.eq(#hints, 0)
	E.eq(toasts[1], "Obsidian: Big is off.")
end)

test("a_hint_box_that_fails_falls_back_to_a_toast", function()
	local e = engine(nil, NOTICE_TEXTS)
	local _, toasts = with_hint_box(e)
	rawget(_G, "Boot").state_machine.get_current_state = function() return { hint_manager = { add_hint = function() error("no") end } } end
	e:core().notice("every_player_off", "Big")
	e:frames(1)
	E.eq(toasts[1], "Obsidian: Big is off.")
end)

test("own_member_data_is_published", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	E.eq(lobby.published.obsidian, "1;" .. OWN .. ";0")
end)

-- The game keeps one NetworkRoom when a player leaves their own lobby to join another, and swaps
-- the engine's lobby inside it. Member data belongs to that one.
test("joining_another_lobby_publishes_in_that_lobby", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	local room = rawget(_G, "NetworkGlobal").lobby
	room.lobby, lobby.members, lobby.published = nil, {}, {}
	e:frames(31)
	E.eq(next(lobby.published), nil, "no lobby to write to while between the two")
	E.eq(cleared(), true)
	room.lobby, lobby.members = {}, { "me", "host" }
	lobby.data.host = "1;" .. OWN .. ";0"
	e:frames(31)
	E.eq(lobby.published.obsidian, "1;" .. OWN .. ";0")
	E.eq(lobby.published.obsidian_every, "")
	E.eq(lobby.published.obsidian_forfeit, "0")
	E.eq(cleared(), true)
end)

test("member_data_is_written_again_once_the_lobby_is_joined", function()
	local lobby = { my_id = "me", members = { "me" }, data = {}, state = "joining" }
	local e = engine(lobby)
	e:frames(31)
	lobby.published, lobby.state = {}, "joined"
	e:frames(31)
	E.eq(lobby.published.obsidian, "1;" .. OWN .. ";0")
end)

local HOOK_MODS = {
	["obsidian/mods/s/main"] = "obsidian.hook('Target', 'run', function(original, self, x) return 'session ' .. x end)",
	["obsidian/mods/c/main"] = "obsidian.hook(Target, 'look', function(original, self) return 'client' end)",
}

local function hook_engine(data)
	rawset(_G, "Target", { run = function(self, x) return "vanilla " .. x end, look = function() return "vanilla" end })
	return engine({ my_id = "me", members = { "me", "p2" }, data = data },
		{ mods = { E.mod("s", { scope = "session" }), E.mod("c", { scope = "client" }) } }, HOOK_MODS)
end

test("session_hook_calls_original_when_not_cleared", function()
	local e = hook_engine({ p2 = "1;" .. OTHER .. ";0" })
	e:frames(31)
	E.has(e:log(), "started s")
	E.eq(rawget(_G, "Target"):run(1), "vanilla 1")
end)

test("session_hook_runs_when_cleared", function()
	local e = hook_engine({ p2 = "1;" .. OWN .. ";0" })
	e:frames(31)
	E.eq(rawget(_G, "Target"):run(1), "session 1")
end)

test("client_hook_always_runs", function()
	local e = hook_engine({})
	e:frames(31)
	E.eq(rawget(_G, "Target"):look(), "client")
end)

test("hook_on_undeclared_scope_is_gated", function()
	rawset(_G, "Target", { run = function() return "vanilla" end })
	local undeclared = E.mod("u")
	undeclared.scope = nil
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = {} },
		{ mods = { undeclared } },
		{ ["obsidian/mods/u/main"] = "obsidian.hook(Target, 'run', function() return 'modded' end)" })
	e:frames(31)
	E.has(e:log(), "started u")
	E.eq(rawget(_G, "Target"):run(), "vanilla")
end)

test("hook_by_global_name_resolves_later", function()
	local e = engine(nil, { mods = { E.mod("c") } },
		{ ["obsidian/mods/c/main"] = "obsidian.hook('LaterClass', 'go', function() return 'hooked' end)" })
	rawset(_G, "LaterClass", { go = function() return "vanilla" end })
	e.opts.modules["lua/later"] = true
	rawget(_G, "require")("lua/later")
	E.eq(rawget(_G, "LaterClass").go(), "hooked")
end)

test("on_require_fires_after_module_loads", function()
	local e = engine(nil, { mods = { E.mod("c") } },
		{ ["obsidian/mods/c/main"] = "obsidian.on_require('lua/thing', function(v) rawset(_G, 'GOT', v) end)" })
	e.opts.modules["lua/thing"] = "value"
	rawget(_G, "require")("lua/thing")
	E.eq(rawget(_G, "GOT"), "value")
end)

test("on_session_fires_on_change_only", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby, { mods = { E.mod("s", { scope = "session" }) } },
		{ ["obsidian/mods/s/main"] = "CALLS = ''; obsidian.on_session(function(on) rawset(_G, 'CALLS', CALLS .. tostring(on) .. ',') end)" })
	e:frames(31)
	lobby.members = { "me", "p2" }
	e:frames(60)
	lobby.members = { "me" }
	e:frames(30)
	E.eq(rawget(_G, "CALLS"), "false,true,")
end)

test("static_session_content_warns_once", function()
	local lobby = { my_id = "me", members = { "me", "p2" }, data = {} }
	local e = engine(lobby, { has_static_session = true })
	e:frames(90)
	local n = 0
	for _, x in ipairs(e:core().notices_shown) do
		if x.key == "session_not_cleared" then n = n + 1 end
	end
	E.eq(n, 1)
	E.has(e:log(), "notice: session_not_cleared")
end)

test("frame_hook_runs_after_the_game_update", function()
	local e = engine(nil)
	e:frames(5)
	E.eq(rawget(_G, "FRAMES"), 5)
	E.eq(e:core().frame_count, 5)
end)

test("publishing_member_data_is_logged_once_per_lobby", function()
	local e = engine({ my_id = "me", members = { "me" }, data = {} })
	e:frames(91)
	E.eq(E.count(e:log(), "session: published member data"), 1)
end)


local BIG = "1111111111111111"

local EVERY_MODS = {
	["obsidian/mods/big/main"] = "CALLS = ''; obsidian.on_session(function(on) rawset(_G, 'CALLS', CALLS .. tostring(on) .. ',') end); obsidian.hook('Target', 'size', function() return 16 end)",
	["obsidian/mods/s/main"] = "obsidian.hook('Target', 'run', function(original, self, x) return 'session ' .. x end)",
}

-- "big" is a mod every player has to run; "s" is an ordinary session mod.
local function every_engine(data, extra)
	rawset(_G, "Target", { size = function() return 4 end, run = function(self, x) return "vanilla " .. x end })
	local lobby = { my_id = "me", members = { "me", "p2" }, data = data, extra = extra or {} }
	local e = engine(lobby, { every = { big = BIG }, mods = { E.mod("big", { scope = "session" }), E.mod("s", { scope = "session" }) } }, EVERY_MODS)
	return e, lobby
end

local function cleared_for(e, id)
	local core = e:core()
	core.current = core.mods_by_id[id]
	local on = rawget(_G, "obsidian").session.cleared()
	core.current = nil
	return on
end

test("a_mod_every_player_needs_stays_off_beside_a_player_who_only_agrees", function()
	local e, lobby = every_engine({ p2 = "1;" .. OTHER .. ";1" })
	e:frames(31)
	E.eq(cleared(), true, "the session is cleared by p2's consent")
	E.eq(rawget(_G, "Target"):run(1), "session 1", "so the ordinary session mod runs")
	E.eq(rawget(_G, "Target"):size(), 4, "but p2 doesn't run big")
	E.eq(cleared_for(e, "big"), false)
	E.eq(cleared_for(e, "s"), true)
	E.has(e:log(), "session: big off, a player here doesn't run it")
	lobby.extra.p2 = { obsidian_every = "other:2222222222222222,big:" .. BIG }
	e:frames(31)
	E.eq(rawget(_G, "Target"):size(), 16, "p2 runs it now")
	E.eq(cleared_for(e, "big"), true)
	E.has(e:log(), "session: every player here runs big")
end)

test("a_mod_every_player_needs_wants_the_same_version_and_options", function()
	local e = every_engine({ p2 = "1;" .. OTHER .. ";1" }, { p2 = { obsidian_every = "big:2222222222222222" } })
	e:frames(31)
	E.eq(rawget(_G, "Target"):size(), 4)
end)

test("a_mod_every_player_needs_runs_alone", function()
	rawset(_G, "Target", { size = function() return 4 end })
	engine({ my_id = "me", members = { "me" }, data = {} }, { every = { big = BIG }, mods = { E.mod("big", { scope = "session" }) } },
		{ ["obsidian/mods/big/main"] = "obsidian.hook('Target', 'size', function() return 16 end)" })
	E.eq(rawget(_G, "Target"):size(), 16)
end)

test("a_mod_every_player_needs_is_published", function()
	local e, lobby = every_engine({ p2 = "1;" .. OWN .. ";0" }, { p2 = { obsidian_every = "big:" .. BIG } })
	e:frames(31)
	E.eq(lobby.published.obsidian_every, "big:" .. BIG)
	E.eq(rawget(_G, "Target"):size(), 16, "the same set, with big in it")
end)

test("without_such_mods_an_empty_list_is_published", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	E.eq(lobby.published.obsidian_every, "")
end)

test("on_session_tells_a_mod_every_player_needs_about_its_own_switch", function()
	local e, lobby = every_engine({ p2 = "1;" .. OTHER .. ";1" })
	e:frames(31)
	lobby.extra.p2 = { obsidian_every = "big:" .. BIG }
	e:frames(31)
	lobby.extra.p2 = {}
	e:frames(31)
	E.eq(rawget(_G, "CALLS"), "false,true,false,")
end)

test("a_player_told_when_such_a_mod_switches_off", function()
	local e = every_engine({ p2 = "1;" .. OTHER .. ";1" })
	e:frames(91)
	local n = 0
	for _, x in ipairs(e:core().notices_shown) do
		if x.key == "every_player_off" then n = n + 1; E.eq(x.args[1], "big") end
	end
	E.eq(n, 1, "once, not every check")
end)

test("a_library_guard_refuses_while_such_a_mod_is_off", function()
	local e = every_engine({ p2 = "1;" .. OTHER .. ";1" })
	e:frames(31)
	local core = e:core()
	core.current = core.mods_by_id.big
	local view = rawget(_G, "obsidian").session.guard({ grow = function() return "grown" end }, {})
	core.current = nil
	E.eq(select(2, view.grow()), "not_cleared")
end)

return T
