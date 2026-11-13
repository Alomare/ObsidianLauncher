-- Expansions: inert outside their content, started by the host once every player runs the same one,
-- followed by every machine, with the war, the pool and the door handled while the content runs.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local HASH, OTHER = "1111111111111111", "2222222222222222"
local TAG = "eeeeeeeeeeee"
local PAIR = "arcade:" .. HASH
local BASE = "3bd83d77c8ae"
local MODSET = string.rep("a", 64)
local OBSIDIAN = "1;" .. MODSET .. ";0"

local TEXTS = { en = {
	expansion_unknown_player = "unknown player",
	expansion_refused = "%s can't start: %s",
	expansion_turned_away = "%s turned away from %s",
	expansion_missing_here = "lobby plays %s",
	expansion_proving_grounds = "%s closed for %s",
	proving_grounds_blocked = "%s closed",
	campaign_not_uploaded_expansion = "not uploaded: %s",
} }

-- "lib" is a library only the expansion "arcade" requires; "skin" is a client mod that uses it.
local SOURCES = {
	["obsidian/mods/lib/main"] = [[
		obsidian.hook("Target", "lib_hook", function(original, self) return "lib:" .. original(self) end)
		obsidian.on_session(function(on) table.insert(rawget(_G, "LIB").events, on) end)
		rawget(_G, "LIB").scope = obsidian.scope()
		return obsidian.session.guard({ spawn = function() return "spawned" end, list = function() return "listed" end }, { "list" })
	]],
	["obsidian/mods/arcade/main"] = [[
		local a = rawget(_G, "ARCADE")
		a.api, a.scope = obsidian.import("lib"), obsidian.scope()
		obsidian.hook("Target", "spawn", function(original, self) return "arcade:" .. original(self) end)
		obsidian.wrap("Target", "draw", function() return "wrapped" end)
		obsidian.on_session(function(on) table.insert(a.events, on) end)
		obsidian.on_frame(function() a.frames = a.frames + 1 end)
	]],
	["obsidian/mods/second/main"] = "return true",
	["obsidian/mods/skin/main"] = [[rawget(_G, "SKIN").api = obsidian.import("lib")]],
}

local function deploy(over)
	local d = {
		expansions = { arcade = HASH, second = OTHER },
		hashes = { open = "aaaaaaaaaaaa", expansions = { arcade = TAG, second = "dddddddddddd" } },
		modset = MODSET,
		texts = TEXTS,
		mods = {
			E.mod("lib", { scope = "expansion", expansions = { "arcade" } }),
			E.mod("arcade", { scope = "expansion", expansions = { "arcade" }, requires = { "lib" }, name = { en = "Arcade" } }),
			E.mod("second", { scope = "expansion", expansions = { "second" } }),
			E.mod("skin", { scope = "client", requires = { "lib" } }),
		},
	}
	for k, v in pairs(over or {}) do d[k] = v end
	return E.deploy(d)
end

-- What the test reads from the mods and the game classes the loader hooks, set before boot.
local function globals()
	rawset(_G, "Target", { spawn = function() return "unit" end, draw = function() return "drawn" end, lib_hook = function() return "x" end })
	rawset(_G, "ARCADE", { events = {}, frames = 0 })
	rawset(_G, "LIB", { events = {} })
	rawset(_G, "SKIN", {})
	rawset(_G, "BUILD_GAME_HASH", BASE)
end

local function engine(lobby, over)
	globals()
	local e = E.new({ deploy = deploy(over), lobby = lobby, mods = SOURCES })
	e:boot()
	e:start_frames()
	return e
end

-- Runs fn as a mod and answers what fn answers.
local function as(e, id, fn, ...)
	local core = e:core()
	return select(2, core.call(core.mods_by_id[id], "test", fn, ...))
end

local function seq(list)
	local out = {}
	for i, v in ipairs(list) do out[i] = tostring(v) end
	return table.concat(out, ",")
end

local function x() return rawget(_G, "obsidian").expansion end
local function spawn() return rawget(_G, "Target"):spawn() end

local function notices(e, key)
	local out = {}
	for _, list in ipairs({ e:core().notices, e:core().notices_shown }) do
		for _, n in ipairs(list) do
			if n.key == key then out[#out + 1] = table.concat(n.args, "|") end
		end
	end
	return table.concat(out, ";")
end

-- This machine hosts p2, who runs both expansions, and the players given: { peer, obsidian, list }.
local function host_lobby(others)
	local l = { my_id = "me", members = { "me", "p2" }, data = { p2 = OBSIDIAN }, extra = { p2 = { obsidian_expansions = PAIR .. ",second:" .. OTHER } } }
	for _, d in ipairs(others or {}) do
		l.members[#l.members + 1] = d[1]
		l.data[d[1]] = d[2]
		l.extra[d[1]] = { obsidian_expansions = d[3] }
	end
	return l
end

-- The host's state, with what the game's join handlers use.
local function game_state(e)
	local st = { calls = {}, sent = {}, players = {}, kicked_peer_ids = {} }
	st.network_router = { transmit_to = function(_, peer, rpc, ...) st.sent[#st.sent + 1] = table.concat({ peer, rpc, ... }, " ") end }
	rawget(_G, "Boot").state_machine = { get_current_state = function() return st end }
	return st
end

local function state_class()
	local class = {}
	class.rpc_on_join_request = function(self, peer, game_hash) self.calls[#self.calls + 1] = "join " .. peer .. " " .. tostring(game_hash) end
	class.rpc_on_join_session_request = function(self, peer) self.calls[#self.calls + 1] = "session " .. peer end
	class.__index = class
	rawset(_G, "StateGame", class)
	return class
end

test("outside_the_content_an_expansion_is_inert_and_its_entry_points_work", function()
	local e = engine({ my_id = "me", members = { "me" }, data = {} })
	e:frames(31)
	E.eq(spawn(), "unit", "the hook calls only the game's function")
	E.eq(rawget(_G, "Target"):draw(), "wrapped", "a wrap keeps working")
	E.eq(rawget(_G, "ARCADE").frames, 0, "frame functions wait for the content")
	local api = rawget(_G, "ARCADE").api
	local ok, why = as(e, "arcade", function() return api.spawn() end)
	E.eq(ok, false)
	E.eq(why, "not_started")
	E.eq(as(e, "arcade", function() return api.list() end), "listed", "a free function stays free")
	E.eq(as(e, "arcade", function() return rawget(_G, "obsidian").session.cleared() end), false)
	E.eq(as(e, "arcade", function() return x().running() end), false)
	E.has(e:log(), "pool: vanilla", "at deploy it counts as a client mod")
end)

test("scope_and_info_of_an_expansion_and_its_library", function()
	local e = engine(nil)
	E.eq(rawget(_G, "ARCADE").scope, "expansion")
	E.eq(rawget(_G, "LIB").scope, "session", "a library sets up what the expansion may use")
	local info = rawget(_G, "obsidian").info
	E.eq(info("arcade").scope, "expansion")
	E.eq(info("lib").scope, "session")
	E.eq(info("skin").scope, "client")
	E.eq(e:core().capabilities.expansion, true)
end)

test("only_an_expansion_starts_content", function()
	local e = engine(nil)
	local ok, why = as(e, "skin", function() return x().start() end)
	E.eq(ok, false)
	E.eq(why, "not_expansion")
	ok, why = as(e, "lib", function() return x().start() end)
	E.eq(why, "not_expansion", "nor a library that serves one")
end)

test("a_session_secondary_scope_of_an_expansion_also_runs_in_a_cleared_lobby_outside_the_content", function()
	globals()
	local t = rawget(_G, "Target")
	t.shared = function() return "x" end
	t.hud = function() return "x" end
	local src = {}
	for k, v in pairs(SOURCES) do src[k] = v end
	src["obsidian/mods/arcade/main"] = [[
		obsidian.hook("Target", "shared", function(o, self) return "shared:" .. o(self) end, { secondary_scope = "shared" })
		obsidian.hook("Target", "hud", function(o, self) return "hud:" .. o(self) end, { secondary_scope = "hud" })
		obsidian.hook("Target", "spawn", function(o, self) return "arcade:" .. o(self) end)
		rawget(_G, "ARCADE").scope = obsidian.scope()
	]]
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = E.new({ lobby = lobby, mods = src, deploy = deploy({ has_session = true, mods = {
		E.mod("lib", { scope = "expansion", expansions = { "arcade" } }),
		E.mod("arcade", { scope = "expansion", expansions = { "arcade" }, requires = { "lib" }, name = { en = "Arcade" }, parts = {
			{ id = "shared", scope = "session", on = true, name = { en = "Shared" } },
			{ id = "hud", scope = "client", on = true, name = { en = "Hud" } },
		} }),
	} }) })
	e:boot()
	e:start_frames()
	e:frames(31)
	E.eq(rawget(_G, "ARCADE").scope, "expansion", "the primary scope stays")
	E.eq(t:shared(), "shared:x", "a cleared lobby, outside the content")
	E.eq(t:hud(), "hud:x")
	E.eq(spawn(), "unit", "untagged code still waits for the content")
	lobby.members = { "me", "p2" }
	e:frames(31)
	E.eq(t:shared(), "x", "off in a lobby that isn't cleared")
	E.eq(t:hud(), "hud:x", "a client secondary scope runs in any lobby")
	lobby.members = { "me" }
	e:frames(31)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	E.eq(spawn(), "arcade:unit", "everything runs in the content")
	E.eq(t:shared(), "shared:x")
end)

test("alone_the_content_starts_and_stops", function()
	local e = engine(nil)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	E.eq(spawn(), "arcade:unit")
	E.eq(as(e, "arcade", function() return rawget(_G, "ARCADE").api.spawn() end), "spawned", "full use of the library")
	E.eq(as(e, "arcade", function() return rawget(_G, "obsidian").session.cleared() end), true)
	E.eq(seq(rawget(_G, "ARCADE").events), "true")
	e:frames(2)
	E.eq(rawget(_G, "ARCADE").frames, 2)
	local ok, why = as(e, "second", function() return x().start() end)
	E.eq(ok, false)
	E.eq(why, "busy", "one content at a time")
	E.eq(as(e, "arcade", function() return x().start() end), true, "starting it again changes nothing")
	E.eq(as(e, "arcade", function() return x().stop() end), true)
	E.eq(spawn(), "unit")
	E.eq(seq(rawget(_G, "ARCADE").events), "true,false")
	ok, why = as(e, "arcade", function() return x().stop() end)
	E.eq(why, "not_running")
end)

test("the_host_starts_the_content_once_every_player_runs_it_and_moves_the_lobby_to_its_pool", function()
	local lobby = host_lobby()
	local e = engine(lobby)
	e:frames(30)
	E.eq(lobby.published.obsidian_expansions, PAIR .. ",second:" .. OTHER, "the list goes out, sorted")
	E.eq(lobby.published.obsidian_run, "")
	E.eq(as(e, "arcade", function() return x().start() end), true)
	E.eq(lobby.published.obsidian_run, PAIR)
	E.eq(lobby.room.build_game_hash, BASE .. TAG)
	E.eq(spawn(), "arcade:unit")
	E.eq(as(e, "arcade", function() return x().stop() end), true)
	E.eq(lobby.published.obsidian_run, "")
	E.eq(lobby.room.build_game_hash, BASE, "back to the pool of the deploy")
	E.eq(spawn(), "unit")
end)

test("a_start_with_players_who_lack_it_is_refused_with_their_names", function()
	local lobby = host_lobby({ { "p3", OBSIDIAN, "arcade:" .. OTHER }, { "p4" } })
	local e = engine(lobby)
	local st = game_state(e)
	st.players = { { player_go = { exists = function() return true end, owner = function() return "p3" end, get = function() return "Bob" end } } }
	e:frames(30)
	local ok, why, names = as(e, "arcade", function() return x().start() end)
	E.eq(ok, false)
	E.eq(why, "players_missing")
	E.eq(table.concat(names, ","), "Bob,unknown player", "another version counts as missing, and so does vanilla")
	E.eq(notices(e, "expansion_refused"), "Arcade|Bob, unknown player")
	E.eq(spawn(), "unit")
	E.eq(lobby.published.obsidian_run, "")
end)

test("another_player_than_the_host_can_not_start_or_stop_it", function()
	local lobby = { my_id = "me", host = "h", members = { "h", "me" }, data = { h = OBSIDIAN }, extra = { h = { obsidian_expansions = PAIR } } }
	local e = engine(lobby)
	e:frames(30)
	local ok, why = as(e, "arcade", function() return x().start() end)
	E.eq(why, "not_host")
	lobby.extra.h.obsidian_run = PAIR
	e:frames(30)
	ok, why = as(e, "arcade", function() return x().stop() end)
	E.eq(ok, false)
	E.eq(why, "not_host")
end)

test("every_machine_follows_the_host", function()
	local lobby = { my_id = "me", host = "h", members = { "h", "me" }, data = { h = OBSIDIAN }, extra = { h = { obsidian_run = PAIR } } }
	local e = engine(lobby)
	e:frames(30)
	E.eq(spawn(), "arcade:unit")
	E.eq(as(e, "arcade", function() return x().running() end), true)
	E.eq(seq(rawget(_G, "LIB").events), "true", "the library hears it too")
	E.eq(lobby.room.build_game_hash, nil, "only the host's lobby carries the pool")
	lobby.extra.h.obsidian_run = ""
	e:frames(30)
	E.eq(spawn(), "unit")
	E.eq(seq(rawget(_G, "ARCADE").events), "true,false")
	E.eq(seq(rawget(_G, "LIB").events), "true,false")
end)

test("a_player_whose_host_runs_content_missing_here_hears_why", function()
	local lobby = { my_id = "me", host = "h", members = { "h", "me" }, data = { h = OBSIDIAN }, extra = { h = { obsidian_run = "arcade:" .. OTHER } } }
	local e = engine(lobby)
	e:frames(60)
	E.eq(spawn(), "unit")
	E.eq(notices(e, "expansion_missing_here"), "arcade", "once")
end)

test("a_player_without_expansions_hears_it_too", function()
	globals()
	local lobby = { my_id = "me", host = "h", members = { "h", "me" }, data = { h = OBSIDIAN }, extra = { h = { obsidian_run = PAIR } } }
	local e = E.new({ deploy = E.deploy({ texts = TEXTS }), lobby = lobby })
	e:boot()
	e:start_frames()
	e:frames(30)
	E.eq(notices(e, "expansion_missing_here"), "arcade")
	E.eq(lobby.published.obsidian_expansions, nil, "nothing to publish")
end)

test("a_vanilla_player_entering_the_lobby_never_switches_running_content_off", function()
	local lobby = host_lobby()
	local e = engine(lobby, { has_session = true })
	e:frames(30)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	lobby.members[#lobby.members + 1] = "p9"
	e:frames(90)
	E.eq(e:core().session.state, false, "session mods switch off, as before")
	E.eq(spawn(), "arcade:unit")
	E.eq(as(e, "arcade", function() return rawget(_G, "ARCADE").api.spawn() end), "spawned")
	E.eq(seq(rawget(_G, "ARCADE").events), "true")
	E.eq(seq(rawget(_G, "LIB").events), "true")
end)

test("a_library_a_session_mod_also_needs_serves_the_expansion_in_any_lobby", function()
	local lobby = host_lobby({ { "p3" } })
	local mods = {
		E.mod("lib", { scope = "session", expansions = { "arcade" } }),
		E.mod("arcade", { scope = "expansion", expansions = { "arcade" }, requires = { "lib" }, name = { en = "Arcade" } }),
	}
	lobby.extra.p3 = { obsidian_expansions = PAIR }
	lobby.data.p3 = OBSIDIAN
	local e = engine(lobby, { has_session = true, mods = mods })
	e:frames(30)
	local api = rawget(_G, "ARCADE").api
	local ok, why = as(e, "arcade", function() return api.spawn() end)
	E.eq(why, "not_started", "outside the content, even in a cleared lobby")
	E.eq(rawget(_G, "Target"):lib_hook(), "lib:x", "the cleared session runs the library")
	lobby.data.p3 = "1;" .. string.rep("c", 64) .. ";0"
	e:frames(30)
	E.eq(rawget(_G, "Target"):lib_hook(), "x", "another set here: the library is off")
	E.eq(as(e, "arcade", function() return x().start() end), true)
	E.eq(rawget(_G, "Target"):lib_hook(), "lib:x", "on for the content")
	E.eq(as(e, "arcade", function() return api.spawn() end), "spawned")
end)

test("a_library_only_expansions_need_works_as_a_client_library_outside_their_content", function()
	local e = engine(nil)
	E.eq(rawget(_G, "Target"):lib_hook(), "lib:x", "never gated, as a client library")
	E.eq(as(e, "lib", function() return rawget(_G, "obsidian").session.cleared() end), true)
	local api = rawget(_G, "SKIN").api
	local ok, why = as(e, "skin", function() return api.spawn() end)
	E.eq(why, "needs_session")
	E.eq(as(e, "arcade", function() return x().start() end), true)
	ok, why = as(e, "skin", function() return api.spawn() end)
	E.eq(why, "needs_session", "the content gives the expansion full use, not a client mod")
	E.eq(seq(rawget(_G, "LIB").events), "true")
	as(e, "arcade", function() return x().stop() end)
	E.eq(seq(rawget(_G, "LIB").events), "true,false", "so the library can undo what changed")
end)

test("leaving_the_lobby_ends_the_content", function()
	local lobby = host_lobby()
	local e = engine(lobby)
	e:frames(30)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	rawget(_G, "NetworkGlobal").lobby.lobby = {}
	e:frames(30)
	E.eq(spawn(), "unit")
	E.has(e:log(), "content off, left the lobby")
end)

test("a_player_who_takes_over_the_lobby_carries_the_content_on", function()
	local lobby = { my_id = "me", host = "h", members = { "h", "me", "p3" }, data = { h = OBSIDIAN, p3 = OBSIDIAN },
		extra = { h = { obsidian_run = PAIR }, p3 = { obsidian_expansions = PAIR } } }
	local e = engine(lobby)
	e:frames(30)
	E.eq(spawn(), "arcade:unit")
	-- The host leaves and another player gets the lobby, who says nothing for a moment.
	lobby.host, lobby.members = "p3", { "me", "p3" }
	e:frames(60)
	E.eq(spawn(), "arcade:unit", "a new host gets a moment to say it still runs")
	lobby.extra.p3.obsidian_run = PAIR
	e:frames(90)
	E.eq(spawn(), "arcade:unit")
	E.eq(seq(rawget(_G, "ARCADE").events), "true", "no switch off and on")
	lobby.host = "me"
	e:frames(30)
	E.eq(spawn(), "arcade:unit")
	E.eq(lobby.published.obsidian_run, PAIR, "this game publishes it now")
	E.eq(lobby.room.build_game_hash, BASE .. TAG)
end)

test("a_new_host_that_never_says_it_runs_the_content_ends_it", function()
	local lobby = { my_id = "me", host = "h", members = { "h", "me", "p3" }, data = { h = OBSIDIAN, p3 = OBSIDIAN }, extra = { h = { obsidian_run = PAIR } } }
	local e = engine(lobby)
	e:frames(30)
	lobby.host = "p3"
	e:frames(120)
	E.eq(spawn(), "unit")
end)

test("a_joiner_without_the_expansion_is_turned_away_at_the_join_request", function()
	local class = state_class()
	local lobby = host_lobby({ { "p5", OBSIDIAN, "second:" .. OTHER }, { "p6" }, { "p7" } })
	local e = engine(lobby)
	local st = setmetatable(game_state(e), class)
	e:frames(30)
	-- Before the content, the game decides.
	st:rpc_on_join_request("p5", BASE, "engine", "Eve", 1, false)
	E.eq(st.calls[1], "join p5 " .. BASE)
	st.calls = {}
	-- p5..p7 aren't in the lobby when the content starts: they arrive afterwards.
	lobby.members = { "me", "p2" }
	E.eq(as(e, "arcade", function() return x().start() end), true)
	lobby.members = { "me", "p2", "p5", "p6", "p7" }
	st:rpc_on_join_request("p5", BASE, "engine", "Eve", 1, false)
	E.eq(#st.calls, 0)
	E.eq(st.sent[1], "p5 rpc_on_join_denied network_join_denied")
	E.eq(notices(e, "expansion_turned_away"), "Eve|Arcade")
	-- A player from another pool of the same build who runs it gets the host's hash.
	st:rpc_on_join_request("p2", BASE .. "aaaaaaaaaaaa", "engine", "Pat", 1, false)
	E.eq(st.calls[1], "join p2 " .. BASE)
	-- Nothing from the joiner yet: the request waits for it.
	st:rpc_on_join_request("p6", BASE, "engine", "Sam", 1, false)
	st:rpc_on_join_request("p7", BASE, "engine", "Kim", 1, false)
	E.eq(#st.calls, 1)
	E.eq(#st.sent, 1)
	lobby.data.p6, lobby.extra.p6.obsidian_expansions = OBSIDIAN, PAIR
	e:frames(1)
	E.eq(st.calls[2], "join p6 " .. BASE, "in once the member data shows it")
	local expansion = e.loaded["obsidian/loader/expansion"]
	local now = expansion.now()
	expansion.now = function() return now + 10 end
	e:frames(1)
	E.eq(st.sent[2], "p7 rpc_on_join_denied network_join_denied", "turned away when nothing comes in time")
	E.eq(#st.calls, 2)
end)

test("held_join_requests_go_to_the_game_when_the_content_ends", function()
	local class = state_class()
	local lobby = host_lobby()
	local e = engine(lobby)
	local st = setmetatable(game_state(e), class)
	e:frames(30)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	st:rpc_on_join_request("p8", BASE, "engine", "Lee", 1, false)
	E.eq(#st.calls, 0)
	as(e, "arcade", function() return x().stop() end)
	E.eq(st.calls[1], "join p8 " .. BASE)
end)

test("a_session_request_from_a_player_without_it_is_denied_and_the_slot_freed", function()
	local class = state_class()
	local lobby = host_lobby({ { "p5", OBSIDIAN } })
	lobby.members = { "me", "p2" }
	local e = engine(lobby)
	local st = setmetatable(game_state(e), class)
	st.players = { { reserved_info = { peer_id = "p5" } }, { reserved_info = { peer_id = "p2" } } }
	e:frames(30)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	st:rpc_on_join_session_request("p5", "game", 7, "Eve", 1)
	E.eq(st.sent[1], "p5 rpc_on_join_session_denied network_join_session_denied")
	E.eq(st.players[1].reserved_info, nil)
	E.eq(#st.calls, 0)
	st:rpc_on_join_session_request("p2", "game", 7, "Pat", 1)
	E.eq(st.calls[1], "session p2")
	E.eq(st.players[2].reserved_info.peer_id, "p2")
end)

test("a_player_in_the_mission_without_it_is_removed_by_the_games_kick", function()
	local lobby = host_lobby()
	local e = engine(lobby)
	local st = game_state(e)
	st.network_session_handler = { is_peer_in_session = function(_, peer) return peer == "p2" or peer == "p5" end }
	e:frames(30)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	lobby.members[#lobby.members + 1] = "p5"
	lobby.data.p5 = OBSIDIAN
	e:frames(60)
	E.eq(#st.sent, 0, "not at the first check")
	e:frames(30)
	E.eq(st.sent[1], "p5 rpc_on_kicked p5")
	E.eq(st.kicked_peer_ids[1].peer_id, "p5")
	E.eq(#st.kicked_peer_ids, 1, "p2 runs it")
	e:frames(60)
	E.eq(#st.sent, 1, "once")
	E.eq(spawn(), "arcade:unit")
end)

-- The parts of the game the Galactic War block touches.
local function war(e)
	local cm = { sent = 0 }
	cm.upload_mission_stats = function(self) self.sent = self.sent + 1 end
	rawset(_G, "CampaignManager", cm)
	local save = { common = { mission_stats_to_upload = { 1 } } }
	rawset(_G, "SaveManager", { set = function(_, k, v) save.common[k] = v end, user_set = function() end, save = function() end, user_save = function() end })
	rawset(_G, "UserManager", { get_user_id = function() return "user" end })
	rawset(_G, "StateMissionStart", { on_enter = function() end })
	rawset(_G, "LocalizationManager", { string = function(_, key) return "game:" .. key end })
	for _, name in ipairs({ "lua/managers/campaign_manager", "foundation/lua/managers/save_manager", "lua/states/state_mission_start" }) do
		e.opts.modules[name] = true
		rawget(_G, "require")(name)
	end
	e:frames(1)
	return cm, save
end

test("galactic_war_results_stay_home_for_the_mission_the_content_ran_in", function()
	local e = engine(nil)
	local cm, save = war(e)
	local lm = rawget(_G, "LocalizationManager")
	cm:upload_mission_stats({})
	E.eq(cm.sent, 1, "outside the content the war counts")
	E.eq(lm:string("mission_end_galactic_campaign_progress"), "game:mission_end_galactic_campaign_progress")
	E.eq(as(e, "arcade", function() return x().start() end), true)
	cm:upload_mission_stats({})
	E.eq(cm.sent, 1)
	E.eq(lm:string("mission_end_galactic_campaign_progress"), "not uploaded: Arcade")
	as(e, "arcade", function() return x().stop() end)
	cm:upload_mission_stats({})
	E.eq(cm.sent, 1, "the results wait on the ship after the content ends")
	E.eq(lm:string("mission_end_galactic_campaign_no_progress"), "not uploaded: Arcade")
	rawget(_G, "StateMissionStart"):on_enter()
	E.eq(save.common.mission_stats_to_upload, nil, "what the content left is dropped")
	cm:upload_mission_stats({})
	E.eq(cm.sent, 2, "the next mission counts")
	E.eq(lm:string("mission_end_galactic_campaign_progress"), "game:mission_end_galactic_campaign_progress")
end)

test("a_mission_that_starts_with_the_content_on_stays_off_the_war", function()
	local e = engine(nil)
	local cm = war(e)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	rawget(_G, "StateMissionStart"):on_enter()
	cm:upload_mission_stats({})
	E.eq(cm.sent, 0)
end)

test("proving_grounds_stay_open_outside_the_content", function()
	local shown = {}
	globals()
	rawset(_G, "MenuScreenCampaign", { show_state = function(_, name) shown[#shown + 1] = name end })
	rawset(_G, "LocalizationManager", { string = function() return "Proving Grounds" end })
	local e = E.new({ deploy = deploy(), mods = SOURCES })
	e:boot()
	e:start_frames()
	local screen = rawget(_G, "MenuScreenCampaign")
	screen:show_state("provinggrounds_selection")
	E.eq(#shown, 1)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	screen:show_state("provinggrounds_selection")
	E.eq(#shown, 1, "closed while the content runs")
	E.eq(notices(e, "expansion_proving_grounds"), "Proving Grounds|Arcade")
	as(e, "arcade", function() return x().stop() end)
	screen:show_state("provinggrounds_selection")
	E.eq(#shown, 2)
end)

test("the_lobby_browser_lists_rooms_playing_the_same_expansion", function()
	local added = {}
	globals()
	rawset(_G, "BUILD_GAME_HASH", BASE .. "aaaaaaaaaaaa")
	rawset(_G, "MenuScreenCampaignMultiplayer", { add_room = function(_, room) added[#added + 1] = room.build_game_hash end })
	local e = E.new({ deploy = deploy(), mods = SOURCES })
	e:boot()
	e:start_frames()
	local browser = rawget(_G, "MenuScreenCampaignMultiplayer")
	browser:add_room({ build_game_hash = BASE .. TAG })
	browser:add_room({ build_game_hash = BASE .. "ffffffffffff" })
	E.eq(added[1], BASE .. "aaaaaaaaaaaa", "the same expansion at the same version")
	E.eq(added[2], BASE .. "ffffffffffff", "another one stays out")
end)

test("the_room_hash_holds_while_the_game_writes_it_again", function()
	globals()
	local written = {}
	rawset(_G, "NetworkRoom", { set_data = function(_, key, value) written[key] = value end })
	local lobby = host_lobby()
	local e = E.new({ deploy = deploy(), lobby = lobby, mods = SOURCES })
	e:boot()
	e:start_frames()
	local room = rawget(_G, "NetworkRoom")
	room:set_data("build_game_hash", BASE)
	E.eq(written.build_game_hash, BASE)
	e:frames(30)
	E.eq(as(e, "arcade", function() return x().start() end), true)
	room:set_data("build_game_hash", BASE)
	E.eq(written.build_game_hash, BASE .. TAG)
	room:set_data("region", "eu")
	E.eq(written.region, "eu")
	as(e, "arcade", function() return x().stop() end)
	room:set_data("build_game_hash", BASE)
	E.eq(written.build_game_hash, BASE)
end)

return T
