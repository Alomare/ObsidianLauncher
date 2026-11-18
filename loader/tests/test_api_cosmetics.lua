-- Custom cosmetics (Obsidian API lib/wear.lua): a copy of a base in the game's helmet, armour and cape
-- lists, the base's index on the network and in the save, the custom one where the name is known.
local E = require "fake_engine"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local function start(opts)
	local e, game, ui = G.start({ "obsidian.api" }, opts)
	e:core().current = e:core().mods_by_id["obsidian.api"]
	return rawget(_G, "obsidian").import("obsidian.api"), game, e, ui
end

-- The game parts lib/wear.lua replaces or hooks.
local function wear_world(e)
	local C = rawget(_G, "CustomizationSettings")
	C.helmet[1].unit_path = "characters/helmet_a"
	C.body[1].male = { unit_path = "characters/body_a_male" }
	C.body[1].female = { unit_path = "characters/body_a_female" }
	local log = { written = {}, drawn = {} }
	-- A replicator with the three fields, writing what it's given.
	rawset(_G, "GameObjectFieldsReplicator", {
		init = function(self)
			self.fields = {}
			for _, name in ipairs({ "helmet", "body", "cape", "other" }) do
				self.fields[#self.fields + 1] = { name = name, write = function(v) log.written[name] = v end }
			end
		end,
	})
	rawset(_G, "CustomizationComponent", {
		update_unit = function(self, state, unit)
			log.drawn[#log.drawn + 1] = { unit = unit, helmet = state.helmet, body = state.body, cape = state.cape }
		end,
	})
	e.opts.modules["x"] = true
	rawget(_G, "require")("x")
	return log
end

local VISOR = { name = "test_visor", base = "cus_helmet_a", title = { en = "Visor", fr = "Visière" }, model = "mods/test/visor" }
local PLATE = { name = "test_plate", base = "cus_body_a", title = "Plate", model = { male = "mods/test/plate_m", female = "mods/test/plate_f" } }

test("a_custom_cosmetic_is_a_copy_of_its_base_with_its_own_model", function()
	local api, game, e = start()
	wear_world(e)
	local C = rawget(_G, "CustomizationSettings")
	local made = api.cosmetics.define(VISOR)
	E.eq(made.slot, "helmet")
	E.eq(made.index, 3)
	E.eq(C.helmet[3].name, "test_visor")
	E.eq(C.helmet[3].unit_path, "mods/test/visor")
	E.eq(C.helmet[1].unit_path, "characters/helmet_a", "the base is left alone")
	local plate = api.cosmetics.define(PLATE)
	E.eq(plate.slot, "body")
	E.eq(C.body[plate.index].male.unit_path, "mods/test/plate_m")
	E.eq(C.body[plate.index].female.unit_path, "mods/test/plate_f")
	E.eq(C.body[1].male.unit_path, "characters/body_a_male")
	local one = api.cosmetics.define({ name = "test_suit", base = "cus_body_a", title = "Suit", model = "mods/test/suit" })
	E.eq(C.body[one.index].female.unit_path, "mods/test/suit", "one path serves both")
	E.eq(rawget(_G, "LocalizationManager"):string("test_visor"), "Visor")
	E.eq(api.cosmetics.get("test_plate").slot, "body")
	E.eq(#api.cosmetics.list(), 3)
end)

test("a_custom_cosmetic_definition_is_checked", function()
	local api, game, e = start()
	wear_world(e)
	local function why(def) return select(2, api.cosmetics.define(def)) end
	E.eq(why({ name = "cus_helmet_a", base = "cus_helmet_a", title = "x" }), "taken")
	E.eq(why({ name = "test_a", base = "cus_nothing", title = "x" }), "not_available")
	E.eq(why({ name = "test a", base = "cus_helmet_a", title = "x" }), "bad_name")
	E.eq(why({ name = "test_a", base = "cus_helmet_a" }), "bad_type")
	E.eq(why({ name = "test_a", base = "cus_body_a", title = "x", model = { child = "y" } }), "bad_type")
	E.eq(why({ name = "test_a", base = "cus_helmet_a", title = "x", passive = {} }), "not_available", "passives come later")
end)

test("the_network_gets_the_bases_index", function()
	local api, game, e = start()
	local log = wear_world(e)
	local made = api.cosmetics.define(VISOR)
	local r = {}
	rawget(_G, "GameObjectFieldsReplicator").init(r)
	r.fields[1].write(made.index)
	E.eq(log.written.helmet, 1, "the base's")
	r.fields[1].write(2)
	E.eq(log.written.helmet, 2, "a game helmet as it is")
	r.fields[4].write(made.index)
	E.eq(log.written.other, made.index, "other fields are left alone")
end)

test("the_save_keeps_the_base_and_the_game_reads_the_custom_one", function()
	local api, game, e = start()
	wear_world(e)
	local made = api.cosmetics.define(VISOR)
	local sm = rawget(_G, "SaveManager")
	sm:user_set("customization", { helmet = made.index, body = 1, cape = 1 }, "user")
	E.eq(game.save.user.customization.helmet, 1, "the save holds the base")
	E.eq(sm:user_get("customization", "user").helmet, made.index, "the game reads the custom one")
	E.eq(api.cosmetics.worn().helmet, "test_visor")
	E.eq(rawget(_G, "obsidian").save_data.get("worn"), "helmet=test_visor", "kept by the library")
	sm:user_set("customization", { helmet = 2, body = 1, cape = 1 }, "user")
	E.eq(sm:user_get("customization", "user").helmet, 2, "another helmet chosen")
	E.eq(api.cosmetics.worn().helmet, nil)
end)

test("a_save_whose_custom_cosmetic_is_gone_reads_as_the_base", function()
	local api, game, e = start()
	wear_world(e)
	rawget(_G, "obsidian").save_data.set("worn", "helmet=test_gone")
	local made = api.cosmetics.define(VISOR)
	game.save.user.customization = { helmet = 1, body = 1, cape = 1 }
	E.eq(rawget(_G, "SaveManager"):user_get("customization", "user").helmet, 1)
	E.eq(made.index, 3)
end)

test("each_helldiver_draws_the_custom_cosmetic_where_this_machine_knows_it", function()
	local lobby = { my_id = "me", members = { "me" }, data = {}, extra = {} }
	local api, game, e = start({ lobby = lobby })
	local log = wear_world(e)
	local made = api.cosmetics.define(VISOR)
	rawget(_G, "SaveManager"):user_set("customization", { helmet = made.index, body = 1, cape = 1 }, "user")
	e:frames(70)
	local key = "mod:obsidian.api:wear"
	E.eq(lobby.published[key], "helmet=test_visor")
	local CC = rawget(_G, "CustomizationComponent")
	-- This machine's own Helldiver: its state came back from the network as the base.
	CC.update_unit({}, { helmet = 1, body = 1, cape = 1 }, game.me)
	E.eq(log.drawn[#log.drawn].helmet, made.index, "the owner sees the custom helmet")
	-- A teammate with the same helmet, and one without.
	local function teammate(peer)
		local avatar = { alive = true, master = {}, slave = {}, path = "characters/helldiver" }
		local go = { avatar_unit = avatar }
		function go:owned() return false end
		function go:owner() return peer end
		function go:exists() return true end
		function go:get(k) return self[k] end
		function go:has(k) return self[k] ~= nil end
		table.insert(game.state.players, { player_go = go, pad_number = 1 })
		return avatar
	end
	local friend, stranger = teammate("friend"), teammate("stranger")
	lobby.members = { "me", "friend", "stranger" }
	lobby.extra = { friend = { [key] = "helmet=test_visor" }, stranger = { [key] = "helmet=their_own" } }
	e:frames(70)
	CC.update_unit({}, { helmet = 1, body = 1, cape = 1 }, friend)
	E.eq(log.drawn[#log.drawn].helmet, made.index)
	CC.update_unit({}, { helmet = 1, body = 1, cape = 1 }, stranger)
	E.eq(log.drawn[#log.drawn].helmet, 1, "an unknown name keeps the base")
	CC.update_unit({}, { helmet = 2, body = 1, cape = 1 }, friend)
	E.eq(log.drawn[#log.drawn].helmet, 2, "a different base that came is drawn as it came")
	-- The Arsenal's preview: a screen as the state, no avatar.
	CC.update_unit({}, { helmet = made.index, body = 1, cape = 1 }, "preview character")
	E.eq(log.drawn[#log.drawn].helmet, made.index)
end)

test("a_custom_cosmetic_unlocks_by_its_own_rule", function()
	local api, game, e = start()
	wear_world(e)
	local P = rawget(_G, "ProgressionSettings")
	P.can_use = function() return false end
	e.opts.modules["y"] = true
	rawget(_G, "require")("y")
	api.cosmetics.define({ name = "test_free", base = "cus_helmet_a", title = "Free", unlock = "free" })
	api.cosmetics.define({ name = "test_manual", base = "cus_helmet_a", title = "Manual", unlock = "manual" })
	E.eq(P.can_use("test_free", "customization", 0), true)
	E.eq(P.can_use("test_manual", "customization", 0), false)
	E.eq(api.cosmetics.unlock("test_manual"), true)
	E.eq(P.can_use("test_manual", "customization", 0), true)
end)

return T
