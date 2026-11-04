local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local function game_hash()
	local _, hash = string.match(rawget(_G, "BUILD_REVISION_INFORMATION"), "parent: ([0-9]+):([0-9abcdef]+)")
	return hash
end

test("no_session_keeps_vanilla_hash", function()
	E.new()
	require("obsidian/loader/pool").apply(E.deploy({ has_session = false }))
	E.eq(game_hash(), "3bd83d77c8ae")
end)

test("session_mods_keep_the_vanilla_hash", function()
	for _, static in ipairs({ false, true }) do
		E.new()
		require("obsidian/loader/pool").apply(E.deploy({ has_session = true, has_static_session = static }))
		E.eq(game_hash(), "3bd83d77c8ae", "vanilla friends can join, static " .. tostring(static))
	end
end)

test("mods_every_player_needs_stay_in_the_vanilla_pool", function()
	E.new()
	require("obsidian/loader/pool").apply(E.deploy({ has_session = true, hashes = { open = "aaaaaaaaaaaa", every = "bbbbbbbbbbbb" } }))
	E.eq(game_hash(), "3bd83d77c8ae", "they switch off live when someone lacks them")
end)

test("letting_others_in_alone_stays_in_the_vanilla_pool", function()
	local e = E.new({ deploy = E.deploy({ has_session = false }), settings = "consent\t1\n" })
	e:boot()
	E.eq(game_hash(), "3bd83d77c8ae", "vanilla players still find the lobby")
end)

test("boot_leaves_the_revision_alone_before_mods_start", function()
	local e = E.new({
		deploy = E.deploy({ has_session = true, hashes = { open = "aaaaaaaaaaaa", every = "bbbbbbbbbbbb" }, mods = { E.mod("a", { scope = "session" }) } }),
		mods = { ["obsidian/mods/a/main"] = "SEEN = BUILD_REVISION_INFORMATION" },
	})
	e:boot()
	E.has(rawget(_G, "SEEN"), "3bd83d77c8ae tip")
	E.has(e:log(), "pool: vanilla")
end)

-- The multiplayer terminal lists only lobbies with the same build hash, so in an Obsidian pool the
-- filter titles say so, and "0/29" doesn't look like a broken lobby list.
local LOBBY_TEXTS = { lobby_public = "Public (Obsidian Only)", lobby_friends = "Friends (Obsidian Only)" }

local function booted(over, settings)
	rawset(_G, "LocalizationManager", { string = function(_, key) return "game:" .. key end })
	local d = { texts = { en = LOBBY_TEXTS } }
	for k, v in pairs(over) do d[k] = v end
	local e = E.new({ deploy = E.deploy(d), settings = settings })
	e:boot()
	e:start_frames()
	return rawget(_G, "LocalizationManager")
end

test("session_mods_leave_the_lobby_filters", function()
	local lm = booted({ has_session = true, has_static_session = true })
	E.eq(lm:string("campaign_public"), "game:campaign_public", "vanilla lobbies are listed")
end)

test("consent_alone_leaves_the_lobby_filters", function()
	local lm = booted({ has_session = false }, "consent\t1\n")
	E.eq(lm:string("campaign_public"), "game:campaign_public", "vanilla lobbies are listed too")
end)

-- The multiplayer terminal: the rooms it lists, and the hash each join request carries. room_data is
-- the "data" string of the lobby being joined, which the engine's lobby answers.
local VANILLA, OPEN = "3bd83d77c8ae", "3bd83d77c8aeaaaaaaaaaaaa"

local function terminal(over, settings)
	local t = { added = {}, sent = {}, lobby = { host = false, room_data = nil } }
	t.lobby.lobby = { data = function(_, key) return key == "data" and t.lobby.room_data or nil end }
	rawset(_G, "GameSettings", { room_data_values = { "build_game_hash", "num_players" } })
	rawset(_G, "NetworkGlobal", {
		lobby = {
			lobby = t.lobby.lobby,
			is_host = function() return t.lobby.host end,
			-- As in the game: NetworkRoom's own table hides the lobby's data method.
			data = {},
		},
		data_string_to_table = function(raw, keys)
			local out, i = {}, 0
			for v in string.gmatch(raw or "", "[^|]+") do
				i = i + 1
				out[keys[i]] = v
			end
			return out
		end,
	})
	rawset(_G, "MenuScreenCampaignMultiplayer", {
		add_room = function(_, room) t.added[#t.added + 1] = room.build_game_hash end,
		update_join = function()
			t.sent[#t.sent + 1] = rawget(_G, "BUILD_GAME_HASH")
			if t.fail then error("join screen failed") end
			return "joining", 2
		end,
	})
	local e = E.new({ deploy = E.deploy(over), settings = settings })
	e:boot()
	-- build_info sets the hash from the revision string once the loader's chunk has run.
	local _, hash = string.match(rawget(_G, "BUILD_REVISION_INFORMATION"), "parent: ([0-9]+):([0-9abcdef]+)")
	rawset(_G, "BUILD_GAME_HASH", hash)
	e:start_frames()
	t.browser = rawget(_G, "MenuScreenCampaignMultiplayer")
	return t, e
end

test("letting_others_in_lists_the_open_pool_beside_vanilla_lobbies", function()
	local t, e = terminal({ has_session = false }, "consent\t1\n")
	E.eq(rawget(_G, "BUILD_GAME_HASH"), VANILLA, "the player's own lobby stays in the vanilla pool")
	t.browser:add_room({ build_game_hash = OPEN })
	t.browser:add_room({ build_game_hash = VANILLA })
	t.browser:add_room({ build_game_hash = VANILLA .. "bbbbbbbbbbbb" })
	E.eq(t.added[1], VANILLA, "an open pool room passes the game's own check")
	E.eq(t.added[2], VANILLA)
	E.eq(t.added[3], VANILLA .. "bbbbbbbbbbbb", "another pool stays out")
	E.has(e:log(), "pool: open pool lobbies listed too")
end)

test("blocking_others_still_lists_the_open_pool", function()
	local t = terminal({ has_session = false }, "consent\t0\n")
	t.browser:add_room({ build_game_hash = OPEN })
	E.eq(t.added[1], VANILLA, "the guardrail is about the player's own lobby")
end)

test("letting_others_in_joins_an_open_pool_lobby_with_its_hash", function()
	local t = terminal({ has_session = false }, "consent\t1\n")
	t.lobby.room_data = OPEN .. "|2"
	t.browser:update_join()
	t.lobby.room_data = VANILLA .. "|2"
	t.browser:update_join()
	t.lobby.host, t.lobby.room_data = true, OPEN .. "|1"
	t.browser:update_join()
	E.eq(t.sent[1], OPEN, "the join request to a host in the open pool")
	E.eq(t.sent[2], VANILLA, "the join request to a vanilla host")
	E.eq(t.sent[3], VANILLA, "a lobby of the player's own")
	E.eq(rawget(_G, "BUILD_GAME_HASH"), VANILLA, "the hash is back after the call")
end)

-- The lobby's room data may not be readable while joining: the room picked in the list decides.
test("an_open_pool_room_picked_in_the_list_is_joined_with_its_hash", function()
	local t, e = terminal({ has_session = false }, "consent\t1\n")
	t.browser:add_room({ build_game_hash = OPEN, address = 1001 })
	t.browser:add_room({ build_game_hash = VANILLA, address = 1002 })
	t.browser.join_address = 1001
	t.browser:update_join()
	t.browser.join_address = 1002
	t.browser:update_join()
	t.browser.join_address, t.lobby.host = 1001, true
	t.browser:update_join()
	E.eq(t.sent[1], OPEN, "the open pool room")
	E.eq(t.sent[2], VANILLA, "the vanilla room")
	E.eq(t.sent[3], VANILLA, "a lobby the player hosts now")
	E.has(e:log(), "pool: joining with the open pool's hash")
end)

test("the_hash_comes_back_when_the_join_fails", function()
	local t = terminal({ has_session = false }, "consent\t1\n")
	t.lobby.room_data = OPEN .. "|2"
	local a, b = t.browser:update_join()
	E.eq(a, "joining", "what the game's function answers")
	E.eq(b, 2)
	t.fail = true
	pcall(t.browser.update_join, t.browser)
	E.eq(#t.sent, 2, "the game's function ran once")
	E.eq(rawget(_G, "BUILD_GAME_HASH"), VANILLA)
end)

test("session_mods_that_switch_off_join_vanilla_lobbies", function()
	local t = terminal({ has_session = true }, "consent\t1\n")
	t.browser:add_room({ build_game_hash = VANILLA })
	E.eq(t.added[1], VANILLA)
	t.lobby.room_data = VANILLA .. "|2|0"
	t.browser:update_join()
	E.eq(t.sent[1], VANILLA, "public or not, the session gate holds them back")
end)

-- Privacy values as GameSettings.room_privacy_settings has them in these tests.
local PUBLIC, FRIENDS, PRIVATE = 0, 1, 2

local function static_terminal()
	local t, e = terminal({ has_session = true, has_static_session = true })
	rawget(_G, "GameSettings").room_data_values = { "build_game_hash", "num_players", "privacy" }
	rawget(_G, "GameSettings").room_privacy_settings = { public = PUBLIC, friends = FRIENDS, private = PRIVATE }
	return t, e
end

test("static_session_content_joins_private_lobbies_with_the_game_hash", function()
	local t = static_terminal()
	t.lobby.room_data = VANILLA .. "|2|" .. PRIVATE
	t.browser:update_join()
	t.lobby.room_data = VANILLA .. "|2|" .. FRIENDS
	t.browser:update_join()
	E.eq(t.sent[1], VANILLA, "a private lobby")
	E.eq(t.sent[2], VANILLA, "a friends-only lobby")
end)

test("static_session_content_stays_out_of_other_public_lobbies", function()
	local t = static_terminal()
	t.lobby.room_data = VANILLA .. "|2|" .. PUBLIC
	t.browser:update_join()
	t.lobby.room_data = nil
	t.browser:add_room({ build_game_hash = VANILLA, address = 7, privacy = PRIVATE })
	t.browser.join_address = 7
	t.browser:update_join()
	t.browser.join_address = nil
	t.browser:update_join()
	E.eq(t.sent[1], OPEN, "a vanilla host turns it away as another version")
	E.eq(t.sent[2], VANILLA, "the room picked in the list is private")
	E.eq(t.sent[3], OPEN, "unknown privacy counts as public")
end)

-- The host's side of the door: the room hash it shows and the join requests it answers.
local function fake_state()
	local S = { got = {} }
	S.rpc_on_join_request = function(_, _, hash) S.got[#S.got + 1] = hash end
	rawset(_G, "StateMenu", S)
	return S
end

local function static_host(privacy)
	local room = { privacy = privacy }
	rawset(_G, "NetworkRoom", { set_data = function(_, key, value) room[key] = value end, get_data = function(_, key) return room[key] end })
	fake_state()
	local _, e = static_terminal()
	local net = rawget(_G, "NetworkGlobal")
	net.lobby.is_host = function() return true end
	net.lobby.get_data = function(_, key) return room[key] end
	return room, e
end

test("a_public_lobby_with_static_content_shows_the_open_pool_hash", function()
	local room = static_host(nil)
	local NR = rawget(_G, "NetworkRoom")
	NR:set_data("build_game_hash", VANILLA)
	E.eq(room.build_game_hash, OPEN, "privacy not set yet counts as public")
	NR:set_data("privacy", PRIVATE)
	E.eq(room.build_game_hash, VANILLA, "a private lobby shows the game's hash")
	NR:set_data("privacy", PUBLIC)
	E.eq(room.build_game_hash, OPEN)
end)

test("a_public_lobby_with_static_content_turns_vanilla_players_away", function()
	local room, e = static_host(PUBLIC)
	local S = rawget(_G, "StateMenu")
	S:rpc_on_join_request("p1", VANILLA)
	S:rpc_on_join_request("p2", OPEN)
	room.privacy = FRIENDS
	S:rpc_on_join_request("p3", VANILLA)
	E.eq(S.got[1] ~= VANILLA, true, "the game answers wrong version")
	E.eq(S.got[2], VANILLA, "an Obsidian player with the open pool's hash")
	E.eq(S.got[3], VANILLA, "anyone may join a friends-only lobby")
	E.has(e:log(), "pool: turned away a player without Obsidian from a public lobby")
end)

test("a_vanilla_pool_host_takes_the_open_pool_hash_as_its_own", function()
	local S = fake_state()
	terminal({ has_session = true })
	S:rpc_on_join_request("p", OPEN)
	E.eq(S.got[1], VANILLA, "a player from Obsidian 1.3.0's open pool")
end)

test("vanilla_pool_leaves_the_lobby_filters", function()
	local lm = booted({ has_session = false })
	E.eq(lm:string("campaign_public"), "game:campaign_public")
end)

return T
