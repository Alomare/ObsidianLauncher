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

-- The host's privacy, in the room data the engine's lobby answers: 1 public, 2 friends, 3 private.
local function privacy(value)
	rawset(_G, "GameSettings", { room_data_values = { "build_game_hash", "privacy" }, room_privacy_settings = { public = 1, friends = 2, private = 3 } })
	local network = rawget(_G, "NetworkGlobal")
	network.data_string_to_table = function(raw, keys)
		local out, i = {}, 0
		for v in string.gmatch(raw or "", "[^|]+") do
			i = i + 1
			out[keys[i]] = v
		end
		return out
	end
	network.lobby.lobby = { data = function(_, key) return key == "data" and value and ("3bd83d77c8ae|" .. value) or nil end }
end

test("a_friends_or_private_lobby_is_cleared_with_anyone", function()
	for _, kind in ipairs({ "2", "3" }) do
		local e = engine({ my_id = "me", members = { "me", "p2", "p3" }, data = { p3 = "1;" .. OTHER .. ";0" } })
		privacy(kind)
		e:frames(31)
		E.eq(cleared(), true, "privacy " .. kind .. ": a vanilla player and one who blocks")
		E.has(e:log(), "session: lobby privacy not public")
	end
end)

test("a_public_lobby_holds_session_mods_back", function()
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = {} })
	privacy("1")
	e:frames(31)
	E.eq(cleared(), false, "a vanilla player")
	E.has(e:log(), "session: lobby privacy public")
end)

test("a_lobby_whose_privacy_cant_be_read_counts_as_public", function()
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = {} })
	privacy(nil)
	e:frames(31)
	E.eq(cleared(), false)
	E.has(e:log(), "session: lobby privacy unknown, counted as public")
end)

test("a_mod_every_player_needs_stays_off_in_a_friends_lobby_without_it", function()
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OTHER .. ";1" } }, { every = { ["x.every"] = "abc" } })
	privacy("2")
	e:frames(31)
	E.eq(cleared(), true)
	E.eq(rawget(_G, "obsidian").session.cleared ~= nil, true)
	E.eq(require("obsidian/loader/session").missing["x.every"], true, "it needs every player, whatever the lobby")
end)

test("malformed_member_data_is_not_cleared", function()
	for _, bad in ipairs({ "1;zz;0", "2;" .. OWN .. ";0", "", "1;" .. OWN .. ";2", "1;" .. OWN }) do
		local e = engine({ my_id = "me", members = { "me", "p2" }, data = { p2 = bad } })
		e:frames(31)
		E.eq(cleared(), false, "data " .. bad)
	end
end)

test("nothing_about_a_players_progress_goes_to_the_lobby", function()
	local lobby = { my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OWN .. ";1" } }
	local e = engine(lobby)
	local core = e:core()
	core.current = { id = "holder" }
	rawget(_G, "obsidian").session.hold_progress()
	core.current = nil
	e:frames(31)
	E.eq(lobby.published.obsidian_forfeit, nil)
end)

-- The ship and a mission have the game's hint box. The main menu doesn't.
local function with_hint_box(e)
	local hints = {}
	rawset(_G, "HintSettings", {})
	rawset(_G, "LocalizationManager", { string = function(_, key) return "game:" .. key end })
	local box = { queued_hints = {} }
	box.add_hint = function(_, key, override, params, color, seconds)
		hints[#hints + 1] = { key = key, override = override, seconds = seconds }
		box.queued_hints[#box.queued_hints + 1] = { text = key }
	end
	hints.box = box
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

local function as_mod(e, id)
	e:core().current = { id = id, mod = {} }
	return rawget(_G, "obsidian")
end

test("a_mod_shows_its_words_in_the_games_hint_box", function()
	local e = engine(nil, NOTICE_TEXTS)
	local hints, toasts = with_hint_box(e)
	e:frames(1)
	E.eq(as_mod(e, "m").notify("Hold the line: 100%", 3), true)
	E.eq(#toasts, 0)
	E.eq(hints[1].override, true, "shown with the game's hints turned off")
	E.eq(hints[1].seconds, 3)
	local entry = rawget(_G, "HintSettings")[hints[1].key]
	E.eq(rawget(_G, "LocalizationManager"):string(entry.text), "Hold the line: 100%", "a % stays as written")
	E.eq(rawget(_G, "obsidian").capabilities.notify, true)
end)

test("a_mod_notification_is_a_toast_where_there_is_no_hint_box", function()
	local e = engine(nil, NOTICE_TEXTS)
	local hints, toasts = with_hint_box(e)
	rawget(_G, "Boot").state_machine.get_current_state = function() return {} end
	E.eq(as_mod(e, "m").notify("Hello"), true)
	E.eq(#hints, 0)
	E.eq(toasts[1], "Hello")
end)

test("a_mod_notification_is_checked", function()
	local e = engine(nil, NOTICE_TEXTS)
	with_hint_box(e)
	local o = as_mod(e, "m")
	E.eq(select(2, o.notify("")), "bad_type")
	E.eq(select(2, o.notify(5)), "bad_type")
	E.eq(select(2, o.notify("words", 0)), "out_of_range")
	E.eq(select(2, o.notify("words", "long")), "out_of_range")
end)

test("one_mod_cant_fill_the_queue", function()
	local e = engine(nil, NOTICE_TEXTS)
	local hints = with_hint_box(e)
	local o = as_mod(e, "noisy")
	for i = 1, 3 do E.eq(o.notify("line " .. i), true) end
	E.eq(select(2, o.notify("line 4")), "queue_full")
	E.eq(#hints, 3)
	E.eq(as_mod(e, "quiet").notify("mine"), true, "another mod still gets through")
	table.remove(hints.box.queued_hints, 1)
	E.eq(as_mod(e, "noisy").notify("line 5"), true, "room again once one has shown")
end)

test("the_loaders_notices_go_ahead_of_what_mods_queued", function()
	local e = engine(nil, NOTICE_TEXTS)
	local hints = with_hint_box(e)
	as_mod(e, "m").notify("a mod's line")
	e:core().notice("every_player_off", "Big")
	e:core().notice("every_player_off", "Small")
	e:frames(1)
	local settings, lm = rawget(_G, "HintSettings"), rawget(_G, "LocalizationManager")
	local order = {}
	for i, q in ipairs(hints.box.queued_hints) do order[i] = lm:string(settings[q.text].text) end
	E.eq(table.concat(order, " | "), "Obsidian: Big is off. | Obsidian: Small is off. | a mod's line")
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

test("a_hook_keeps_a_nil_result_as_one_value", function()
	rawset(_G, "Target", { get = function() return nil end, pair = function() return nil, 2, nil end })
	local e = engine({ my_id = "me", members = { "me" } }, { mods = { E.mod("c", { scope = "client" }) } }, {
		["obsidian/mods/c/main"] = "obsidian.hook(Target, 'get', function(original, ...) return original(...) end) obsidian.hook(Target, 'pair', function(original, ...) return original(...) end)",
	})
	e:frames(1)
	local target = rawget(_G, "Target")
	E.eq(select("#", target:get()), 1)
	E.eq(tonumber(target:get()), nil)
	E.eq(select("#", target:pair()), 3)
end)

test("a_wrap_runs_with_the_lobby_not_cleared_and_keeps_a_hooks_safety", function()
	rawset(_G, "Target", { run = function(_, x) return "vanilla " .. x end })
	local e = engine({ my_id = "me", members = { "me", "p2" }, data = { p2 = "1;" .. OTHER .. ";0" } },
		{ mods = { E.mod("s", { scope = "session" }) } },
		{ ["obsidian/mods/s/main"] = [[obsidian.wrap("Target", "run", function(original, self, x)
			if x == 2 then error("broken") end
			return "wrapped " .. original(self, x)
		end)]] })
	e:frames(31)
	E.eq(rawget(_G, "Target"):run(1), "wrapped vanilla 1")
	E.eq(rawget(_G, "Target"):run(2), "vanilla 2", "a failure falls back to the game's function")
	E.eq(rawget(_G, "obsidian").capabilities.wrap, true)
end)

test("merged_tables_come_out_the_same_in_any_load_order", function()
	local source = assert(io.open("core/data/merge_tables.lua", "rb")):read("*a")
	local function merge(names, tables)
		local env = setmetatable({ NAMES = names, require = function(name) return tables[name] end }, { __index = _G })
		return setfenv(assert(loadstring(source)), env)()
	end
	local one = { [1] = "game", [2] = "zed", game = 1, zed = 2 }
	local two = { [1] = "alpha", [2] = "game", alpha = 1, game = 2 }
	local t = { one = one, two = two }
	local a, b = merge({ "one", "two" }, t), merge({ "two", "one" }, t)
	for _, m in ipairs({ a, b }) do
		E.eq(m[1], "alpha")
		E.eq(m[2], "game")
		E.eq(m[3], "zed")
		E.eq(m.zed, 3, "the number each path has")
		E.eq(m.game, 2)
	end
	-- A plain list stays a list.
	local list = merge({ "x", "y" }, { x = { "b", "a" }, y = { "c", "a" } })
	E.eq(table.concat(list, ","), "a,b,c")
	E.eq(list.a, nil)
	-- A map keeps every key, and the later mod's value.
	local map = merge({ "x", "y" }, { x = { p = "one", q = { dynamic = "d" } }, y = { p = "two", r = "three" } })
	E.eq(map.p, "two")
	E.eq(map.q.dynamic, "d")
	E.eq(map.r, "three")
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

test("on_require_fires_at_once_for_a_module_already_loaded", function()
	local e = engine(nil, { mods = { E.mod("c") } }, { ["obsidian/mods/c/main"] = "LATE = function() obsidian.on_require('lua/thing', function(v) rawset(_G, 'GOT', v) end) end" })
	e.opts.modules["lua/thing"] = "value"
	rawget(_G, "require")("lua/thing")
	E.eq(rawget(_G, "GOT"), nil)
	-- Registered from code the game runs later, long after the script loaded.
	e:core().current = e:core().mods_by_id["c"]
	rawget(_G, "LATE")()
	e:core().current = nil
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

-- The Third Person Camera mod wraps require and, when the game loads a class, loads its own copy
-- that declares the same methods again on the same table.
local CAMERA = "local old = require; rawset(_G, 'require', function(n) local v = old(n); if n == 'lua/proxy' then old('hd2cam/lua/proxy') end; return v end)"

test("a_hook_comes_back_after_a_mod_declares_the_method_again", function()
	rawset(_G, "Target", { run = function() return "vanilla" end })
	local e = engine(nil, { mods = { E.mod("c", { scope = "client" }), E.mod("cam", { scope = "client" }) } }, {
		["obsidian/mods/c/main"] = "obsidian.hook(Target, 'run', function(original, self) return 'hooked ' .. original(self) end)",
		["obsidian/mods/cam/main"] = CAMERA,
		["hd2cam/lua/proxy"] = "Target.run = function() return 'camera' end",
	})
	E.eq(rawget(_G, "Target"):run(), "hooked vanilla")
	e.opts.modules["lua/proxy"] = true
	rawget(_G, "require")("lua/proxy")
	E.eq(rawget(_G, "Target"):run(), "hooked camera", "the hook wraps the new method")
	E.has(e:log(), "hook installed again: table.run by c")
end)

test("hooks_come_back_in_the_order_they_went_in", function()
	rawset(_G, "Target", { run = function() return "v" end })
	local e = engine(nil, { mods = { E.mod("a", { scope = "client" }), E.mod("b", { scope = "client" }) } }, {
		["obsidian/mods/a/main"] = "obsidian.hook(Target, 'run', function(original, self) return 'a(' .. original(self) .. ')' end)",
		["obsidian/mods/b/main"] = "obsidian.hook(Target, 'run', function(original, self) return 'b(' .. original(self) .. ')' end)",
		["lua/redeclare"] = "Target.run = function() return 'new' end",
	})
	E.eq(rawget(_G, "Target"):run(), "b(a(v))")
	rawget(_G, "require")("lua/redeclare")
	E.eq(rawget(_G, "Target"):run(), "b(a(new))")
end)

test("a_mod_that_wraps_a_hooked_method_doesnt_get_it_hooked_twice", function()
	local calls = 0
	rawset(_G, "Target", { run = function() return "v" end })
	rawset(_G, "COUNT", function() calls = calls + 1 end)
	local e = engine(nil, { mods = { E.mod("c", { scope = "client" }) } }, {
		["obsidian/mods/c/main"] = "obsidian.hook(Target, 'run', function(original, self) COUNT(); return original(self) end)",
		["lua/wrapper"] = "local old = Target.run; Target.run = function(self) return 'w ' .. old(self) end",
		["lua/stored"] = "local keep = { f = Target.run }; Target.run = function(self) return 's ' .. keep.f(self) end",
	})
	rawget(_G, "require")("lua/wrapper")
	rawget(_G, "require")("lua/stored")
	E.eq(rawget(_G, "Target"):run(), "s w v")
	E.eq(calls, 1, "the hook ran once")
	E.lacks(e:log(), "installed again")
end)

return T
