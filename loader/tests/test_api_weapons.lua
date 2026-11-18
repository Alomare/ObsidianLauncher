-- Custom weapons (Obsidian API lib/arms.lua): a copy of a base in the game's weapon lists, the base on
-- the wire, its own numbers on the weapon spawned for it, and its look where the name is known.
local E = require "fake_engine"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local RIFLE = { weapon_info = { power = 75, magazine_capacity = 30, firerate = 650 }, hud_info = { weapon_icon = "wepicon_rifle" } }

local function start(opts)
	local e, game, ui = G.start({ "obsidian.api" }, opts)
	e:core().current = e:core().mods_by_id["obsidian.api"]
	return rawget(_G, "obsidian").import("obsidian.api"), game, e, ui
end

-- The game parts lib/arms.lua replaces or hooks, with a log of what was drawn.
local function weapon_world(e)
	local log = { spawned = {}, linked = {}, destroyed = {}, visible = {} }
	local fields = {}
	rawset(_G, "GameObject", {
		set = function(self, field, value) fields[field] = value end,
		get = function(self, field) return fields[field] end,
	})
	local lsm = rawget(_G, "LuaSettingsManager")
	lsm.get_modified_settings = function(self, unit, upgrade)
		local path = type(unit) == "table" and unit.path or unit
		if path == "equipment/rifle" then return RIFLE end
		return nil
	end
	lsm.get_settings_by_unit = function(self, unit) return unit and unit.path == "equipment/rifle" and RIFLE or nil end
	local U = rawget(_G, "Unit")
	U.set_unit_visibility = function(u, on) log.visible[u] = on end
	rawset(_G, "World", {
		spawn_unit = function(world, path)
			if path == "missing/model" then error("unknown resource") end
			local u = { alive = true, master = {}, slave = {}, path = path }
			log.spawned[#log.spawned + 1] = u
			return u
		end,
		link_unit = function(world, child, child_node, parent, node) log.linked[#log.linked + 1] = { child = child, parent = parent, node = node } end,
		destroy_unit = function(world, u) u.alive = false log.destroyed[#log.destroyed + 1] = u end,
	})
	-- The inventory asks for each weapon by name and spawns the entry's unit_path.
	local spawner = {}
	rawset(_G, "EntitySpawner", {
		create_entity = function(self, unit) unit.registered = true return unit end,
	})
	rawset(_G, "InventoryComponent", {
		set_loadout = function(self, avatar, ctx, from, loadout)
			local L = rawget(_G, "LoadoutSettings")
			local state = rawget(_G, "EntityAux").state_master(avatar, "inventory")
			local e = L.get_item("primary_weapon", loadout.primary_weapon)
			local u = { alive = true, master = {}, slave = {}, path = e.unit_path }
			rawget(_G, "EntitySpawner").create_entity(spawner, u)
			state.primary_weapon = u
			return u
		end,
	})
	rawget(_G, "LoadoutSettings").get_item = function(kind, name)
		for _, e in ipairs(rawget(_G, "LoadoutSettings")[kind]) do
			if e.name == name then return e end
		end
	end
	E.eq(type(rawget(_G, "Boot")), "table", "the fake has Boot")
	e.opts.modules["x"] = true
	rawget(_G, "require")("x")
	return log, fields
end

local BREAKER = { name = "test_breaker", base = "weapon_rifle", title = { en = "Breaker", fr = "Briseur" },
	description = "Short and loud.", model = "mods/test/breaker", icon = "wepicon_breaker",
	stats = { power = 120, magazine_capacity = 12 }, attributes = { attr_power = 5 } }

local function loadout(game, name)
	return rawget(_G, "InventoryComponent").set_loadout({}, game.me, {}, nil, { primary_weapon = name })
end

test("a_custom_weapon_is_a_copy_of_its_base_with_its_own_words", function()
	local api, game, e = start()
	weapon_world(e)
	local L = rawget(_G, "LoadoutSettings")
	local count = #L.primary_weapon
	local made = api.weapons.define(BREAKER)
	E.eq(made.slot, "primary_weapon")
	E.eq(made.index, count + 1)
	local entry = L.primary_weapon[made.index]
	E.eq(entry.name, "test_breaker")
	E.eq(entry.unit_path, "equipment/rifle", "the base's unit: what goes out on the wire")
	E.eq(entry.icon, "wepicon_breaker")
	E.eq(entry.attributes.attr_power, 5)
	E.eq(#entry.upgrades, 2, "the base's upgrades")
	E.eq(L.primary_weapon[1].name, "weapon_rifle", "the base is left alone")
	local lm = rawget(_G, "LocalizationManager")
	E.eq(lm:string("test_breaker"), "Breaker")
	E.eq(lm:string(entry.description), "Short and loud.")
	lm.current_language = "fr"
	E.eq(lm:string("test_breaker"), "Briseur")
	E.eq(api.weapons.list()[1].base, "weapon_rifle")
	E.eq(api.weapons.get("test_breaker").model, "mods/test/breaker")
end)

test("a_custom_weapon_definition_is_checked", function()
	local api, game, e = start()
	weapon_world(e)
	local function why(def) return select(2, api.weapons.define(def)) end
	E.eq(why({ name = "weapon_rifle", base = "weapon_rifle", title = "x" }), "taken")
	E.eq(why({ name = "test_a", base = "weapon_nothing", title = "x" }), "not_available")
	E.eq(why({ name = "test a", base = "weapon_rifle", title = "x" }), "bad_name")
	E.eq(why({ name = "test_a", base = "weapon_rifle" }), "bad_type", "no title")
	E.eq(why({ name = "test_a", base = "weapon_rifle", title = "x", stats = { power = "high" } }), "bad_type")
	E.eq(why({ name = "test_a", base = "weapon_rifle", title = "x", stats = { power = -1 } }), "bad_type")
	E.eq(why({ name = "test_a", base = "weapon_rifle", title = "x", model = 5 }), "bad_type")
	E.eq(why({ name = "test_a", base = "weapon_rifle", title = "x", unlock = "someday" }), "bad_type")
	api.weapons.define(BREAKER)
	E.eq(why({ name = "test_a", base = "test_breaker", title = "x" }), "not_available", "a base is one of the game's")
	E.eq(why(BREAKER), "taken")
	E.eq(select(2, api.stratagems.define({ name = "test_breaker", base = "stratagem_resupply", title = "x" })), nil,
		"a stratagem check only looks at stratagems")
end)

test("a_sidearm_base_makes_a_sidearm", function()
	local api, game, e = start()
	weapon_world(e)
	local made = api.weapons.define({ name = "test_sidearm", base = "weapon_pistol", title = "Pocket" })
	E.eq(made.slot, "sidearm_weapon")
	E.eq(rawget(_G, "LoadoutSettings").sidearm_weapon[made.index].unit_path, "equipment/pistol")
end)

test("the_loadout_sends_the_bases_index_and_this_machine_reads_its_own", function()
	local api, game, e = start()
	local log, fields = weapon_world(e)
	local made = api.weapons.define(BREAKER)
	local go = rawget(_G, "GameObject")
	go.set({}, "primary_weapon_index", made.index)
	E.eq(fields.primary_weapon_index, 1, "the rifle's index on the wire")
	E.eq(go.get({}, "primary_weapon_index"), 1, "another object reads what came")
	local mine = {}
	go.set(mine, "primary_weapon_index", made.index)
	E.eq(go.get(mine, "primary_weapon_index"), made.index, "this object reads what it set")
	go.set(mine, "primary_weapon_index", 2)
	E.eq(fields.primary_weapon_index, 2, "a game weapon goes as it is")
	E.eq(go.get(mine, "primary_weapon_index"), 2)
end)

test("the_weapon_spawned_for_a_custom_entry_reads_its_numbers_and_the_others_dont", function()
	local api, game, e = start()
	weapon_world(e)
	api.weapons.define(BREAKER)
	local lsm = rawget(_G, "LuaSettingsManager")
	local mine = loadout(game, "test_breaker")
	E.eq(mine.registered, true)
	E.eq(mine.path, "equipment/rifle", "the base's network object")
	local s = lsm:get_modified_settings(mine, 0)
	E.eq(s.weapon_info.power, 120)
	E.eq(s.weapon_info.magazine_capacity, 12)
	E.eq(s.weapon_info.firerate, 650, "the rest is the base's")
	E.eq(s.hud_info.weapon_icon, "wepicon_breaker")
	E.eq(lsm:get_settings_by_unit(mine).weapon_info.power, 120)
	E.eq(lsm:get_modified_settings(mine, 0), s, "the same table each time")
	E.eq(RIFLE.weapon_info.power, 75, "the base's settings are left alone")
	local plain = loadout(game, "weapon_rifle")
	E.eq(lsm:get_modified_settings(plain, 0).weapon_info.power, 75, "a rifle stays a rifle")
	E.eq(lsm:get_modified_settings("equipment/rifle", 0).weapon_info.power, 75, "by path, the base's")
	E.eq(api.weapons.of(mine), "test_breaker")
	E.eq(api.weapons.of(plain), nil)
end)

test("outside_a_cleared_lobby_a_custom_weapon_has_the_bases_numbers_and_keeps_its_look", function()
	local lobby = { my_id = "me", members = { "me", "stranger" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	local log = weapon_world(e)
	api.weapons.define(BREAKER)
	e:frames(31)
	local mine = loadout(game, "test_breaker")
	E.eq(rawget(_G, "LuaSettingsManager"):get_modified_settings(mine, 0).weapon_info.power, 75)
	E.eq(#log.spawned, 1, "the look is drawn")
end)

test("the_look_hides_the_base_and_follows_the_games_visibility", function()
	local api, game, e = start()
	local log = weapon_world(e)
	api.weapons.define(BREAKER)
	local mine = loadout(game, "test_breaker")
	E.eq(#log.spawned, 1)
	local model = log.spawned[1]
	E.eq(model.path, "mods/test/breaker")
	E.eq(log.linked[1].child, model)
	E.eq(log.linked[1].parent, mine)
	E.eq(log.visible[mine], false, "the base's meshes hidden")
	local U = rawget(_G, "Unit")
	U.set_unit_visibility(mine, false)
	E.eq(log.visible[model], false, "the game hides the weapon: the model goes")
	E.eq(log.visible[mine], false)
	U.set_unit_visibility(mine, true)
	E.eq(log.visible[model], true, "and comes back")
	E.eq(log.visible[mine], false, "the base stays hidden")
	mine.alive = false
	e:frames(1)
	E.eq(log.destroyed[1], model, "the model goes with the weapon")
	E.eq(api.weapons.look and nil, nil)
end)

test("a_model_the_game_cant_spawn_leaves_the_base_look_and_says_so_once", function()
	local api, game, e = start()
	local log = weapon_world(e)
	api.weapons.define({ name = "test_bad_model", base = "weapon_rifle", title = "x", model = "missing/model" })
	local a = loadout(game, "test_bad_model")
	local b = loadout(game, "test_bad_model")
	E.eq(#log.spawned, 0)
	E.eq(log.visible[a], nil, "the base is not hidden")
	E.eq(log.visible[b], nil)
	E.eq(api.weapons.of(a), "test_bad_model", "the numbers still apply")
end)

test("a_weapon_without_a_model_keeps_the_bases_look", function()
	local api, game, e = start()
	local log = weapon_world(e)
	api.weapons.define({ name = "test_plain", base = "weapon_rifle", title = "Plain", stats = { power = 80 } })
	loadout(game, "test_plain")
	E.eq(#log.spawned, 0)
end)

test("other_players_see_the_look_when_they_have_the_same_weapon", function()
	local lobby = { my_id = "me", members = { "me" }, data = {}, extra = {} }
	local api, game, e = start({ lobby = lobby })
	local log = weapon_world(e)
	api.weapons.define(BREAKER)
	loadout(game, "test_breaker")
	e:frames(70)
	local key = "mod:obsidian.api:weapons"
	E.eq(lobby.published[key], "primary_weapon=test_breaker", "this machine shares its weapon")
	-- A teammate with the same weapon, and one with a weapon this machine doesn't know.
	local function teammate(peer, weapon_unit)
		local avatar = { alive = true, master = {}, slave = { inventory = { primary_weapon = weapon_unit } }, path = "characters/helldiver" }
		local go = { avatar_unit = avatar }
		function go:owned() return false end
		function go:owner() return peer end
		function go:exists() return true end
		function go:get(k) return self[k] end
		function go:has(k) return self[k] ~= nil end
		table.insert(game.state.players, { player_go = go, pad_number = 1 })
	end
	local theirs = { alive = true, master = {}, slave = {}, path = "equipment/rifle" }
	local other = { alive = true, master = {}, slave = {}, path = "equipment/rifle" }
	teammate("friend", theirs)
	teammate("stranger", other)
	lobby.members = { "me", "friend", "stranger" }
	lobby.extra = { friend = { [key] = "primary_weapon=test_breaker" }, stranger = { [key] = "primary_weapon=their_own_gun" } }
	e:frames(70)
	E.eq(#log.spawned, 2, "this machine's look and the friend's")
	E.eq(log.linked[2].parent, theirs)
	E.eq(log.visible[theirs], false)
	E.eq(log.visible[other], nil, "an unknown name keeps the base")
	local lsm = rawget(_G, "LuaSettingsManager")
	E.eq(lsm:get_modified_settings(theirs, 0).weapon_info.power, 75, "another machine's weapon keeps the base's numbers here")
	E.eq(api.weapons.of(theirs), "test_breaker")
end)

test("the_shared_text_is_read_by_slot_and_name_only", function()
	local api = start()
	local got = api.weapons.parse("primary_weapon=a_b;sidearm_weapon=c;backpack=d;x")
	E.eq(got.primary_weapon, "a_b")
	E.eq(got.sidearm_weapon, "c")
	E.eq(got.backpack, nil)
	E.eq(next(api.weapons.parse(nil)), nil)
end)

test("a_custom_weapon_unlocks_by_its_own_rule_or_its_base", function()
	local api, game, e = start({ locked = { "weapon_rifle" } })
	weapon_world(e)
	local P = rawget(_G, "ProgressionSettings")
	P.can_use = function() return false end
	e.opts.modules["y"] = true
	rawget(_G, "require")("y")
	api.weapons.define({ name = "test_base", base = "weapon_rifle", title = "Base" })
	api.weapons.define({ name = "test_free", base = "weapon_rifle", title = "Free", unlock = "free" })
	api.weapons.define({ name = "test_manual", base = "weapon_rifle", title = "Manual", unlock = "manual" })
	E.eq(P.can_use("test_free", "primary_weapon", 0), true)
	E.eq(P.can_use("test_manual", "primary_weapon", 0), false)
	E.eq(api.weapons.unlock("test_manual"), true)
	E.eq(api.weapons.is_unlocked("test_manual"), true)
	E.eq(P.can_use("test_manual", "primary_weapon", 0), true)
	E.eq(api.weapons.unlock("weapon_rifle"), false, "only a custom one")
end)

test("a_weapon_defined_before_the_game_has_its_lists_is_made_before_the_loadout_is_read", function()
	local api, game, e = start()
	local L = rawget(_G, "LoadoutSettings")
	local list = L.primary_weapon
	L.primary_weapon = nil
	local made = api.weapons.define(BREAKER)
	E.eq(made.pending, true)
	L.primary_weapon = list
	weapon_world(e)
	rawget(_G, "Boot").ensure_loadout(1, {})
	E.eq(L.primary_weapon[#L.primary_weapon].name, "test_breaker")
	E.eq(api.weapons.get("test_breaker").slot, "primary_weapon")
end)

return T
