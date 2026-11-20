-- Custom perks (Obsidian API lib/feats.lua): a copy of a base in the game's perk list, the base's
-- functions with the mod's after them on the owner's master, and the base's index on the network.
local E = require "fake_engine"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local function start(opts)
	local e, game, ui = G.start({ "obsidian.api" }, opts)
	e:core().current = e:core().mods_by_id["obsidian.api"]
	return rawget(_G, "obsidian").import("obsidian.api"), game, e, ui
end

local function perk_world(e)
	local log = { written = {} }
	local fields = {}
	rawset(_G, "GameObject", {
		set = function(self, field, value) fields[field] = value end,
		get = function(self, field) return fields[field] end,
	})
	rawset(_G, "GameObjectFieldsReplicator", {
		init = function(self)
			self.fields = { { name = "perk", write = function(v) log.written.perk = v end } }
		end,
	})
	e.opts.modules["x"] = true
	rawget(_G, "require")("x")
	return log, fields
end

test("a_custom_perk_runs_the_bases_effect_and_then_the_mods", function()
	local api, game, e = start()
	perk_world(e)
	local seen = {}
	local made = api.perks.define({ name = "test_steady", base = "perk_cardio", title = "Steady",
		on_setup = function(ctx) seen[#seen + 1] = "setup " .. ctx.name ctx.state.mine = true end,
		on_update = function(ctx) seen[#seen + 1] = "update " .. tostring(ctx.dt) end,
		on_remove = function(ctx) seen[#seen + 1] = "remove " .. tostring(ctx.state.mine) end })
	local L, P = rawget(_G, "LoadoutSettings"), rawget(_G, "PerkSettings")
	E.eq(made.index, 3)
	E.eq(L.perks[3].name, "test_steady")
	E.eq(rawget(_G, "LocalizationManager"):string("test_steady"), "Steady")
	local p = P.test_steady
	local state = {}
	p.setup_master("unit", {}, state, {})
	E.eq(game.perk_calls[#game.perk_calls], "cardio setup_master", "the base first")
	E.eq(seen[1], "setup test_steady")
	p.update_master("unit", {}, state, 0.5)
	E.eq(seen[2], "update 0.5")
	p.remove_master("unit", {}, state)
	E.eq(game.perk_calls[#game.perk_calls], "cardio remove_master")
	E.eq(seen[3], "remove true")
	E.eq(P.perk_cardio.update_master, nil, "the base is left alone")
end)

test("a_custom_perks_slave_functions_are_the_bases", function()
	local api, game, e = start()
	perk_world(e)
	api.perks.define({ name = "test_sight", base = "perk_lasersight", title = "Sight" })
	local P = rawget(_G, "PerkSettings")
	E.eq(P.test_sight.setup_slave, P.perk_lasersight.setup_slave)
end)

test("the_perk_goes_out_as_the_bases_index", function()
	local api, game, e = start()
	local log, fields = perk_world(e)
	local made = api.perks.define({ name = "test_steady", base = "perk_cardio", title = "Steady" })
	local go = rawget(_G, "GameObject")
	local mine = {}
	go.set(mine, "perk_index", made.index)
	E.eq(fields.perk_index, 1)
	E.eq(go.get(mine, "perk_index"), made.index)
	local r = {}
	rawget(_G, "GameObjectFieldsReplicator").init(r)
	r.fields[1].write(made.index)
	E.eq(log.written.perk, 1)
	r.fields[1].write(2)
	E.eq(log.written.perk, 2)
end)

test("a_custom_perk_definition_is_checked", function()
	local api, game, e = start()
	perk_world(e)
	local function why(def) return select(2, api.perks.define(def)) end
	E.eq(why({ name = "perk_cardio", base = "perk_cardio", title = "x" }), "taken")
	E.eq(why({ name = "test_a", base = "perk_nothing", title = "x" }), "not_available")
	E.eq(why({ name = "test_a", base = "perk_cardio", title = "x", on_setup = "no" }), "bad_type")
	E.eq(why({ name = "test_a", base = "perk_cardio" }), "bad_type")
end)

test("a_mods_perk_function_stops_when_the_lobby_isnt_cleared", function()
	local lobby = { my_id = "me", members = { "me", "stranger" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	perk_world(e)
	local ran = 0
	api.perks.define({ name = "test_steady", base = "perk_cardio", title = "Steady", on_setup = function() ran = ran + 1 end })
	e:frames(31)
	rawget(_G, "PerkSettings").test_steady.setup_master("unit", {}, {}, {})
	E.eq(ran, 0)
	E.eq(game.perk_calls[#game.perk_calls], "cardio setup_master", "the base still runs")
end)

return T
