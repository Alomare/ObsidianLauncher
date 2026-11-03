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

test("session_uses_the_open_tag", function()
	E.new()
	require("obsidian/loader/pool").apply(E.deploy({ has_session = true }))
	E.eq(game_hash(), "3bd83d77c8aeaaaaaaaaaaaa")
end)

test("mods_every_player_needs_get_their_own_pool", function()
	for _, consent in ipairs({ false, true }) do
		E.new()
		require("obsidian/loader/pool").apply(E.deploy({ has_session = true, hashes = { open = "aaaaaaaaaaaa", every = "bbbbbbbbbbbb" } }), consent)
		E.eq(game_hash(), "3bd83d77c8aebbbbbbbbbbbb", "consent " .. tostring(consent))
	end
end)

test("consent_alone_joins_the_open_pool", function()
	local e = E.new({ deploy = E.deploy({ has_session = false }), settings = "consent\t1\n" })
	e:boot()
	E.eq(game_hash(), "3bd83d77c8aeaaaaaaaaaaaa")
end)

test("rewritten_hash_matches_build_info_pattern", function()
	E.new()
	require("obsidian/loader/pool").apply(E.deploy({ has_session = true }))
	local rev = rawget(_G, "BUILD_REVISION_INFORMATION")
	E.eq(string.match(rev, "parent: ([0-9]+):"), "32148")
	E.eq(game_hash(), "3bd83d77c8aeaaaaaaaaaaaa")
	E.has(rev, " tip\n [code] ok", "rest of the revision string")
end)

test("bad_tag_leaves_the_hash_vanilla", function()
	E.new()
	require("obsidian/loader/pool").apply(E.deploy({ has_session = true, hashes = { open = "XYZ" } }))
	E.eq(game_hash(), "3bd83d77c8ae")
end)

test("boot_applies_the_pool_before_mods_start", function()
	local e = E.new({
		deploy = E.deploy({ has_session = true, mods = { E.mod("a", { scope = "session" }) } }),
		mods = { ["obsidian/mods/a/main"] = "SEEN = BUILD_REVISION_INFORMATION" },
	})
	e:boot()
	E.has(rawget(_G, "SEEN"), "3bd83d77c8aeaaaaaaaaaaaa")
	E.has(e:log(), "pool: open aaaaaaaaaaaa")
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

test("obsidian_pool_marks_the_lobby_filters", function()
	local lm = booted({ has_session = true, modset = string.rep("a", 64) })
	E.eq(lm:string("campaign_public"), "Public (Obsidian Only)")
	E.eq(lm:string("campaign_friends"), "Friends (Obsidian Only)")
	E.eq(lm:string("campaign_back"), "game:campaign_back", "other texts")
end)

test("consent_alone_leaves_the_lobby_filters", function()
	local lm = booted({ has_session = false }, "consent\t1\n")
	E.eq(lm:string("campaign_public"), "game:campaign_public", "vanilla lobbies are listed too")
end)

-- The multiplayer terminal for a player in the open pool: the rooms it lists, and the hash each join
-- request carries. room_data is the "data" string of the lobby being joined.
local VANILLA, OPEN = "3bd83d77c8ae", "3bd83d77c8aeaaaaaaaaaaaa"

local function terminal(over, settings)
	local t = { added = {}, sent = {}, lobby = { lobby = {}, host = false, room_data = nil } }
	rawset(_G, "GameSettings", { room_data_values = { "build_game_hash", "num_players" } })
	rawset(_G, "NetworkGlobal", {
		lobby = {
			lobby = t.lobby.lobby,
			is_host = function() return t.lobby.host end,
			data = function(_, key) return key == "data" and t.lobby.room_data or nil end,
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

test("consent_alone_lists_vanilla_lobbies_beside_the_open_pool", function()
	local t, e = terminal({ has_session = false }, "consent\t1\n")
	E.eq(rawget(_G, "BUILD_GAME_HASH"), OPEN, "the player's own lobby stays in the open pool")
	t.browser:add_room({ build_game_hash = VANILLA })
	t.browser:add_room({ build_game_hash = OPEN })
	t.browser:add_room({ build_game_hash = VANILLA .. "bbbbbbbbbbbb" })
	E.eq(t.added[1], OPEN, "a vanilla room passes the game's own check")
	E.eq(t.added[2], OPEN)
	E.eq(t.added[3], VANILLA .. "bbbbbbbbbbbb", "another pool stays out")
	E.has(e:log(), "pool: vanilla lobbies listed too")
end)

test("consent_alone_joins_a_vanilla_lobby_with_the_vanilla_hash", function()
	local t = terminal({ has_session = false }, "consent\t1\n")
	t.lobby.room_data = VANILLA .. "|2"
	t.browser:update_join()
	t.lobby.room_data = OPEN .. "|2"
	t.browser:update_join()
	t.lobby.host, t.lobby.room_data = true, VANILLA .. "|1"
	t.browser:update_join()
	E.eq(t.sent[1], VANILLA, "the join request to a vanilla host")
	E.eq(t.sent[2], OPEN, "the join request to an Obsidian host")
	E.eq(t.sent[3], OPEN, "a lobby of the player's own")
	E.eq(rawget(_G, "BUILD_GAME_HASH"), OPEN, "the hash is back after the call")
end)

test("the_hash_comes_back_when_the_join_fails", function()
	local t = terminal({ has_session = false }, "consent\t1\n")
	t.lobby.room_data = VANILLA .. "|2"
	local a, b = t.browser:update_join()
	E.eq(a, "joining", "what the game's function answers")
	E.eq(b, 2)
	t.fail = true
	pcall(t.browser.update_join, t.browser)
	E.eq(#t.sent, 2, "the game's function ran once")
	E.eq(rawget(_G, "BUILD_GAME_HASH"), OPEN)
end)

test("session_mods_keep_vanilla_lobbies_out", function()
	local t = terminal({ has_session = true }, "consent\t1\n")
	t.browser:add_room({ build_game_hash = VANILLA })
	E.eq(t.added[1], VANILLA, "the game's check leaves it out")
	t.lobby.room_data = VANILLA .. "|2"
	t.browser:update_join()
	E.eq(t.sent[1], OPEN)
end)

test("vanilla_pool_leaves_the_lobby_filters", function()
	local lm = booted({ has_session = false })
	E.eq(lm:string("campaign_public"), "game:campaign_public")
end)

return T
