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
local LOBBY_TEXTS = { lobby_public = "Public (Obsidian only)", lobby_friends = "Friends (Obsidian only)" }

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
	E.eq(lm:string("campaign_public"), "Public (Obsidian only)")
	E.eq(lm:string("campaign_friends"), "Friends (Obsidian only)")
	E.eq(lm:string("campaign_back"), "game:campaign_back", "other texts")
end)

test("consent_alone_marks_the_lobby_filters", function()
	local lm = booted({ has_session = false }, "consent\t1\n")
	E.eq(lm:string("campaign_public"), "Public (Obsidian only)")
end)

test("vanilla_pool_leaves_the_lobby_filters", function()
	local lm = booted({ has_session = false })
	E.eq(lm:string("campaign_public"), "game:campaign_public")
end)

return T
