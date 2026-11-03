-- Obsidian API against a fake game that fails the way the real one does.
local E = require "fake_engine"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local function start(opts)
	local e, game, ui = G.start({ "obsidian.api" }, opts)
	e:core().current = e:core().mods_by_id["obsidian.api"]
	return rawget(_G, "obsidian").import("obsidian.api"), game, e, ui
end

local function no_tampering(game)
	E.eq(#game.tampered, 0, "protected values written directly: " .. table.concat(game.tampered, ","))
end

test("every_module_loads_and_the_api_is_exported", function()
	local api = start()
	for _, name in ipairs({ "game", "effects", "edits", "console", "catalog", "world", "player", "gear", "stratagems", "enemies", "mission", "progress", "events", "combat", "vehicles", "lobby", "ui", "audio", "map", "bots", "music" }) do
		E.eq(type(api[name]), "table", name)
	end
end)

test("local_player_is_found_by_ownership_not_position", function()
	local api, game = start()
	E.eq(api.game.my_unit(), game.me)
	E.eq(api.game.is_host(), true)
end)

test("a_command_outside_the_known_list_is_refused_before_the_game_can_assert", function()
	local api, game = start()
	E.eq(api.game.command(game.me, "health", "not_a_command"), false)
	E.eq(api.game.command(game.me, "nonsense", "reset"), false)
	E.eq(#game.commands, 0)
end)

test("a_command_on_a_unit_without_the_component_is_refused", function()
	local api, game = start()
	E.eq(api.game.command(game.weapon, "health", "reset"), false)
end)

test("god_mode_uses_the_games_flags_and_comes_back_after_a_respawn", function()
	local api, game, e = start()
	api.effects.set("god", true)
	E.eq(game.components.health.player_invincibility, true)
	E.eq(game.components.status.status_immune, true)
	E.eq(G.command(game, "health", "set_invincible").data, true)
	game.commands = {}
	game.state.players[1].player_go.avatar_unit = game.enemies[1]
	game.enemies[1].master.health = game.enemies[1].master.health
	e:frames(1)
	game.state.players[1].player_go.avatar_unit = game.me
	e:frames(1)
	E.eq(G.command(game, "health", "set_invincible").data, true, "applied to the new avatar")
	api.effects.set("god", false)
	E.eq(game.components.health.player_invincibility, false)
	no_tampering(game)
end)

test("god_mode_gets_a_downed_player_up", function()
	local api, game, e = start()
	api.effects.set("god", true)
	game.commands = {}
	game.me.slave.health.is_downed = true
	e:frames(1)
	E.eq(G.command(game, "health", "reset") ~= nil, true)
end)

test("no_status_and_god_share_a_flag_without_clearing_each_other", function()
	local api, game = start()
	api.effects.set("god", true)
	api.effects.set("no_status", true)
	api.effects.set("god", false)
	E.eq(game.components.status.status_immune, true, "no_status still on")
	api.effects.set("no_status", false)
	E.eq(game.components.status.status_immune, false)
end)

test("a_session_that_stops_being_cleared_takes_every_change_away_and_gives_it_back", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	api.effects.set("infinite_ammo", true)
	E.eq(api.edits.set_unit("equipment/rifle", "weapon_info.power", 500), true)
	e:frames(31)
	lobby.members = { "me", "stranger" }
	e:frames(31)
	E.eq(game.components.firearm.infinite_ammo, false, "effect off")
	E.eq(game.rifle.weapon_info.power, 75, "edit undone")
	local ok, why = api.edits.set_unit("equipment/rifle", "weapon_info.power", 900)
	E.eq(ok, false, "nothing new while not cleared")
	E.eq(why, "not_cleared")
	E.eq(api.console.run("anything"), false)
	lobby.members = { "me" }
	e:frames(31)
	e:frames(2)
	E.eq(game.components.firearm.infinite_ammo, true, "effect back")
	E.eq(game.rifle.weapon_info.power, 500, "edit back")
end)

test("an_edit_reaches_upgraded_weapons_and_undoes_exactly", function()
	local api, game = start()
	local base, upgraded = game.rifle.weapon_info, game.rifle_upgraded.weapon_info
	E.eq(api.edits.set_unit("equipment/rifle", "weapon_info.power", 200), true)
	E.eq(base.power, 200)
	E.eq(upgraded.power, 200, "the upgrade's own clone")
	E.eq(upgraded.magazine_capacity, 45, "what the upgrade sets stays")
	E.eq(game.rifle.weapon_info, base, "same table: live entities hold it")
	E.eq(game.rifle_upgraded.weapon_info, upgraded)
	E.eq(api.edits.count(), 1)
	api.edits.revert_all()
	E.eq(base.power, 75)
	E.eq(upgraded.power, 75)
	E.eq(api.edits.count(), 0)
end)

test("an_edit_is_recorded_with_the_mod_that_made_it", function()
	local api = start()
	api.edits.set_unit("equipment/rifle", "weapon_info.power", 200, { target = "Rifle" })
	local list = api.edits.list()
	E.eq(list[1].owner, "obsidian.api")
	E.eq(list[1].target, "Rifle")
end)

test("changing_a_field_twice_keeps_one_record_and_the_first_original", function()
	local api, game = start()
	api.edits.set_unit("equipment/rifle", "weapon_info.power", 200)
	api.edits.set_unit("equipment/rifle", "weapon_info.power", 300)
	E.eq(api.edits.count(), 1)
	api.edits.revert_all()
	E.eq(game.rifle.weapon_info.power, 75)
end)

test("reverting_one_edit_leaves_the_others", function()
	local api, game = start()
	api.edits.set_unit("equipment/rifle", "weapon_info.power", 200)
	api.edits.set_unit("equipment/rifle", "weapon_info.firerate", 900)
	api.edits.revert_where("equipment/rifle", "weapon_info.power")
	E.eq(game.rifle.weapon_info.power, 75)
	E.eq(game.rifle.weapon_info.firerate, 900)
end)

test("edits_are_checked", function()
	local api = start()
	local set = function(...) return select(2, api.edits.set_unit("equipment/rifle", ...)) end
	E.eq(set("weapon_info.power", "strong"), "bad_type")
	E.eq(set("weapon_info.power", 0 / 0), "bad_value")
	E.eq(set("weapon_info.power", 1 / 0), "bad_value")
	E.eq(set("weapon_info.power", 99999, { max = 20000 }), "out_of_range")
	E.eq(set("weapon_info.nothing", 1), "no_field")
	E.eq(set("missing.table.field", 1), "no_field")
	E.eq(api.edits.count(), 0)
end)

test("protected_keys_cant_be_edited_whatever_table_they_are_in", function()
	local api = start()
	for _, key in ipairs({ "hitpoints_real", "rounds", "num_mags", "cooldown_timer", "uses_left", "num_grenades", "defend_timer", "extra_lives" }) do
		local t = { [key] = 1 }
		local ok, why = api.edits.set_table("t", t, key, 2)
		E.eq(ok, false, key)
		E.eq(why, "protected", key)
		E.eq(t[key], 1, key)
	end
end)

test("paid_content_switches_cant_be_edited", function()
	local api = start()
	local dlc = rawget(_G, "DlcSettings")
	dlc.unlock_all = false
	local ok, why = api.edits.set_table("t", dlc, "unlock_all", true)
	E.eq(ok, false)
	E.eq(why, "protected")
	local progression = rawget(_G, "ProgressionSettings")
	progression.unlock_all = false
	E.eq(api.edits.set_table("t", progression, "unlock_all", true), false)
end)

test("the_console_collects_what_the_game_registers_and_refuses_what_it_must", function()
	local api, game = start()
	local debug_table = { console = "the dummy" }
	rawset(_G, "Debug", debug_table)
	rawget(_G, "Application").build = function() return "release" end
	api.console.installed = false
	E.eq(api.console.install(), true)
	debug_table.console:register_plugin("state_game", {
		time_scale = function(args) rawset(_G, "SCALE", args[2]) return { error = false, message = "ok" } end,
		push_planet = function() rawset(_G, "PUSHED", true) end,
		boom = function() error("broken") end,
	})
	E.eq(api.console.run("time_scale 2"), true)
	E.eq(rawget(_G, "SCALE"), "2")
	local ok, why = api.console.run("push_planet 0 1")
	E.eq(ok, false)
	E.eq(why, "denied")
	E.eq(rawget(_G, "PUSHED"), nil, "a command that reaches the backend never runs")
	E.eq(api.console.run("no_such_command"), false)
	E.eq(api.console.run("boom"), false, "a failing command is an answer, not a crash")
	local names = table.concat(api.console.commands(), " ")
	E.has(names, "time_scale")
	E.lacks(names, "push_planet")
	debug_table.console:clear()
	E.eq(#api.console.commands(), 0, "a new state starts clean")
	debug_table.console:do_multiple_commands_string("x,y")
	E.eq(debug_table.console:no_such_method(), nil, "the rest of the console stays a dummy")
end)

test("the_console_leaves_a_real_debug_module_alone", function()
	local api = start()
	local debug_table = { console = "real" }
	rawset(_G, "Debug", debug_table)
	rawget(_G, "Application").build = function() return "dev" end
	api.console.installed = false
	api.console.install()
	E.eq(debug_table.console, "real")
end)

test("every_command_that_reaches_the_backend_or_wipes_a_save_is_denied", function()
	local api = start()
	for _, name in ipairs({ "push_planet", "push_defend_event", "push_attack_event", "push_bogus_statistics", "clear_save_and_tus",
		"clear_achievemenets", "unlock_all", "unlock_dlc", "season_end", "console_command" }) do
		E.eq(api.console.denied(name), true, name)
	end
	E.eq(api.console.denied("upgrade_all"), false, "the player's own progression is theirs")
end)

test("paid_items_a_player_doesnt_own_are_left_out_of_every_list", function()
	local api = start()
	for _, item in ipairs(api.catalog.primaries()) do E.eq(item.name ~= "weapon_dlc", true) end
	for _, item in ipairs(api.catalog.stratagems()) do E.eq(item.name ~= "stratagem_dlc", true) end
	E.eq(api.catalog.stratagem("stratagem_dlc"), nil)
	for _, item in ipairs(api.catalog.cosmetics("helmet")) do E.eq(item.name ~= "cus_helmet_dlc", true) end
end)

test("support_weapons_are_found_through_their_crate", function()
	local api = start()
	local list = api.catalog.support_weapons()
	E.eq(#list, 2)
	E.eq(list[1].unit_path, "equipment/mg")
end)

test("the_mortar_and_the_other_anti_tank_weapons_are_support_weapons", function()
	local api = start()
	E.eq(api.catalog.support_weapons()[2].name, "stratagem_mortar", "its subtypes are weapon and anti_tank")
end)

test("a_weapon_comes_with_the_upgrades_the_player_owns", function()
	local api, game = start()
	E.eq(api.gear.give_weapon("primary_weapon", "equipment/rifle"), true)
	E.eq(G.command(game, "inventory", "set_weapon").data.upgrade_value, 5)
	game.commands = {}
	E.eq(api.gear.give_weapon("support_weapon", "equipment/mg"), true)
	E.eq(G.command(game, "inventory", "set_weapon").data.upgrade_value, 2, "kept under the stratagem's name")
	game.commands = {}
	E.eq(api.gear.give_weapon("support_weapon", "equipment/mortar"), true)
	E.eq(G.command(game, "inventory", "set_weapon").data.upgrade_value, 0, "none owned")
end)

test("a_weapon_can_ask_for_its_upgrades_and_the_game_range_is_kept", function()
	local api, game = start()
	E.eq(api.gear.give_weapon("primary_weapon", "equipment/rifle", 3), true)
	E.eq(G.command(game, "inventory", "set_weapon").data.upgrade_value, 3)
	game.commands = {}
	for _, bad in ipairs({ -1, 16, 1.5, 0 / 0, "all" }) do
		local ok, why = api.gear.give_weapon("primary_weapon", "equipment/rifle", bad)
		E.eq(ok, false)
		E.eq(why, "out_of_range")
	end
	E.eq(#game.commands, 0, "the upgradeable component would have asserted")
end)

test("a_perk_swap_takes_the_old_one_off_and_sets_up_the_new_one", function()
	local api, game = start()
	local inv = game.me.master.inventory
	E.eq(api.gear.give_perk("perk_lasersight"), true)
	E.eq(table.concat(game.perk_calls, ","), "cardio remove_master,laser setup_master,laser setup_slave")
	E.eq(inv.perk, 2)
	E.eq(inv.dirty, true, "peers hear of it")
	E.eq(G.command(game, "inventory", "set_perk"), nil, "the command skips all of that")
	game.perk_calls = {}
	E.eq(api.gear.give_perk("perk_cardio"), true)
	E.eq(table.concat(game.perk_calls, ","), "laser remove_slave,cardio setup_master")
end)

test("a_perk_can_be_given_when_the_player_has_none", function()
	local api, game = start()
	game.me.master.inventory.perk = 0
	E.eq(api.gear.give_perk("perk_cardio"), true)
	E.eq(table.concat(game.perk_calls, ","), "cardio setup_master")
end)

test("laser_sight_stays_out_of_coop_where_peers_would_run_it_with_no_setup", function()
	local api, game = start()
	rawget(_G, "NetworkGlobal").lobby.members = function() return { "me", "friend" } end
	local ok, why = api.gear.give_perk("perk_lasersight")
	E.eq(ok, false)
	E.eq(why, "alone_only")
	E.eq(#game.perk_calls, 0)
	E.eq(api.gear.give_perk("perk_cardio"), true, "other perks have no such code")
end)

test("a_perk_that_doesnt_exist_is_refused", function()
	local api = start()
	local ok, why = api.gear.give_perk("perk_nothing")
	E.eq(ok, false)
	E.eq(why, "not_available")
end)

test("a_stratagem_edit_holds_at_every_upgrade_level_and_undoes_as_one", function()
	local api = start()
	local s = api.catalog.stratagem("stratagem_resupply")
	E.eq(api.stratagems.edit("stratagem_resupply", "cooldown_time", 30), true)
	E.eq(s.cooldown_time, 30)
	E.eq(s.upgrades[1].cooldown_time, 30, "level 1 sets its own")
	E.eq(s.upgrades[2].cooldown_time, nil, "level 2 doesn't, and inherits")
	E.eq(s.upgrades[3].cooldown_time, 30)
	E.eq(api.edits.count(), 1, "one change on the list")
	api.edits.revert_all()
	E.eq(s.cooldown_time, 120)
	E.eq(s.upgrades[1].cooldown_time, 90)
	E.eq(s.upgrades[3].cooldown_time, 60)
end)

test("a_stratagem_edit_is_checked_and_paid_stratagems_are_out", function()
	local api = start()
	local ok, why = api.stratagems.edit("stratagem_resupply", "cooldown_time", 5000)
	E.eq(ok, false)
	E.eq(why, "out_of_range")
	E.eq(select(2, api.stratagems.edit("stratagem_dlc", "cooldown_time", 5)), "not_available")
end)

test("the_enemy_cap_lifts_both_limits_and_gives_both_back", function()
	local api, game, e = start()
	local em = game.state.encounter_manager
	em.max_num_enemies = 8
	api.effects.set("enemy_cap", 24)
	E.eq(rawget(_G, "GameSettings").MAX_NUM_ALLOWED_ENEMIES, 24)
	E.eq(em.max_num_enemies, 24, "the limit waves stop at")
	em.max_num_enemies = 10
	e:frames(1)
	E.eq(em.max_num_enemies, 24, "held against the mission's start writing it")
	api.effects.set("enemy_cap", false)
	E.eq(rawget(_G, "GameSettings").MAX_NUM_ALLOWED_ENEMIES, 16)
	E.eq(em.max_num_enemies, 8)
end)

test("auto_gives_the_mission_its_own_light_back", function()
	local api, game = start()
	api.effects.set("time_of_day", "night")
	E.eq(game.shading_setting, "night")
	api.effects.set("time_of_day", false)
	E.eq(game.shading_setting, "dusk")
end)

test("finishing_objectives_lets_a_player_out_of_a_terminal", function()
	local api, game = start()
	api.mission.finish_objectives()
	E.eq(G.command(game, "character", "exit_interact_objective"), nil, "nobody is in one")
	game.me.master.character.current_interact_objective_unit = "terminal"
	api.mission.finish_objectives()
	E.eq(G.command(game, "character", "exit_interact_objective").unit, game.me)
end)

test("the_consoles_spawn_enemy_runs_only_alone", function()
	local api = start()
	local debug_table = { console = "the dummy" }
	rawset(_G, "Debug", debug_table)
	rawget(_G, "Application").build = function() return "release" end
	api.console.installed = false
	api.console.install()
	debug_table.console:register_plugin("enemy_spawner", { spawn_enemy = function() rawset(_G, "SPAWNED", true) end })
	rawget(_G, "NetworkGlobal").lobby.members = function() return { "me", "friend" } end
	local ok, why = api.console.run("spawn_enemy characters/bug_scout/bug_scout")
	E.eq(ok, false)
	E.eq(why, "alone_only")
	E.eq(rawget(_G, "SPAWNED"), nil, "a peer without the package couldn't mirror it")
	rawget(_G, "NetworkGlobal").lobby.members = function() return { "me" } end
	E.eq(api.console.run("spawn_enemy characters/bug_scout/bug_scout"), true)
	E.eq(rawget(_G, "SPAWNED"), true)
end)

test("a_display_name_falls_back_to_something_readable", function()
	local api = start()
	E.eq(api.catalog.text("weapon_rifle"), "Rifle")
	E.eq(api.catalog.text("weapon_pulse_laser"), "Pulse laser", "a missing key")
end)

test("names_the_font_cant_draw_fall_back_too", function()
	local api = start()
	rawget(_G, "LocalizationManager").string = function() return "\208\159\209\131\208\187\209\143" end
	E.eq(api.catalog.text("weapon_rifle"), "Rifle", "cyrillic isn't in the font")
end)

test("refill_treats_beam_weapons_as_heat", function()
	local api, game = start()
	E.eq(api.gear.refill(), true)
	E.eq(game.commands[1].command, "set_ammo")
	E.eq(game.commands[1].data.num_mags, 1000000)
	no_tampering(game)
end)

test("instant_reload_goes_through_the_games_setter", function()
	local api, game, e = start()
	api.effects.set("instant_reload", true)
	e:frames(1)
	E.eq(game.weapon.master.firearm.reload_time, 0.01)
	E.eq(game.protected[1], "reload_time")
	no_tampering(game)
end)

test("stratagem_uses_are_restored_through_the_games_setter", function()
	local api, game = start()
	E.eq(api.stratagems.refill_uses(), true)
	E.eq(game.me.master.inventory.stratagems[1].uses_left, 4)
	no_tampering(game)
end)

test("cooldowns_are_reset_by_command_never_by_a_write", function()
	local api, game = start()
	E.eq(api.stratagems.reset_cooldowns(), true)
	E.eq(G.command(game, "inventory", "set_stratagem_info").data.cooldown_timer, 0)
	E.eq(G.command(game, "inventory", "broadcast_stratagem_cooldown") ~= nil, true)
	E.eq(game.me.master.inventory.stratagems[1].cooldown_timer, 30)
	no_tampering(game)
end)

test("any_arrow_calls_the_chosen_stratagem_only_when_on", function()
	local api = start()
	local find = rawget(_G, "LoadoutSettings").find_stratagem
	rawget(_G, "require")("lua/network/network_global")
	E.eq(api.stratagems.find(find, { "up" }), "from the arrows", "off")
	api.stratagems.select(2)
	api.effects.set("any_arrow", true)
	E.eq(api.stratagems.find(find, { "up" }), 2)
	E.eq(api.stratagems.find(find, {}), "from the arrows", "no arrow yet")
end)

test("a_stratagem_drops_with_no_flare_except_reinforce", function()
	local api, game = start()
	E.eq(api.stratagems.drop("stratagem_resupply", api.world.cursor()), true)
	E.eq(game.pods ~= nil, true)
	local ok, why = api.stratagems.drop("stratagem_reinforce", api.world.cursor())
	E.eq(ok, false)
	E.eq(why, "needs_flare")
	local ok2, why2 = api.stratagems.drop("stratagem_dlc", api.world.cursor())
	E.eq(ok2, false, "paid content")
	E.eq(why2, "not_available")
end)

test("lives_change_only_through_the_games_setter", function()
	local api, game = start()
	api.player.set_lives(5)
	E.eq(game.lives, 5)
	E.eq(game.record.extra_lives, 1, "the value itself is the game's to write")
	no_tampering(game)
end)

test("respawn_needs_a_dead_player", function()
	local api, game = start()
	local ok, why = api.player.respawn()
	E.eq(ok, false)
	E.eq(why, "alive")
	game.me.slave.health.is_dead = true
	E.eq(api.player.respawn(), true)
	E.eq(game.record.spawn_timer, 0.01, "never zero")
end)

test("auto_respawn_starts_the_countdown_once_per_death", function()
	local api, game, e = start()
	local record, health = game.record, game.me.slave.health
	record.spawn_timer, record.extra_lives = 0, 0
	api.effects.set("auto_respawn", true)
	e:frames(1)
	E.eq(record.spawn_timer, 0, "alive")
	health.is_dead = true
	e:frames(1)
	E.eq(record.spawn_timer, 3, "dead, with no lives left")
	record.spawn_timer = 0
	e:frames(3)
	E.eq(record.spawn_timer, 0, "not again while the drop is on its way")
	health.is_dead = false
	e:frames(1)
	health.is_dead = true
	record.spawn_timer = 7
	e:frames(1)
	E.eq(record.spawn_timer, 7, "a respawn the game started is left alone")
	health.is_dead = false
	e:frames(1)
	api.effects.set("auto_respawn", 1.5)
	health.is_dead, record.spawn_timer = true, 0
	e:frames(1)
	E.eq(record.spawn_timer, 1.5, "the seconds given")
end)

test("auto_respawn_waits_for_the_first_drop_and_stops_after_extraction", function()
	local api, game, e = start()
	local record = game.record
	record.spawn_timer = 0
	record.player_go.avatar_unit = nil
	api.effects.set("auto_respawn", true)
	e:frames(2)
	E.eq(record.spawn_timer, 0, "no avatar yet: the drop hasn't happened")
	record.player_go.avatar_unit = game.me
	e:frames(1)
	record.player_go.has_extracted = true
	game.me.slave.health.is_dead = true
	e:frames(1)
	E.eq(record.spawn_timer, 0, "extracted")
end)

test("helping_a_teammate_goes_to_its_owner", function()
	local api, game = start()
	local other = game.enemies[2]
	local go = { avatar_unit = other }
	function go:owned() return false end
	function go:exists() return true end
	function go:get(k) return self[k] end
	function go:has(k) return self[k] ~= nil end
	game.state.players[2] = { player_go = go, pad_number = 1 }
	E.eq(api.player.heal_everyone(), true)
	E.eq(game.rpc[1][2], "rpc_heal_request")
	E.eq(G.command(game, "health", "reset").unit, game.me, "this player is healed by command")
end)

test("teleport_finds_the_objective_and_the_extraction", function()
	local api, game = start()
	E.eq(api.player.teleport_objective(), true)
	E.eq(G.command(game, "mover", "position_teleport").data.x, 9)
	game.commands = {}
	E.eq(api.player.teleport_extraction(), true)
	E.eq(G.command(game, "mover", "position_teleport").data.x, 7)
end)

test("teleport_refuses_in_a_seat", function()
	local api, game = start()
	game.me.master.mover = nil
	local ok, why = api.player.teleport_cursor()
	E.eq(ok, false)
	E.eq(why, "seated")
end)

test("an_enemy_whose_package_isnt_loaded_never_spawns", function()
	local api, game = start()
	E.eq(api.enemies.spawn("bug_scout", 1), true, "bugs are loaded")
	E.eq(game.spawned[1], "characters/bug_scout/bug_scout")
	local ok, why = api.enemies.spawn("cyborg_soldier", 1)
	E.eq(ok, false)
	E.eq(why, "not_loaded")
	E.eq(#game.spawned, 1)
end)

test("large_spawns_go_in_batches_the_spawner_can_place", function()
	local api, game = start()
	E.eq(api.enemies.spawn("bug_scout", 23), true)
	E.eq(table.concat(game.spawned, ","), "10,10,3")
end)

test("enemies_spawn_at_a_position_given", function()
	local api, game = start()
	E.eq(api.enemies.spawn("bug_scout", 3, { x = 5, y = 6, z = 1 }), true)
	local V = rawget(_G, "Vector3")
	E.eq(game.spawned_at.x, 5)
	E.eq(game.spawned_at.y, 6)
	E.eq(api.enemies.spawn("bug_scout", 3, V(7, 8, 0)), true, "or a vector")
	E.eq(game.spawned_at.x, 7)
	-- The game's type() names its vectors.
	local real_type = type
	rawset(_G, "type", function(v) if real_type(v) == "table" and v.x and v.y and v.z then return "vector3" end return real_type(v) end)
	local ok = api.enemies.spawn("bug_scout", 3, V(9, 8, 0))
	rawset(_G, "type", real_type)
	E.eq(ok, true, "a vector the game's type() calls vector3")
	E.eq(game.spawned_at.x, 9)
	E.eq(select(2, api.enemies.spawn("bug_scout", 3, "sideways")), "bad_type")
end)

test("a_position_in_the_level_has_a_place_on_the_screen", function()
	local api = start()
	local V = rawget(_G, "Vector3")
	rawget(_G, "CameraManager").world_to_screen = function(_, p) return V(p.x * 10, p.y * 10, p.z) end
	rawget(_G, "Application").resolution = function() return 1920, 1080 end
	local x, y, on = api.world.to_screen({ x = 10, y = 20, z = 5 })
	E.eq(x, 100) E.eq(y, 200) E.eq(on, true)
	E.eq(select(3, api.world.to_screen({ x = 500, y = 20, z = 5 })), false, "past the right edge")
	E.eq(select(3, api.world.to_screen({ x = 10, y = 20, z = -5 })), false, "behind the camera")
end)

test("a_value_shared_through_the_lobby_reaches_the_others", function()
	local api = start()
	local lobby = rawget(_G, "NetworkGlobal").lobby
	local data = { friend = {} }
	lobby.members = function() return { "me", "friend" } end
	lobby.set_member_data = function(_, key, value) data.me = data.me or {} data.me[key] = value end
	lobby.member_data = function(_, peer, key) return (data[peer] or {})[key] end
	E.eq(api.lobby.share("run", "5;1"), true)
	local key
	for k in pairs(data.me) do key = k end
	E.eq(key, "mod:obsidian.api:run", "under the mod's id and the name")
	data.friend[key] = "7;2"
	local shared = api.lobby.shared("run")
	E.eq(shared.friend, "7;2")
	E.eq(shared.me, nil, "not this machine's own")
	E.eq(select(2, api.lobby.share("run", string.rep("x", 201))), "out_of_range")
	E.eq(select(2, api.lobby.share("", "x")), "bad_type")
end)

test("groups_are_for_the_host", function()
	local api = start({ host = false })
	local ok, why = api.enemies.spawn("bug_scout", 5)
	E.eq(ok, false)
	E.eq(why, "host_only")
	E.eq(api.enemies.spawn("bug_scout", 1), true, "a single spawn is a request to the host")
end)

test("a_kill_goes_to_the_owner_and_credits_the_player", function()
	local api, game = start()
	E.eq(api.enemies.kill_all(), true)
	local kill = G.command(game, "health", "set_hitpoints")
	E.eq(kill.data.new_hitpoints, -10, "the unit's own constitution")
	E.eq(kill.data.owner_unit, game.me)
	E.eq(game.rpc[1][2], "rpc_set_hitpoints_request", "the other peer's enemy")
end)

test("no_spawns_holds_and_lets_go", function()
	local api, game, e = start()
	api.effects.set("no_spawns", true)
	E.eq(game.encounters_off, true)
	E.eq(game.patrols_off, true)
	game.encounters_off = false
	for _ = 1, 70 do e:frames(1) end
	E.eq(game.encounters_off, true, "held against the game turning it back on")
	api.effects.set("no_spawns", false)
	E.eq(game.encounters_off, false)
end)

test("blind_enemies_get_their_settings_back", function()
	local api = start()
	local scout = rawget(_G, "LuaSettingsManager"):get_settings_by_settings_path("characters/bug_scout/bug_scout")
	local threat = scout.enemy_info.threat_calculation
	api.effects.set("blind", true)
	E.eq(scout.enemy_info.threat_calculation(), -2)
	E.eq(scout.enemy_info.aggro_range, 1, "never 0: it is a divisor")
	api.effects.set("blind", false)
	E.eq(scout.enemy_info.threat_calculation, threat)
	E.eq(scout.enemy_info.aggro_range, 80)
end)

test("one_failing_objective_doesnt_stop_the_others", function()
	local api, game = start()
	E.eq(api.mission.finish_objectives(), true)
	E.eq(game.objective_runs[1], "objective_interact_object")
end)

test("objectives_and_the_end_of_a_mission_are_the_hosts", function()
	local api, game = start({ host = false })
	for _, fn in ipairs({ "finish_objectives", "win", "abort", "spawn_samples" }) do
		local ok, why = api.mission[fn]()
		E.eq(ok, false, fn)
		E.eq(why, "host_only", fn)
	end
	E.eq(game.completed, nil)
end)

test("winning_marks_the_player_extracted_first", function()
	local api, game = start()
	E.eq(api.mission.win(), true)
	E.eq(game.completed, true)
	E.eq(game.extraction_done, true)
	E.eq(game.record.player_go.has_extracted, true)
end)

test("a_refused_action_leaves_the_mission_alone", function()
	local api, game = start({ host = false })
	E.eq(api.mission.win(), false)
	E.eq(game.completed, nil)
end)

test("practice_starts_only_from_the_ship_and_pays_nothing", function()
	local api, game, e, ui = start()
	local ok, why = api.mission.practice({ time = 1, environment = 1, enemy = 1, kind = 1, difficulty = 5 })
	E.eq(ok, false)
	E.eq(why, "not_on_ship")
	ui.state = "menu"
	rawset(math, "create_weak_unique_identifier", function() return "id" end)
	E.eq(api.mission.practice({ time = 2, environment = 3, enemy = 2, kind = 1, difficulty = 7 }), true)
	E.eq(game.changed[2].is_training, true)
	E.eq(game.changed[2].mission_seed, 777)
	E.eq(game.seed_parts[1], 1, "time, zero based")
	E.eq(game.seed_parts[3], 2, "environment")
	E.eq(game.seed_parts[7], 7, "difficulty")
end)

local LOCKED = { locked = { "weapon_rifle", "perk_cardio", "stratagem_machinegun", "stratagem_exosuit", "stratagem_jumppack", "cus_helmet_a" } }

-- The rifle is the fake game's default weapon, so it has to stop being that to be locked.
local function start_locked()
	local api, game, e, ui = start(LOCKED)
	rawget(_G, "LoadoutSettings").default_loadout.primary_weapon = "weapon_other"
	game.save.user.player_xp = game.save.user.player_xp + 1
	return api, game, e, ui
end

local function has(list, name)
	for _, item in ipairs(list) do
		if item.name == name then return true end
	end
	return false
end

test("what_a_player_hasnt_unlocked_can_still_be_called", function()
	local api, game = start_locked()
	E.eq(api.catalog.unlocked("stratagem_machinegun"), false, "the rule is about ownership, not progress")
	E.eq(has(api.catalog.primaries(), "weapon_rifle"), true, "a weapon")
	E.eq(has(api.catalog.perks(), "perk_cardio"), true, "a perk")
	E.eq(has(api.catalog.support_weapons(), "stratagem_machinegun"), true, "the weapon in its crate")
	E.eq(has(api.catalog.vehicles(), "stratagem_exosuit"), true, "a vehicle")
	E.eq(has(api.catalog.cosmetics("helmet"), "cus_helmet_a"), true, "a helmet")
	E.eq(api.gear.give_weapon("primary_weapon", "equipment/rifle"), true)
	E.eq(api.gear.give_perk("perk_cardio"), true)
	E.eq(api.stratagems.give("stratagem_machinegun"), true)
	E.eq(api.stratagems.call("stratagem_machinegun", api.world.cursor()), true)
end)

test("dlc_goes_by_ownership", function()
	local api = start()
	E.eq(has(api.catalog.primaries(), "weapon_dlc"), false, "not owned")
	E.eq(select(2, api.gear.give_weapon("primary_weapon", "equipment/dlc")), "not_available")
	E.eq(select(2, api.stratagems.give("stratagem_dlc")), "not_available")
	E.eq(select(2, api.stratagems.call("stratagem_dlc", api.world.cursor())), "not_available")
	rawget(_G, "DlcSettings").can_use = function(name) return name == "weapon_dlc" end
	E.eq(has(api.catalog.primaries(), "weapon_dlc"), true, "owned, whatever the progress")
	E.eq(api.gear.give_weapon("primary_weapon", "equipment/dlc"), true)
end)

test("a_path_nothing_lists_can_still_be_given", function()
	local api = start()
	E.eq(api.catalog.unowned_path("equipment/made_by_a_mod"), false)
	E.eq(api.gear.give_weapon("primary_weapon", "equipment/made_by_a_mod", 0), true, "the API keeps that power")
end)

test("every_player_keeps_what_they_start_with", function()
	local api, game = start({ locked = { "stratagem_reinforce", "weapon_pistol", "perk_lasersight" } })
	E.eq(api.catalog.unlocked("stratagem_reinforce"), true, "always carried")
	E.eq(api.catalog.unlocked("weapon_pistol"), true, "the default sidearm")
	E.eq(api.catalog.unlocked("perk_lasersight"), true, "the default perk")
end)

test("a_rank_or_a_reward_unlocks_an_item_as_soon_as_it_comes", function()
	local api, game = start_locked()
	E.eq(api.catalog.unlocked("late_item"), false, "a rank not reached")
	game.save.user.player_xp = 5000
	E.eq(api.catalog.unlocked("late_item"), true, "reached")
	E.eq(api.catalog.unlocked("weapon_rifle"), false)
	local rewards = game.save.user.item_unlocks
	rewards[#rewards + 1] = "weapon_rifle"
	E.eq(api.catalog.unlocked("weapon_rifle"), true, "earned")
end)

test("a_paid_pack_the_player_owns_counts", function()
	local api = start()
	rawget(_G, "DlcSettings").can_use = function(name) return name == "weapon_dlc" end
	E.eq(api.catalog.unlocked("weapon_dlc"), true)
	E.eq(api.catalog.unlocked("stratagem_dlc"), false)
end)

test("there_is_no_way_to_change_xp", function()
	local api, game = start()
	for name in pairs(api.progress) do E.lacks(string.lower(name), "xp", "progress." .. name) end
	api.progress.rank()
	E.eq(game.save.user.player_xp, 1000)
end)

test("unlocking_items_merges_and_skips_paid_content", function()
	local api, game, e, ui = start()
	ui.state = "menu"
	game.state.active_screens = {}
	E.eq(api.progress.unlock_items(), true)
	local all = " " .. table.concat(game.save.user.item_unlocks, " ") .. " "
	E.has(all, " owned_item ", "kept what was there")
	E.has(all, " reward_item ")
	E.has(all, " rank_item ")
	E.has(all, " cus_helmet_a ")
	E.lacks(all, "weapon_dlc")
	E.lacks(all, "cus_helmet_dlc")
	E.lacks(all, " boost ", "a repeatable reward, not an item")
	E.lacks(all, "cus_cape_tutorial")
	E.eq(game.save.user.player_xp, 1000, "XP untouched")
	E.eq(game.saved > 0, true, "saved")
end)

test("save_edits_wait_for_the_ships_screens_to_close", function()
	local api, game, e, ui = start()
	ui.state = "menu"
	game.state.active_screens = { { name = "menu_screen_armory" } }
	local ok, why = api.progress.set_samples(100)
	E.eq(ok, false)
	E.eq(why, "close_screen")
	E.eq(game.save.user.research_samples, 50)
	local ok2, why2 = api.progress.upgrade_all()
	E.eq(ok2, false)
	E.eq(why2, "close_screen")
end)

test("samples_stop_at_the_games_cap", function()
	local api, game, e, ui = start()
	ui.state = "menu"
	game.state.active_screens = {}
	api.progress.set_samples(999999)
	E.eq(game.save.user.research_samples, 3168)
end)

test("steam_achievements_are_the_players", function()
	local api, game, e = start()
	e.opts.modules["x"] = true
	rawget(_G, "require")("x")
	rawget(_G, "UserManager").unlock_trophy(rawget(_G, "UserManager"), "trophy_gold_all_upgrades")
	E.eq(game.trophy, "trophy_gold_all_upgrades")
end)

test("a_shake_is_dropped_only_when_asked", function()
	local api, game, e = start()
	e.opts.modules["x"] = true
	rawget(_G, "require")("x")
	rawget(_G, "CameraManager").add_shake(rawget(_G, "CameraManager"))
	E.eq(game.shakes, 1)
	api.effects.set("no_camera_shake", true)
	rawget(_G, "CameraManager").add_shake(rawget(_G, "CameraManager"))
	E.eq(game.shakes, 1)
end)

local function hooked(e)
	e.opts.modules["x"] = true
	rawget(_G, "require")("x")
end

local function subscribe(e, api, name, fn)
	e:core().current = e:core().mods_by_id["obsidian.api"]
	return api.events.on(name, fn)
end

test("a_mod_hears_a_game_event_with_what_the_game_sends", function()
	local api, game, e = start()
	local heard
	E.eq(api.events.on("on_enemy_killed", function(...) heard = { ... } end), true)
	e:frames(1)
	game.state.event_delegate:trigger("on_enemy_killed", "bug", "bullet", "player")
	E.eq(table.concat(heard, ","), "bug,bullet,player")
end)

test("nothing_joins_the_games_delegate_until_a_mod_asks", function()
	local api, game, e = start()
	e:frames(1)
	E.eq(next(game.state.event_delegate.events), nil)
end)

test("every_new_state_gets_a_listener_and_no_delegate_gets_two", function()
	local api, game, e = start()
	local count = 0
	api.events.on("on_player_death", function() count = count + 1 end)
	e:frames(3)
	game.state.event_delegate = G.delegate()
	e:frames(3)
	game.state.event_delegate:trigger("on_player_death", "unit")
	E.eq(count, 1, "heard once, on the new delegate")
	E.lacks(e:log("obsidianapi"), "already registered", "the game's assert")
end)

test("a_failing_callback_stops_neither_the_game_nor_the_others", function()
	local api, game, e = start()
	local reached = false
	api.events.on("on_objective_completed", function() error("broken mod") end)
	api.events.on("on_objective_completed", function() reached = true end)
	e:frames(1)
	game.state.event_delegate:trigger("on_objective_completed")
	E.eq(reached, true)
end)

test("events_stop_when_the_session_stops_being_cleared", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	local count = 0
	api.events.on("on_enemy_spawned", function() count = count + 1 end)
	e:frames(2)
	game.state.event_delegate:trigger("on_enemy_spawned", "bug", "name")
	E.eq(count, 1)
	lobby.members = { "me", "stranger" }
	e:frames(31)
	game.state.event_delegate:trigger("on_enemy_spawned", "bug", "name")
	E.eq(count, 1, "the delegate isn't gated by the loader")
end)

test("as_a_client_mod_it_refuses_what_changes_the_game", function()
	local api = start({ scope = "client" })
	for name, call in pairs({
		heal = function() return api.player.heal() end,
		kill = function() return api.enemies.kill_near(20) end,
		effect = function() return api.effects.set("god", true) end,
		edit = function() return api.edits.set_unit("equipment/wep_assaultrifle_01", "weapon_info.power", 150) end,
		command = function() return api.game.command({}, "health", "reset") end,
		console = function() return api.console.run("help") end,
		ping = function() return api.ui.ping(nil) end,
	}) do
		local ok, why = call()
		E.eq(ok, false, name)
		E.eq(why, "needs_session", name)
	end
end)

test("as_a_client_mod_it_still_reads_and_shows", function()
	local api = start({ scope = "client" })
	E.eq(type(api.catalog.stratagems()), "table")
	E.eq(api.game.in_mission(), true)
	E.eq(type(api.mission.objectives()), "table")
	E.eq(type(api.lobby.players()), "table")
	E.eq(api.ui.hint("words"), true)
	E.eq(api.events.on("on_avatar_extract", function() end), true)
end)

test("as_a_client_mod_it_leaves_the_game_alone", function()
	local _, _, e = start({ scope = "client" })
	E.eq(e:log():find("hook installed: HealthComponent", 1, true), nil)
	E.eq(e:log():find("hook installed: UserManager.unlock_trophy by obsidian.api", 1, true), nil)
end)

test("as_a_client_mod_it_hears_events_in_any_lobby", function()
	local api, game, e = start({ scope = "client", lobby = { my_id = "me", members = { "me", "stranger" }, data = {} } })
	local count = 0
	api.events.on("on_avatar_extract", function() count = count + 1 end)
	e:frames(31)
	E.eq(rawget(_G, "obsidian").session.cleared(), false)
	game.state.event_delegate:trigger("on_avatar_extract", "unit", "shuttle")
	E.eq(count, 1)
end)

test("a_guarded_function_waits_for_a_cleared_session", function()
	local api, _, e = start({ lobby = { my_id = "me", members = { "me", "stranger" }, data = {} } })
	e:frames(31)
	E.eq(rawget(_G, "obsidian").session.cleared(), false)
	local ok, why = api.player.heal()
	E.eq(ok, false)
	E.eq(why, "not_cleared")
	E.eq(type(api.catalog.stratagems()), "table")
end)

test("every_function_it_lists_as_free_exists", function()
	local _, _, e = start()
	E.eq(e:log():find("guard:", 1, true), nil, e:log())
end)

test("a_mod_that_was_switched_off_hears_nothing", function()
	local api, game, e = start()
	local count = 0
	api.events.on("on_avatar_extract", function() count = count + 1 end)
	e:frames(1)
	e:core().mods_by_id["obsidian.api"].off = true
	game.state.event_delegate:trigger("on_avatar_extract", "unit", "shuttle")
	E.eq(count, 0)
end)

test("a_callback_runs_as_the_mod_that_asked", function()
	local api, game, e = start()
	local who
	api.events.on("on_avatar_extract", function() who = rawget(_G, "obsidian").caller() end)
	e:core().current = nil
	e:frames(1)
	game.state.event_delegate:trigger("on_avatar_extract", "unit", "shuttle")
	E.eq(who, "obsidian.api")
end)

test("an_event_can_be_left_and_a_callback_can_leave_during_its_event", function()
	local api, game, e = start()
	local count = 0
	local function once()
		count = count + 1
		api.events.off("on_stratagem_activated", once)
	end
	api.events.on("on_stratagem_activated", once)
	e:frames(1)
	game.state.event_delegate:trigger("on_stratagem_activated", "peer", {}, "pos", "unit")
	game.state.event_delegate:trigger("on_stratagem_activated", "peer", {}, "pos", "unit")
	E.eq(count, 1)
end)

test("an_event_the_game_doesnt_have_is_refused", function()
	local api = start()
	local ok, why = api.events.on("on_nothing", function() end)
	E.eq(ok, false)
	E.eq(why, "unknown_event")
	E.eq(select(2, api.events.on("on_enemy_killed", "not a function")), "bad_type")
end)

test("damage_scales_by_who_takes_it_and_keeps_all_three_answers", function()
	local api, game, e = start()
	hooked(e)
	local calc = rawget(_G, "HealthComponent").calculate_damage
	E.eq(api.combat.scale({ players = 0.5, enemies = 2 }), true)
	local damage, zone, kills = calc(game.me, 10)
	E.eq(damage, 5)
	E.eq(zone, 1)
	E.eq(kills, false)
	E.eq((calc(game.enemies[1], 10)), 20)
	E.eq((calc(game.convoy, 10)), 10, "not a player, not an enemy")
	no_tampering(game)
end)

test("a_scaled_hit_says_truly_whether_it_kills", function()
	local api, game, e = start()
	hooked(e)
	local calc = rawget(_G, "HealthComponent").calculate_damage
	api.combat.scale({ enemies = 3 })
	E.eq(select(3, calc(game.enemies[1], 20)), true, "60 is past 30 hitpoints and 10 constitution")
	E.eq(select(3, calc(game.enemies[1], 5)), false)
end)

test("a_huge_scale_stops_just_past_dead", function()
	local api, game, e = start()
	hooked(e)
	api.combat.scale({ enemies = 100 })
	E.eq((rawget(_G, "HealthComponent").calculate_damage(game.enemies[1], 20000)), 20000, "the game's own answer stands")
	E.eq((rawget(_G, "HealthComponent").calculate_damage(game.enemies[1], 100)), 100 * 1 < 41 and 41 or 100)
	E.eq((rawget(_G, "HealthComponent").calculate_damage(game.enemies[1], 5)), 41, "hitpoints 30, constitution 10 below zero")
end)

test("shields_scale_too", function()
	local api, game, e = start()
	hooked(e)
	local calc = rawget(_G, "ShieldComponent").calculate_damage
	api.combat.scale({ enemies = 2 })
	local damage, broken = calc(game.enemies[1], 15)
	E.eq(damage, 30)
	E.eq(broken, true)
end)

test("a_scale_of_one_or_false_clears_a_side_and_bad_numbers_change_nothing", function()
	local api, game, e = start()
	hooked(e)
	api.combat.scale({ players = 2, enemies = 2 })
	E.eq(api.combat.scale({ players = 1 }), true)
	E.eq(api.effects.is_on("damage_to_players"), false)
	E.eq(api.effects.get("damage_to_enemies"), 2, "left out, left alone")
	E.eq(api.combat.scale({ enemies = false }), true)
	E.eq(api.effects.is_on("damage_to_enemies"), false)
	for _, bad in ipairs({ -1, 101, 0 / 0, "x" }) do
		local ok, why = api.combat.scale({ players = bad })
		E.eq(ok, false)
		E.eq(why, "out_of_range")
	end
end)

test("no_damage_at_all_is_a_scale_too", function()
	local api, game, e = start()
	hooked(e)
	api.combat.scale({ players = 0 })
	E.eq((rawget(_G, "HealthComponent").calculate_damage(game.me, 10)), 0)
end)

test("the_session_gate_takes_the_scale_off", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	hooked(e)
	api.combat.scale({ enemies = 2 })
	lobby.members = { "me", "stranger" }
	e:frames(31)
	E.eq((rawget(_G, "HealthComponent").calculate_damage(game.enemies[1], 10)), 10)
end)

-- HealthComponent's two commands that work out a hit, as far as on_hit and on_fatal use them: each
-- asks calculate_damage first, then takes the health off, the master's count through the setter.
local function fake_hits(game)
	local H = rawget(_G, "HealthComponent")
	local function take(unit, data)
		local damage = H.calculate_damage(unit, data.power)
		local s = unit.slave.health
		s.hitpoints = s.hitpoints - damage
		if s.hitpoints_real then game.components.health:set_protected(s, "hitpoints_real", s.hitpoints_real - damage) end
	end
	H.command_master = function(_, unit, _, name, data) if name == "do_damage" then take(unit, data) end end
	H.filter_slave_command = function(_, unit, _, name, data) if name == "do_damage" then take(unit, data) end end
	return H
end

test("on_hit_hears_this_machines_hits_on_any_machines_enemies", function()
	local api, game, e = start()
	local H = fake_hits(game)
	hooked(e)
	local heard = {}
	E.eq(api.combat.on_hit(function(victim, hit) heard[#heard + 1] = { victim, hit.damage, hit.slot, hit.owner_unit } end), true)
	local mine, theirs = game.enemies[1], game.enemies[2]
	H:command_master(mine, nil, "do_damage", { power = 12, owner_unit = game.me, attacker_unit = game.weapon })
	H:filter_slave_command(theirs, nil, "do_damage", { power = 7, owner_unit = game.me, attacker_unit = game.weapon })
	H:command_master(mine, nil, "do_damage", { power = 3, owner_unit = game.me, attacker_unit = game.arm })
	E.eq(#heard, 3)
	E.eq(heard[1][1] == mine and heard[1][2] == 12 and heard[1][3] == "primary_weapon" and heard[1][4] == game.me, true)
	E.eq(heard[2][1] == theirs and heard[2][2] == 7, true, "an enemy another machine runs")
	E.eq(heard[3][3], nil, "not a weapon the player holds")
	-- Another player's hit, one that costs nothing, a heal and a sum asked for outside a hit.
	H:filter_slave_command(mine, nil, "do_damage", { power = 5, owner_unit = theirs, attacker_unit = game.weapon })
	H:command_master(mine, nil, "do_damage", { power = 0, owner_unit = game.me })
	H:command_master(mine, nil, "do_heal", 10)
	H.calculate_damage(mine, 4)
	E.eq(#heard, 3)
	E.eq(mine.slave.health.hitpoints, 30 - 12 - 3 - 5, "every hit still lands")
end)

test("on_hit_stops_with_off_hit_and_a_failing_listener_hurts_nothing", function()
	local api, game, e = start()
	local H = fake_hits(game)
	hooked(e)
	local count = 0
	local function fn() count = count + 1 end
	api.combat.on_hit(function() error("broken listener") end)
	api.combat.on_hit(fn)
	H:command_master(game.enemies[1], nil, "do_damage", { power = 5, owner_unit = game.me, attacker_unit = game.weapon })
	E.eq(count, 1, "the next listener still runs")
	E.eq(game.enemies[1].slave.health.hitpoints, 25)
	E.eq(api.combat.off_hit(fn), true)
	H:command_master(game.enemies[1], nil, "do_damage", { power = 5, owner_unit = game.me, attacker_unit = game.weapon })
	E.eq(count, 1)
	E.eq(api.combat.on_hit("not a function"), false)
end)

test("on_fatal_can_spare_a_player_once_per_hit_with_one_hitpoint", function()
	local api, game, e = start()
	local H = fake_hits(game)
	hooked(e)
	local asked, spare = {}, true
	api.combat.on_fatal(function(unit, damage) asked[#asked + 1] = damage return spare end)
	local health = game.me.slave.health
	H:command_master(game.me, nil, "do_damage", { power = 40, owner_unit = game.enemies[1] })
	E.eq(#asked, 0, "not a killing hit")
	H:command_master(game.me, nil, "do_damage", { power = 500, owner_unit = game.enemies[1] })
	E.eq(asked[1], 500)
	E.eq(health.hitpoints_real, 1, "60 left, cut to 59")
	-- A volley in one frame: hitpoints lags the master's own count, which is the one asked.
	health.hitpoints = 60
	H:command_master(game.me, nil, "do_damage", { power = 30, owner_unit = game.enemies[1] })
	E.eq(asked[2], 30, "1 left by the master's count")
	E.eq(health.hitpoints_real, 1)
	spare = false
	H:command_master(game.me, nil, "do_damage", { power = 30, owner_unit = game.enemies[1] })
	E.eq(health.hitpoints_real, -29, "not spared")
	-- Only a player of this machine, and only where this machine runs them.
	H:command_master(game.enemies[1], nil, "do_damage", { power = 500, owner_unit = game.me })
	H:filter_slave_command(game.me, nil, "do_damage", { power = 500, owner_unit = game.enemies[1] })
	E.eq(#asked, 3)
end)

test("a_client_mod_hears_hits_where_session_mods_are_off_and_nobody_is_spared_there", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	local H = fake_hits(game)
	hooked(e)
	local core = e:core()
	local own = core.current
	core.current = { id = "dev.hud", state = {}, mod = { options = {}, scope = "client" } }
	local heard = {}
	E.eq(api.combat.on_hit(function(_, hit) heard[#heard + 1] = hit end), true, "open to a client mod")
	core.current = own
	local asked = 0
	api.combat.on_fatal(function() asked = asked + 1 return true end)
	lobby.members = { "me", "stranger" }
	e:frames(31)
	H:command_master(game.enemies[1], nil, "do_damage", { power = 5, owner_unit = game.me, attacker_unit = game.weapon })
	E.eq(#heard, 1, "hearing a hit changes nothing")
	H:command_master(game.me, nil, "do_damage", { power = 500, owner_unit = game.enemies[1] })
	E.eq(asked, 0, "sparing a player changes the game")
end)

test("a_downed_player_spared_stays_as_they_were", function()
	local api, game, e = start()
	local H = fake_hits(game)
	hooked(e)
	api.combat.on_fatal(function() return true end)
	local health = game.me.slave.health
	health.constitution, health.hitpoints = -50, -10
	game.components.health:set_protected(health, "hitpoints_real", -10)
	H:command_master(game.me, nil, "do_damage", { power = 100, owner_unit = game.enemies[1] })
	E.eq(health.hitpoints_real, -10)
end)

local function drive(game, vehicle)
	game.me.master.character.current_seat_unit = vehicle or game.mech
end

test("vehicle_speed_uses_the_movers_multiplier_never_the_guarded_speed", function()
	local api, game, e = start()
	drive(game)
	api.effects.set("vehicle_speed", 2)
	e:frames(1)
	E.eq(game.mech.master.mover.velocity_scale, 2)
	api.effects.set("vehicle_speed", 3)
	E.eq(game.mech.master.mover.velocity_scale, 3, "a new value at once")
	api.effects.set("vehicle_speed", false)
	E.eq(game.mech.master.mover.velocity_scale, nil)
	no_tampering(game)
end)

test("vehicle_effects_let_go_of_a_vehicle_the_player_leaves", function()
	local api, game, e = start()
	drive(game)
	api.effects.set("vehicle_speed", 2)
	api.effects.set("vehicle_god", true)
	e:frames(1)
	E.eq(G.command(game, "health", "set_invincible").data, true)
	game.commands = {}
	game.me.master.character.current_seat_unit = nil
	e:frames(1)
	E.eq(game.mech.master.mover.velocity_scale, nil)
	E.eq(G.command(game, "health", "set_invincible").data, false)
	game.commands = {}
	drive(game, game.apc)
	e:frames(1)
	E.eq(game.apc.master.mover.velocity_scale, 2, "the next vehicle gets it")
	E.eq(G.command(game, "health", "set_invincible").unit, game.apc)
end)

test("vehicle_effects_wait_until_the_vehicle_is_this_players", function()
	local api, game, e = start()
	local mover = game.mech.master.mover
	game.mech.master.vehicle = nil
	game.mech.master.mover = nil
	game.mech.master.health = nil
	drive(game)
	api.effects.set("vehicle_god", true)
	api.effects.set("vehicle_speed", 2)
	e:frames(2)
	E.eq(#game.commands, 0, "a passenger, or a vehicle still migrating")
	E.eq(mover.velocity_scale, nil)
end)

test("a_walker_turns_quicker_with_a_smaller_blend_and_wheels_with_more_degrees", function()
	local api, game, e = start()
	drive(game)
	api.effects.set("vehicle_turn", 2)
	e:frames(1)
	E.eq(game.mech.master.vehicle.rotation_speed, 0.25)
	game.mech.master.vehicle.rotation_speed = 1
	e:frames(1)
	E.eq(game.mech.master.vehicle.rotation_speed, 1, "a state that stands still stays still")
	game.mech.master.vehicle.rotation_speed = 0.5
	drive(game, game.apc)
	e:frames(1)
	E.eq(game.apc.master.vehicle.rotation_speed, 110)
	api.effects.set("vehicle_turn", false)
	E.eq(game.apc.master.vehicle.rotation_speed, 55)
end)

test("vehicle_armor_reaches_both_copies_and_the_settings_bring_it_back", function()
	local api, game, e = start()
	drive(game, game.apc)
	api.effects.set("vehicle_armor", 5000)
	e:frames(1)
	E.eq(G.command(game, "health", "armor").data.armor_max, 5000)
	E.eq(game.slave_commands[1].data.armor_min, 5000, "the damage sums read the slave")
	game.commands, game.slave_commands = {}, {}
	api.effects.set("vehicle_armor", false)
	E.eq(G.command(game, "health", "armor").data.armor_min, 20)
	E.eq(game.slave_commands[1].data.armor_max, 2000)
	no_tampering(game)
end)

test("rockets_stay_full_by_the_games_own_refill", function()
	local api, game, e = start()
	drive(game)
	api.effects.set("vehicle_rockets", true)
	e:frames(1)
	E.eq(G.command(game, "exosuit_inventory", "set_full_ammo").unit, game.mech)
	game.commands = {}
	game.in_setter = true
	game.mech.master.exosuit_inventory.num_grenades = 8
	game.in_setter = false
	e:frames(1)
	E.eq(G.command(game, "exosuit_inventory", "set_full_ammo"), nil, "already full")
	no_tampering(game)
end)

test("a_vehicle_refill_reaches_guns_that_are_units_of_their_own", function()
	local api, game = start()
	drive(game)
	E.eq(api.gear.refill_vehicle(), true)
	local reached = false
	for _, c in ipairs(game.commands) do
		if c.unit == game.arm and c.command == "set_ammo" then reached = true end
	end
	E.eq(reached, true, "the arms of the second exosuit")
end)

test("the_vehicle_under_the_player_is_known_and_driving_needs_it_to_be_theirs", function()
	local api, game = start()
	E.eq(api.vehicles.current(), nil)
	E.eq(api.vehicles.driving(), false)
	drive(game)
	E.eq(api.vehicles.current(), game.mech)
	E.eq(api.vehicles.driving(), true)
	game.mech.master.vehicle = nil
	E.eq(api.vehicles.driving(), false, "still migrating, or a passenger")
end)

test("stratagems_can_be_listed_by_subtype_and_waves_by_faction", function()
	local api = start()
	E.eq(#api.catalog.by_subtype("turret"), 2)
	E.eq(api.catalog.by_subtype("mine")[1].unit_path, "gameobjects/minedrop_01")
	E.eq(#api.catalog.by_subtype("support"), 1, "the paid pack stays out")
	E.eq(#api.catalog.waves("bugs"), 2)
	E.eq(#api.catalog.waves(nil), 0, "a client has no mission faction")
end)

test("a_failed_timed_drop_doesnt_try_again_every_frame", function()
	local api, game, e = start()
	local tries = 0
	rawget(_G, "SpatialSpawnUtlity").generate_points_in_donut = function()
		tries = tries + 1
		error("no point found")
	end
	api.mission.mutator("orbital_support")
	for _ = 1, 1300 do e:frames(1) end
	E.eq(tries, 1, "one try in 20 s")
end)

test("the_vehicle_list_leaves_out_the_mannable_turret", function()
	local api = start()
	local list = api.catalog.vehicles()
	E.eq(#list, 1)
	E.eq(list[1].name, "stratagem_exosuit")
end)

test("vehicles_and_strikes_have_their_editor_families", function()
	local api, game = start()
	local families = table.concat(api.catalog.families_of(game.mech_settings), " ")
	E.has(families, "vehicle")
	E.has(families, "toughness")
	local lsm = rawget(_G, "LuaSettingsManager")
	E.has(table.concat(api.catalog.families_of(lsm:get_settings_by_unit_path("gameobjects/airstrike_01")), " "), "airstrike")
	E.has(table.concat(api.catalog.families_of(lsm:get_settings_by_unit_path("gameobjects/minedrop_01")), " "), "mine")
	E.has(table.concat(api.catalog.families_of(game.pack), " "), "backpack")
	E.lacks(table.concat(api.catalog.families_of(lsm:get_settings_by_unit_path("gameobjects/sentry")), " "), "firearm", "one damage row")
end)

test("the_edits_of_the_new_families_reach_their_tables", function()
	local api, game = start()
	E.eq(api.edits.set_unit("gameobjects/mech", "vehicle_info.move_speed", 8, { min = 0.5, max = 30 }), true)
	E.eq(game.mech_settings.vehicle_info.move_speed, 8)
	E.eq(select(2, api.edits.set_unit("gameobjects/mech", "vehicle_info.wanted_move_speed", 8, { new = true })), "protected")
end)

test("a_backpack_comes_from_the_catalog_with_its_own_uses", function()
	local api, game = start()
	E.eq(api.gear.give_backpack("equipment/pack"), true)
	local c = G.command(game, "inventory", "set_backpack")
	E.eq(c.data.backpack_path, "equipment/pack")
	E.eq(c.data.num_uses, 60)
	E.eq(c.data.upgrade_value, 0)
	game.commands = {}
	E.eq(api.gear.give_backpack("equipment/pack", 1), true)
	E.eq(G.command(game, "inventory", "set_backpack").data.num_uses, 90, "the upgrade's own number")
end)

test("drones_ride_outside_the_backpack_slot_with_only_the_drones_seen_and_go_with_their_player", function()
	local api, game, e = start()
	E.eq(select(2, api.gear.add_drone("equipment/pack")), "not_available", "a pack with no drones")
	game.pack.drone_station_info = { docking_node = "j_attach" }
	game.pack.pickup_info.child_unit_paths = { "equipment/drone" }
	E.eq(select(2, api.gear.add_drone("equipment/rifle")), "not_available")
	E.eq(select(2, api.gear.add_drone("equipment/pack_dlc")), "not_available", "paid")
	E.eq(select(2, api.gear.add_drone("equipment/pack", nil, 99)), "out_of_range")
	local st = api.game.state()
	local spawned, gone = {}, {}
	st.entity_spawner.spawn_entity = function(_, path, _, _, _, setup)
		local parts = path == "equipment/pack" and { equipment = {}, drone_station = {} } or {}
		local u = { alive = true, owned = true, master = parts, slave = parts }
		spawned[#spawned + 1] = { path = path, setup = setup, unit = u }
		return u
	end
	st.entity_spawner.despawn_entity = function(_, u) gone[#gone + 1] = u u.alive = false end
	local units = rawget(_G, "Unit")
	units.set_unit_visibility = function(u, on) u.visible = on end

	local dog = api.gear.add_drone("equipment/pack")
	E.eq(dog:alive(), true)
	local station = spawned[1].unit
	E.eq(spawned[1].setup.wielder_unit, game.me)
	E.eq(spawned[1].setup.upgrade_value, 0)
	E.eq(spawned[2].path, "equipment/drone")
	E.eq(spawned[2].setup.station_unit, station, "the drone flies around its station")
	E.eq(G.command(game, "equipment", "wielder").data, game.me, "the station rides on the player")
	E.eq(G.command(game, "equipment", "wield").unit, station)
	E.eq(G.command(game, "inventory", "set_backpack"), nil, "the backpack slot is left alone")

	-- Every game hides a station that isn't its player's backpack.
	local em = st.entity_manager
	local has, slaves = em.has_entities, em.get_slave_entities
	em.has_entities = function(self, name) return name == "equipment" or has(self, name) end
	em.get_slave_entities = function(self, name)
		if name == "equipment" then return { [station] = { state = { wielder = game.me } } } end
		return slaves(self, name)
	end
	e:frames(1)
	E.eq(station.visible, false)
	game.me.master.inventory.backpack = station
	e:frames(1)
	E.eq(station.visible, true, "seen again as the player's own pack")
	game.me.master.inventory.backpack = nil

	-- Docked in a vehicle, out again after.
	local seat = {}
	rawset(_G, "CharacterStateSeated", seat)
	station.master.drone_station.drone_unit = spawned[2].unit
	game.me.master.character.current_state = seat
	e:frames(1)
	E.eq(G.command(game, "drone_station", "set_force_dock").data, true)
	game.commands = {}
	game.me.master.character.current_state = nil
	e:frames(1)
	E.eq(G.command(game, "drone_station", "set_force_dock").data, false)

	game.me.master.health.is_dead = true
	e:frames(1)
	E.eq(gone[1], station, "the station goes, and its drones with it")
	E.eq(#gone, 1, "the game takes the drone away itself")
	E.eq(dog:alive(), false)
	E.eq(select(2, api.gear.add_drone("equipment/pack")), "no_avatar")
	em.has_entities, em.get_slave_entities = has, slaves
end)

test("a_perk_goes_on_top_of_the_main_one_and_comes_off", function()
	local api, game = start()
	game.perk_calls = {}
	rawget(_G, "PerkSettings").perk_extra = { setup_master = function() game.perk_calls[#game.perk_calls + 1] = "extra setup" end,
		remove_master = function() game.perk_calls[#game.perk_calls + 1] = "extra remove" end }
	rawget(_G, "PerkSettings").perk_each_frame = { update_master = function() end }
	E.eq(api.gear.add_perk("perk_extra"), true)
	E.eq(game.perk_calls[1], "extra setup")
	E.eq(game.me.master.inventory.perk, 1, "the main perk stays")
	E.eq(api.gear.extra_perks()[1], "perk_extra")
	E.eq(api.gear.add_perk("perk_extra"), true)
	E.eq(#game.perk_calls, 1, "already on: its setup runs once")
	E.eq(api.gear.add_perk("perk_cardio"), true)
	E.eq(#api.gear.extra_perks(), 1, "the main perk isn't added again")
	E.eq(select(2, api.gear.add_perk("perk_each_frame")), "not_available", "the game would run its frames for no one")
	E.eq(select(2, api.gear.add_perk("perk_lasersight")), "not_available")
	E.eq(select(2, api.gear.add_perk("perk_none")), "not_available")
	E.eq(api.gear.remove_extra_perk("perk_extra"), true)
	E.eq(game.perk_calls[2], "extra remove")
	E.eq(#api.gear.extra_perks(), 0)
	E.eq(api.gear.remove_extra_perk("perk_extra"), true)
	E.eq(#game.perk_calls, 2, "already off: nothing more to undo")
end)

test("a_path_outside_the_catalog_never_reaches_set_backpack", function()
	local api, game = start()
	local ok, why = api.gear.give_backpack("equipment/rifle")
	E.eq(ok, false, "the game would read pickup_info outside any pcall")
	E.eq(why, "not_available")
	E.eq(select(2, api.gear.give_backpack("equipment/pack_dlc")), "not_available", "paid")
	E.eq(select(2, api.gear.give_backpack("equipment/pack", 99)), "out_of_range")
	E.eq(#game.commands, 0)
end)

test("the_backpack_list_has_each_pack_once_and_no_paid_ones", function()
	local api = start()
	local list = api.catalog.backpacks()
	E.eq(#list, 1, "two in one crate")
	E.eq(list[1].name, "pickup_jumppack")
	E.eq(list[1].stratagem, "stratagem_jumppack")
end)

test("a_perk_can_be_taken_away", function()
	local api, game = start()
	E.eq(api.gear.remove_perk(), true)
	E.eq(game.me.master.inventory.perk, 0)
	E.eq(game.perk_calls[1], "cardio remove_master")
	E.eq(game.me.master.inventory.dirty, true)
end)

test("the_highest_upgrade_of_a_unit_stays_in_the_games_range", function()
	local api = start()
	E.eq(api.catalog.max_upgrade("equipment/rifle"), 1, "one bit")
	E.eq(api.catalog.max_upgrade("gameobjects/mech"), 0)
	E.eq(api.catalog.max_upgrade("equipment/nothing"), 0)
end)

test("a_called_stratagem_is_a_real_flare_at_the_players_level", function()
	local api, game = start()
	game.me.master.inventory.item_upgrades.stratagem_resupply = 2
	local pos = api.world.cursor()
	E.eq(api.stratagems.call("stratagem_resupply", pos, 15), true)
	E.eq(game.spawned[1], "gameobjects/flare_01")
	E.eq(game.spawn_pos.x, pos.x)
	E.eq(game.spawn_pos.y, pos.y)
	E.eq(game.spawn_setup.has_been_thrown, true)
	E.eq(game.spawn_setup.stratagem_id, 1)
	E.eq(game.spawn_setup.extra_activation_time, 15)
	E.eq(game.spawn_setup.owner_unit, game.me)
	E.eq(game.spawn_setup.upgrade_value, 2)
	local direction = G.command(game, "stratagem", "set_thrown_direction")
	E.eq(direction.unit, game.flare)
	E.eq(type(direction.data), "table", "a vector in a command has to be boxed")
	E.eq(game.pods, nil, "the flare calls the pod, not this")
	E.eq(api.stratagems.call("stratagem_resupply", { x = 3, y = 4 }, 15), true)
	E.eq(getmetatable(game.spawn_pos), G.vector, "a plain position reaches the game as a vector")
	E.eq(game.spawn_pos.z, 0)
	no_tampering(game)
end)

test("a_stratagem_can_be_called_for_another_player_on_this_machine", function()
	local api, game = start()
	local second = { alive = true, owned = true, master = { inventory = { item_upgrades = { stratagem_resupply = 3 } } }, slave = {} }
	E.eq(api.stratagems.call("stratagem_resupply", api.world.cursor(), 0, second), true)
	E.eq(game.spawn_setup.owner_unit, second, "their call-in, so their kills")
	E.eq(game.spawn_setup.upgrade_value, 3, "at their own level")
end)

test("reinforce_works_through_a_flare", function()
	local api, game = start()
	E.eq(api.stratagems.call("stratagem_reinforce", api.world.cursor()), true)
	E.eq(game.spawn_setup.stratagem_id, 4)
end)

test("a_call_is_checked", function()
	local api, game = start()
	E.eq(select(2, api.stratagems.call("stratagem_dlc", api.world.cursor())), "not_available")
	E.eq(select(2, api.stratagems.call("stratagem_resupply", nil)), "no_position")
	for _, bad in ipairs({ -1, 601, 0 / 0, "soon" }) do
		E.eq(select(2, api.stratagems.call("stratagem_resupply", api.world.cursor(), bad)), "out_of_range")
	end
	E.eq(#game.spawned, 0)
end)

test("a_call_and_a_spawn_need_a_mission", function()
	local api, game, e, ui = start()
	local pos = api.world.cursor()
	ui.state = "menu"
	E.eq(select(2, api.stratagems.call("stratagem_resupply", pos)), "not_in_mission", "the ship has a spawner too")
	E.eq(select(2, api.enemies.spawn_entity("gameobjects/supplycrate_ammunition", pos)), "not_in_mission")
	E.eq(#game.spawned, 0)
end)

test("a_crate_and_its_pickups_spawn_by_path_and_the_lists_leave_paid_ones_out", function()
	local api, game = start()
	local crates, pickups = {}, {}
	for _, item in ipairs(api.catalog.crates()) do crates[item.unit_path] = true end
	for _, item in ipairs(api.catalog.pickups()) do pickups[item.unit_path] = true end
	E.eq(crates["gameobjects/crate_pack"], true)
	E.eq(pickups["equipment/pack_pickup"], true)
	E.eq(crates["gameobjects/crate_pack_dlc"], nil, "paid")
	E.eq(pickups["equipment/pack_dlc_pickup"], nil, "paid")
	E.eq(api.enemies.spawn_entity("gameobjects/crate_pack", api.world.cursor()), true)
	E.eq(game.spawned[1], "gameobjects/crate_pack")
end)

test("jammers_answer_no_only_while_the_effect_is_on", function()
	local api, game, e = start()
	hooked(e)
	local area = rawget(_G, "StratagemsDisabledAreaComponent")
	E.eq(area.is_unit_inside_any_area(game.me, "manager"), true)
	api.effects.set("ignore_jammers", true)
	E.eq(area.is_unit_inside_any_area(game.me, "manager"), false)
	api.effects.set("ignore_jammers", false)
	E.eq(area.is_unit_inside_any_area(game.me, "manager"), true)
end)

test("a_status_travels_on_a_hit_of_no_power_from_any_peer", function()
	local api, game = start()
	local other = game.enemies[2]
	E.eq(api.enemies.apply_status(other, "flashbang_stun"), true)
	local c = game.commands[1]
	E.eq(c.unit, other, "a unit this peer doesn't own: add_status would fail on it")
	E.eq(c.interface, "damageable")
	E.eq(c.command, "do_damage")
	E.eq(c.data.power, 0)
	E.eq(c.data.hit_status_effects[1], "flashbang_stun")
	E.eq(c.data.owner_unit, game.me)
end)

test("a_status_is_checked", function()
	local api, game = start()
	E.eq(select(2, api.enemies.apply_status(game.enemies[1], "no_such_status")), "unknown_status")
	E.eq(select(2, api.enemies.apply_status(game.me, "flashbang_stun")), "no_enemy")
	game.enemies[1].slave.health.is_dead = true
	E.eq(select(2, api.enemies.apply_status(game.enemies[1], "flashbang_stun")), "no_enemy")
	E.eq(#game.commands, 0)
end)

test("a_stun_reaches_only_the_enemies_in_range", function()
	local api, game = start()
	E.eq(api.enemies.stun_near(20), true)
	E.eq(#game.commands, 1)
	E.eq(game.commands[1].unit, game.enemies[1])
end)

test("one_enemy_can_be_read_found_and_killed", function()
	local api, game = start()
	local near, distance = api.enemies.nearest()
	E.eq(near, game.enemies[1])
	E.eq(distance > 0 and distance < 3, true)
	E.eq((api.enemies.nearest({ x = 99, y = 0, z = 0 })), game.enemies[2])
	local info = api.enemies.info(near)
	E.eq(info.key, "bug_scout")
	E.eq(info.faction, "bugs")
	E.eq(info.hitpoints, 30)
	E.eq(info.owned, true)
	E.eq(api.enemies.info(game.me), nil)
	E.eq(api.enemies.kill(near), true)
	E.eq(G.command(game, "health", "set_hitpoints").data.owner_unit, game.me, "the player gets the kill")
	E.eq(select(2, api.enemies.kill(game.me)), "no_enemy")
end)

test("pressure_sets_the_managers_and_gives_their_numbers_back", function()
	local api, game = start()
	local em, pm = game.state.encounter_manager, game.state.patrol_manager
	E.eq(api.enemies.set_pressure({ cap = 24, difficulty = 15, patrols = 20, interval = 2 }), true)
	E.eq(em.max_num_enemies, 24)
	E.eq(em.mission_difficulty, 15)
	E.eq(pm.max_num_patrols, 20)
	E.eq(pm.patrol_spawn_interval, 2)
	E.eq(api.enemies.set_pressure({ difficulty = false, patrols = false, interval = false }), true)
	E.eq(em.mission_difficulty, 5)
	E.eq(pm.max_num_patrols, 6)
	E.eq(pm.patrol_spawn_interval, 8)
	E.eq(em.max_num_enemies, 24, "left out, left alone")
end)

test("pressure_is_the_hosts_and_a_bad_number_changes_nothing", function()
	local api, game = start({ host = false })
	E.eq(select(2, api.enemies.set_pressure({ cap = 24 })), "host_only")
	local api2 = api
	rawget(_G, "NetworkGlobal").lobby.is_host = function() return true end
	E.eq(select(2, api2.enemies.set_pressure({ cap = 33 })), "out_of_range", "32 at most until a live run says more")
	E.eq(select(2, api2.enemies.set_pressure({ difficulty = 5, interval = 0 / 0 })), "out_of_range")
	E.eq(game.state.encounter_manager.mission_difficulty, 5)
	E.eq(api2.effects.is_on("enemy_difficulty"), false)
end)

test("pressure_goes_away_with_the_session_and_comes_back", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	api.enemies.set_pressure({ difficulty = 15 })
	e:frames(2)
	lobby.members = { "me", "stranger" }
	e:frames(31)
	E.eq(game.state.encounter_manager.mission_difficulty, 5)
	lobby.members = { "me" }
	e:frames(32)
	E.eq(game.state.encounter_manager.mission_difficulty, 15)
end)

test("a_named_wave_spawns_each_of_its_groups", function()
	local api, game = start()
	E.eq(table.concat(api.enemies.waves(), ","), "bug_wave_scouts,bug_wave_strange")
	E.eq(api.enemies.force_wave("bug_wave_scouts", api.world.cursor()), true)
	E.eq(table.concat(game.spawned, ","), "2,2")
	E.eq(game.group_cost, 2)
end)

test("a_wave_needs_its_units_loaded_and_the_host", function()
	local api, game = start()
	E.eq(select(2, api.enemies.force_wave("bug_wave_strange")), "not_loaded")
	E.eq(select(2, api.enemies.force_wave("bug_wave_none")), "unknown_wave")
	E.eq(#game.spawned, 0)
	local api2 = start({ host = false })
	E.eq(select(2, api2.enemies.force_wave("bug_wave_scouts")), "host_only")
end)

test("the_objective_list_has_state_and_progress", function()
	local api = start()
	local list = api.mission.objectives()
	E.eq(#list, 2)
	E.eq(list[1].name, "objective_a")
	E.eq(list[1].state, "pending")
	E.eq(list[1].done, 1)
	E.eq(list[1].total, 3)
	E.eq(list[1].position.x, 9)
	E.eq(list[2].total, nil)
end)

test("the_objective_list_is_empty_off_a_mission", function()
	local api, game, e, ui = start()
	ui.state = "menu"
	E.eq(#api.mission.objectives(), 0)
end)

test("protected_objectives_cant_die_and_go_back_to_what_the_game_wants", function()
	local api, game, e = start()
	api.effects.set("protect_objectives", true)
	local units = {}
	for _, c in ipairs(game.commands) do
		if c.command == "set_invincible" then units[c.unit] = c.data end
	end
	E.eq(units[game.convoy], true)
	E.eq(units[game.escorted], true)
	game.commands = {}
	game.convoy.master.health.invincible = false
	for _ = 1, 70 do e:frames(1) end
	E.eq(G.command(game, "health", "set_invincible").unit, game.convoy, "the game turns it back off as a convoy starts")
	game.commands = {}
	game.masters.npc_escort[game.escorted].state.in_goal = true
	api.effects.set("protect_objectives", false)
	units = {}
	for _, c in ipairs(game.commands) do units[c.unit] = c.data end
	E.eq(units[game.convoy], false, "a running convoy is mortal")
	E.eq(units[game.escorted], true, "one who got there stays safe")
end)

test("a_mutator_waits_for_the_level_to_be_built", function()
	local api, game, e = start()
	local record = game.state.players[1].player_go
	local avatar = record.avatar_unit
	record.avatar_unit = nil
	E.eq(api.mission.mutator("super_boots"), true)
	e:frames(1)
	E.eq(game.state.generated_level.provinggrounds_params, nil, "level generation reads the field")
	record.avatar_unit = avatar
	e:frames(1)
	E.eq(game.state.generated_level.provinggrounds_params.sub_modifier, "super_boots")
	api.mission.mutator(nil)
	E.eq(game.state.generated_level.provinggrounds_params, nil)
	E.eq(api.effects.is_on("mutator"), false)
end)

test("a_real_proving_ground_keeps_its_own_modifier", function()
	local api, game, e = start()
	local own = { sub_modifier = "marksman", sub_challenge = "x" }
	game.state.generated_level.provinggrounds_params = own
	api.mission.mutator("super_boots")
	e:frames(1)
	E.eq(game.state.generated_level.provinggrounds_params, own)
	E.eq(own.sub_modifier, "marksman")
	api.mission.mutator(nil)
	E.eq(game.state.generated_level.provinggrounds_params, own)
end)

test("changing_mutator_drops_the_old_one", function()
	local api, game, e = start()
	api.mission.mutator("super_boots")
	e:frames(1)
	api.mission.mutator("orbital_support")
	e:frames(1)
	E.eq(game.state.generated_level.provinggrounds_params, nil, "a timer, not a field")
end)

test("the_timed_mutators_drop_real_flares_on_their_interval", function()
	local api, game, e = start()
	api.mission.mutator("orbital_bombardments")
	for _ = 1, 900 do e:frames(1) end
	E.eq(#game.spawned, 0, "not yet at 14 s")
	for _ = 1, 40 do e:frames(1) end
	E.eq(game.spawned[1], "gameobjects/flare_01")
	E.eq(game.spawn_setup.owner_unit, nil, "nobody's, as in the proving grounds")
	E.eq(game.spawn_setup.stratagem_id, 9, "the strike")
	E.eq(game.spawn_setup.extra_activation_time, 3, "a warning")
end)

test("a_mutator_the_study_left_out_is_refused", function()
	local api = start()
	E.eq(select(2, api.mission.mutator("emp")), "unknown_mutator")
	E.eq(select(2, api.mission.mutator("marksman")), "unknown_mutator")
end)

test("a_hint_shows_the_mods_words_through_the_games_own_formatter", function()
	local api, game, e = start({ hints = false })
	e:frames(1)
	E.eq(api.ui.hint("Hold the line: 100%", 3), true)
	local hint = rawget(_G, "HintSettings")[game.hint.name]
	E.eq(rawget(_G, "LocalizationManager"):string(hint.text), "Hold the line: 100%", "with hints turned off, and a % stays as written")
	E.eq(game.hint.seconds, 3)
end)

test("a_hint_goes_through_the_loaders_limit_on_what_a_mod_has_waiting", function()
	local api, game = start()
	game.state.hint_manager.queued_hints = {}
	local add = game.state.hint_manager.add_hint
	game.state.hint_manager.add_hint = function(self, name, ...)
		self.queued_hints[#self.queued_hints + 1] = { text = name }
		return add(self, name, ...)
	end
	for i = 1, 3 do E.eq(api.ui.hint("line " .. i), true) end
	E.eq(select(2, api.ui.hint("line 4")), "queue_full")
end)

test("on_an_older_loader_a_hint_uses_the_games_formatter", function()
	local api, game, e = start({ hints = false })
	e:core().capabilities.notify = nil
	E.eq(api.ui.hint("Hold the line: 100%", 3), true)
	E.eq(game.hint.text, "Hold the line: 100%", "and a % doesn't break the swap")
end)

test("a_menu_opens_by_name_for_the_main_player", function()
	local api, game = start()
	local shown = {}
	game.state.screens = { armory = {}, loadout = {} }
	game.state.active_screens = {}
	function game.state.show_menu(_, name, params) shown = { name = name, params = params } game.state.active_screens[1] = name end
	local popped = 0
	function game.state.pop_menu() popped = popped + 1 end
	E.eq(api.ui.open("armory"), true)
	E.eq(shown.name, "armory")
	E.eq(shown.params.user_pad_number ~= nil, true, "a menu reads who opened it")
	E.eq(select(2, api.ui.open("nothing")), "unknown")
	E.eq(select(2, api.ui.open("loadout", { seed = "x" })), "bad_type")
	E.eq(api.ui.open("loadout", { seed = 51000003 }), true)
	E.eq(game.state.mission_seed, 51000003, "the briefing and Loadout read it from the state")
	E.eq(table.concat(api.ui.menus(), " "), "armory loadout")
	E.eq(api.ui.close(), true)
	E.eq(popped, 1)
	game.state.active_screens = {}
	E.eq(select(2, api.ui.close()), "out_of_range")
	game.state.screens, game.state.show_menu, game.state.pop_menu, game.state.active_screens = nil, nil, nil, nil
	E.eq(select(2, api.ui.open("armory")), "not_on_ship")
end)

test("a_hint_name_shows_the_game_text", function()
	local api, game = start()
	E.eq(api.ui.hint("hint_alarm"), true)
	E.eq(game.hint.text, "text of hint_alarm")
	E.eq(game.hint.seconds, 7)
end)

test("a_hint_is_checked", function()
	local api, game, e, ui = start()
	E.eq(select(2, api.ui.hint("")), "bad_type")
	E.eq(select(2, api.ui.hint(5)), "bad_type")
	E.eq(select(2, api.ui.hint("words", 0)), "out_of_range")
	E.eq(select(2, api.ui.hint("words", "long")), "out_of_range", "the game compares it with a number in its update")
	game.state.hint_manager = nil
	E.eq(select(2, api.ui.hint("words")), "not_in_mission")
end)

test("a_ping_lands_in_the_maps_units_and_can_be_cleared", function()
	local api, game = start()
	local center = rawget(_G, "Vector3")(96, 96, 0)
	E.eq(api.ui.ping(center), true)
	local ping = game.record.player_go.map_ping
	E.eq(math.abs(ping[1]) < 0.001 and math.abs(ping[2]) < 0.001, true, "the middle of the level is the middle of the map")
	api.ui.ping(rawget(_G, "Vector3")(196, 96, 0))
	ping = game.record.player_go.map_ping
	local expected = 100 * math.cos(math.rad(45)) * 350 / (10 * 48) * 2
	E.eq(math.abs(ping[1] - expected) < 0.001, true, "turned 45 degrees and scaled to the map")
	api.ui.ping(nil)
	E.eq(game.record.player_go.map_ping[1], -1000)
end)

test("a_ping_stays_in_the_fields_range", function()
	local api, game = start()
	api.ui.ping(rawget(_G, "Vector3")(1000000, 96, 0))
	E.eq(game.record.player_go.map_ping[1] < 1000, true)
end)

test("a_ping_needs_a_mission", function()
	local api, game, e, ui = start()
	ui.state = "menu"
	E.eq(select(2, api.ui.ping(rawget(_G, "Vector3")(1, 1, 0))), "not_in_mission")
end)

test("the_player_list_reads_names_ping_and_host", function()
	local api, game = start()
	local other = { avatar_unit = game.enemies[2], name = "Friend" }
	function other:owned() return false end
	function other:owner() return "friend" end
	function other:exists() return true end
	function other:get(k) return self[k] end
	function other:has(k) return self[k] ~= nil end
	game.state.players[2] = { player_go = other, pad_number = 1 }
	rawget(_G, "NetworkGlobal").lobby.members = function() return { "me", "friend", "lost" } end
	local list = api.lobby.players()
	E.eq(#list, 3)
	E.eq(list[1].me, true)
	E.eq(list[1].host, true)
	E.eq(list[1].name, "Tester")
	E.eq(list[2].name, "Friend")
	E.eq(list[2].ping, 42)
	E.eq(list[2].host, false)
	E.eq(list[3].name, nil, "no mission record yet")
	E.eq(list[3].ping, nil, "the engine has no answer")
end)

test("the_player_list_works_with_no_lobby", function()
	local api = start()
	rawget(_G, "NetworkGlobal").lobby = nil
	local list = api.lobby.players()
	E.eq(#list, 1)
	E.eq(list[1].host, true)
end)

test("every_new_effect_has_a_way_off", function()
	local api = start()
	for _, id in ipairs({ "damage_to_players", "damage_to_enemies", "vehicle_speed", "vehicle_turn", "vehicle_armor", "vehicle_god", "vehicle_rockets",
		"ignore_jammers", "enemy_cap", "enemy_difficulty", "patrol_count", "patrol_interval", "protect_objectives", "mutator", "time_of_day" }) do
		E.eq(type(api.effects.defs[id].remove), "function", id)
	end
end)

test("nothing_new_writes_a_protected_key", function()
	local api, game, e = start()
	hooked(e)
	drive(game)
	for _, id in ipairs({ "vehicle_speed", "vehicle_turn", "vehicle_armor", "vehicle_god", "vehicle_rockets", "enemy_cap", "protect_objectives" }) do
		api.effects.set(id, 2)
	end
	for _ = 1, 70 do e:frames(1) end
	api.gear.give_perk("perk_lasersight")
	api.stratagems.call("stratagem_resupply", api.world.cursor())
	api.enemies.stun_near(200)
	api.effects.clear()
	e:frames(1)
	no_tampering(game)
end)

local function commands_of(game, name)
	local out = {}
	for _, c in ipairs(game.commands) do
		if c.command == name then out[#out + 1] = c end
	end
	return out
end

test("armor_reaches_both_copies_and_what_was_there_comes_back", function()
	local api, game = start()
	api.effects.set("armor", 200)
	E.eq(G.command(game, "health", "armor").data.armor_max, 200)
	E.eq(game.slave_commands[1].data.armor_min, 200, "the damage sums read the slave")
	game.commands, game.slave_commands = {}, {}
	api.effects.set("armor", false)
	E.eq(G.command(game, "health", "armor").data.armor_min, 15)
	E.eq(game.slave_commands[1].data.armor_max, 35)
	no_tampering(game)
end)

test("a_value_on_a_field_gives_back_what_a_perk_set", function()
	local api, game = start()
	game.me.master.character.throw_range_multiplier = 1.5
	api.effects.set("throw_range", 3)
	E.eq(G.command(game, "character", "set_throw_range_multiplier").data, 3)
	game.commands = {}
	api.effects.set("throw_range", false)
	E.eq(G.command(game, "character", "set_throw_range_multiplier").data, 1.5)
	game.commands = {}
	api.effects.set("sprint_speed", 12)
	api.effects.set("acceleration", 60)
	api.effects.set("deceleration", 60)
	E.eq(G.command(game, "character", "move_speed_sprint_max").data, 12)
	E.eq(G.command(game, "mover", "acceleration").data, 60)
	game.commands = {}
	api.effects.set("acceleration", false)
	E.eq(G.command(game, "mover", "acceleration").data, 20)
end)

test("the_switches_go_both_ways", function()
	local api, game = start()
	api.effects.set("ignore_slow", true)
	api.effects.set("no_screen_leash", true)
	E.eq(G.command(game, "mover", "ignore_slow").data, true)
	E.eq(G.command(game, "mover", "keep_inside_screen").data, false)
	game.commands = {}
	api.effects.set("ignore_slow", false)
	api.effects.set("no_screen_leash", false)
	E.eq(G.command(game, "mover", "ignore_slow").data, false)
	E.eq(G.command(game, "mover", "keep_inside_screen").data, true)
end)

test("a_hidden_model_stays_hidden_after_a_seat_shows_it", function()
	local api, game, e = start()
	api.effects.set("hide_model", true)
	E.eq(G.command(game, "customization", "set_visible").data, false)
	game.me.master.customization.visible = false
	e:frames(1)
	game.commands = {}
	e:frames(1)
	E.eq(G.command(game, "customization", "set_visible"), nil, "already hidden")
	game.me.master.customization.visible = true
	e:frames(1)
	E.eq(G.command(game, "customization", "set_visible").data, false)
	game.commands = {}
	api.effects.set("hide_model", false)
	E.eq(G.command(game, "customization", "set_visible").data, true)
end)

test("the_heavy_armor_look_waits_for_a_mission_where_its_model_is_loaded", function()
	local api, game, e = start()
	-- On the ship: the model isn't loaded, and spawning it takes the game down.
	E.eq(api.effects.set("heavy_armor_look", true), true)
	E.eq(G.command(game, "customization", "enable_heavy_armor"), nil)
	e:frames(1)
	E.eq(G.command(game, "customization", "enable_heavy_armor"), nil)
	-- The mission's body, with mission_common loaded.
	game.packages["resource_packages/mission_common"] = true
	game.state.players[1].player_go.avatar_unit = game.enemies[1]
	e:frames(1)
	game.state.players[1].player_go.avatar_unit = game.me
	e:frames(1)
	E.eq(G.command(game, "customization", "enable_heavy_armor") ~= nil, true)
end)

test("the_heavy_armor_look_leaves_the_perks_own_alone", function()
	local api, game = start()
	game.packages["resource_packages/mission_common"] = true
	api.effects.set("heavy_armor_look", true)
	E.eq(G.command(game, "customization", "enable_heavy_armor") ~= nil, true)
	game.commands = {}
	api.effects.set("heavy_armor_look", false)
	E.eq(G.command(game, "customization", "disable_heavy_armor") ~= nil, true)
	game.commands = {}
	game.me.master.customization.heavy_armor = 2
	api.effects.set("heavy_armor_look", true)
	api.effects.set("heavy_armor_look", false)
	E.eq(G.command(game, "customization", "disable_heavy_armor"), nil, "the perk gave it")
end)

test("the_outline_is_boxed_and_goes_back_to_the_players_color", function()
	local api, game = start()
	game.record.player_go.index = 1
	E.eq(api.player.set_outline(1, 2, 3), true)
	local c = G.command(game, "customization", "set_silhouette_color").data
	E.eq(c.y, 1, "boxed as the game's colors are")
	E.eq(c.w, 3)
	game.commands = {}
	E.eq(api.player.set_outline(nil), true)
	E.eq(G.command(game, "customization", "set_silhouette_color").data.y, 10)
	E.eq(select(2, api.player.set_outline(300, 0, 0)), "out_of_range")
	E.eq(select(2, api.player.set_outline(1, 2)), "out_of_range")
end)

test("a_status_comes_off_by_template_or_element_and_a_wrong_name_never_reaches_the_game", function()
	local api, game = start()
	E.eq(api.player.clear_status("heavy_burn"), true)
	E.eq(G.command(game, "status", "remove_status").data, "heavy_burn")
	E.eq(api.player.clear_status("fire"), true)
	E.eq(G.command(game, "status", "remove_statuses_by_element").data, "fire")
	game.commands = {}
	E.eq(select(2, api.player.clear_status("nothing")), "unknown_status", "the game indexes its table with it, outside any pcall")
	E.eq(#game.commands, 0)
end)

test("an_immunity_has_a_way_off_the_game_lacks", function()
	local api, game = start()
	E.eq(api.player.immune("fire"), true)
	E.eq(G.command(game, "status", "add_immunity_element").data, "fire")
	game.me.master.status.immunities_elements.fire = true
	E.eq(api.player.immune("fire", false), true)
	E.eq(game.me.master.status.immunities_elements.fire, nil)
	E.eq(api.player.immune("flashbang_stun"), true)
	E.eq(G.command(game, "status", "add_immunity_status").data, "flashbang_stun")
	E.eq(select(2, api.player.immune("nothing")), "unknown_status")
end)

test("the_small_player_actions_are_checked", function()
	local api, game = start()
	E.eq(api.player.knockdown(), true)
	E.eq(api.player.drop_carried(), true)
	E.eq(G.command(game, "character", "do_knockdown") ~= nil, true)
	E.eq(G.command(game, "character", "drop_carry") ~= nil, true)
	E.eq(api.player.heal_by(25), true)
	E.eq(G.command(game, "health", "do_heal").data, 25)
	E.eq(api.player.set_health(50), true)
	E.eq(G.command(game, "health", "set_hitpoints").data, 50)
	for _, bad in ipairs({ 0, -5, "x", 0 / 0 }) do
		E.eq(select(2, api.player.heal_by(bad)), "out_of_range")
		E.eq(select(2, api.player.set_health(bad)), "out_of_range")
	end
	no_tampering(game)
end)

test("near_lists_enemies_by_distance", function()
	local api, game = start()
	local list = api.enemies.near(500)
	E.eq(list[1], game.enemies[1])
	E.eq(list[2], game.enemies[2])
	E.eq(#api.enemies.near(20), 1)
	E.eq(api.enemies.near(5, { x = 100, y = 0, z = 0 })[1], game.enemies[2])
end)

test("a_hit_from_a_mod_goes_as_a_weapons_does", function()
	local api, game = start()
	local other = game.enemies[2]
	E.eq(api.enemies.damage(other, 500, { "flashbang_stun" }), true)
	local c = game.commands[1]
	E.eq(c.unit, other, "any enemy, from any peer")
	E.eq(c.interface, "damageable")
	E.eq(c.data.power, 500)
	E.eq(c.data.owner_unit, game.me)
	E.eq(c.data.hit_status_effects[1], "flashbang_stun")
	game.commands = {}
	E.eq(api.enemies.damage(other, 10), true)
	E.eq(game.commands[1].data.hit_status_effects, nil)
	game.commands = {}
	E.eq(select(2, api.enemies.damage(other, -1)), "out_of_range")
	E.eq(select(2, api.enemies.damage(other, 1, { "nope" })), "unknown_status")
	E.eq(select(2, api.enemies.damage(game.me, 1)), "no_enemy")
	E.eq(#game.commands, 0)
end)

test("hitpoints_of_one_enemy_go_through_its_owner", function()
	local api, game = start()
	E.eq(api.enemies.set_hitpoints(game.enemies[1], 5), true)
	E.eq(G.command(game, "health", "set_hitpoints").data, 5)
	E.eq(api.enemies.set_hitpoints(game.enemies[2], 7), true)
	local rpc = game.rpc[1]
	E.eq(rpc[2], "rpc_set_hitpoints_request")
	E.eq(rpc[6], 7)
	E.eq(select(2, api.enemies.set_hitpoints(game.enemies[1], 0)), "out_of_range")
	no_tampering(game)
end)

test("one_enemy_can_be_despawned", function()
	local api, game = start()
	E.eq(api.enemies.despawn(game.enemies[1]), true)
	E.eq(G.command(game, "enemy", "remove_from_group") ~= nil, true)
	E.eq(api.enemies.despawn(game.enemies[2]), true)
	E.eq(game.rpc[1][2], "rpc_despawn_enemy_unit")
	E.eq(select(2, api.enemies.despawn(game.me)), "no_enemy")
end)

test("a_faction_loads_for_a_solo_player_and_goes_at_the_end_of_the_mission", function()
	local api, game, e, ui = start()
	E.eq(api.enemies.load_faction("cyborgs"), true)
	E.eq(game.packages["resource_packages/race_cyborgs"], true)
	E.eq(game.flushed, true, "ready at once, for a spawn on the next line")
	e:frames(5)
	E.eq(game.packages["resource_packages/race_cyborgs"], true, "kept through the mission")
	ui.state = "mission_end"
	e:frames(1)
	E.eq(game.packages["resource_packages/race_cyborgs"], nil, "the game wouldn't release it")
	E.eq(game.packages["resource_packages/race_bugs"], true, "and its own is its own")
end)

test("loading_a_faction_the_game_already_has_takes_nothing_away_later", function()
	local api, game, e, ui = start()
	E.eq(api.enemies.load_faction("bugs"), true)
	ui.state = "mission_end"
	e:frames(1)
	E.eq(game.packages["resource_packages/race_bugs"], true)
end)

test("a_faction_needs_a_lone_player_a_mission_and_a_real_name", function()
	local lobby = { my_id = "me", members = { "me", "friend" }, data = {} }
	local api, game = start({ lobby = lobby })
	E.eq(select(2, api.enemies.load_faction("cyborgs")), "alone_only", "every peer would need the package")
	lobby.members = { "me" }
	E.eq(select(2, api.enemies.load_faction("robots")), "unknown_faction")
	E.eq(game.packages["resource_packages/race_cyborgs"], nil)
	local api2, game2, e2, ui2 = start()
	ui2.state = "menu"
	E.eq(select(2, api2.enemies.load_faction("cyborgs")), "not_in_mission")
end)

test("the_field_of_view_is_checked_and_comes_back", function()
	local api = start()
	local cam = rawget(_G, "CameraManager")
	E.eq(api.world.camera.fov(90), true)
	E.eq(math.abs(cam.fov - math.rad(90)) < 0.001, true)
	E.eq(api.world.camera.fov(nil), true)
	E.eq(math.abs(cam.fov - math.rad(60)) < 0.001, true)
	for _, bad in ipairs({ 5, 121, 0 / 0, "wide" }) do E.eq(select(2, api.world.camera.fov(bad)), "out_of_range") end
end)

test("the_spin_goes_back_to_what_a_mission_starts_with", function()
	local api, game = start()
	api.world.camera.spin(30)
	E.eq(game.camera[4], 30)
	api.world.camera.spin(nil)
	E.eq(game.camera[4], -45)
end)

test("the_spin_leaves_the_ship_alone", function()
	local api, game, e, ui = start()
	ui.state = "menu"
	game.camera = nil
	api.world.camera.spin(30)
	E.eq(game.camera, nil)
end)

local function teammate(game)
	local go = { avatar_unit = game.enemies[2], name = "Friend" }
	function go:owned() return false end
	function go:owner() return "friend" end
	function go:exists() return true end
	function go:get(k) return self[k] end
	function go:has(k) return self[k] ~= nil end
	game.state.players[2] = { player_go = go, pad_number = 1 }
	return game.enemies[2]
end

test("spectating_swaps_the_cameras_target_and_holds_past_a_respawn", function()
	local api, game, e = start()
	local cam = rawget(_G, "CameraManager")
	cam.targets[game.me] = {}
	local friend = teammate(game)
	E.eq(api.world.camera.spectate(friend), true)
	E.eq(cam.targets[friend] ~= nil, true)
	E.eq(cam.targets[game.me], nil)
	cam.targets[game.me] = {}
	e:frames(1)
	E.eq(cam.targets[game.me], nil, "the game adds a new avatar back")
	E.eq(api.world.camera.spectate(nil), true)
	E.eq(cam.targets[friend], nil)
	E.eq(cam.targets[game.me] ~= nil, true)
end)

test("only_a_players_avatar_can_be_spectated", function()
	local api, game = start()
	E.eq(select(2, api.world.camera.spectate(game.enemies[1])), "no_avatar")
end)

test("a_spectated_player_who_is_gone_gives_the_camera_back", function()
	local api, game, e = start()
	local cam = rawget(_G, "CameraManager")
	local friend = teammate(game)
	api.world.camera.spectate(friend)
	friend.alive = false
	cam.targets = {}
	e:frames(1)
	E.eq(cam.targets[game.me] ~= nil, true)
end)

test("photo_mode_freezes_the_camera_and_clears_the_screen", function()
	local api, game = start()
	local cam = rawget(_G, "CameraManager")
	E.eq(api.world.camera.photo(true), true)
	E.eq(cam.state, "fully_static")
	E.eq(api.effects.is_on("hide_hud"), true)
	E.eq(game.state.hint_manager.active, false)
	E.eq(game.state.mission_manager.display_objectives, false)
	E.eq(api.world.camera.place("here", "that way"), true)
	E.eq(cam.at, "here")
	E.eq(api.world.camera.photo(false), true)
	E.eq(cam.state, "follow")
	E.eq(api.effects.is_on("hide_hud"), false)
	E.eq(game.state.hint_manager.active, true)
	E.eq(game.state.mission_manager.display_objectives, true)
	E.eq(select(2, api.world.camera.place("here")), "not_photo_mode")
end)

test("photo_mode_leaves_a_hidden_hud_hidden", function()
	local api = start()
	api.effects.set("hide_hud", true)
	api.world.camera.photo(true)
	api.world.camera.photo(false)
	E.eq(api.effects.is_on("hide_hud"), true)
end)

test("photo_mode_needs_a_mission", function()
	local api, game, e, ui = start()
	ui.state = "menu"
	E.eq(select(2, api.world.camera.photo(true)), "not_in_mission")
end)

test("a_shading_value_goes_on_by_name_and_kind_and_comes_off", function()
	local api, game = start()
	E.eq(api.world.shading("fog_color", "vector3", { 0.5, 0.4, 0.3 }), true)
	E.eq(game.shadings.fog_color.kind, "vector3")
	E.eq(game.shadings.fog_color.value[3], 0.3)
	E.eq(api.world.shading("exposure", "scalar", 2), true)
	E.eq(game.shadings.exposure.value, 2)
	E.eq(api.world.shading("fog_color", "vector3"), true)
	E.eq(game.shadings.fog_color, nil)
end)

test("a_shading_value_comes_back_in_a_new_mission", function()
	local api, game, e = start()
	api.world.shading("sun_color", "vector3", { 8, 1, 1 })
	game.shadings = nil
	game.state.players[1].player_go.avatar_unit = game.enemies[1]
	e:frames(1)
	E.eq(game.shadings.sun_color.value[1], 8)
end)

test("the_engine_gets_no_shading_name_or_value_it_wouldnt_take", function()
	local api, game = start()
	E.eq(select(2, api.world.shading("made_up", "scalar", 1)), "unknown_shading")
	E.eq(select(2, api.world.shading("fog_color", "scalar", 1)), "bad_type", "the viewport would hand it over as the wrong type")
	E.eq(select(2, api.world.shading("fog_color", "vector3", { 1, 2 })), "bad_value", "a list of 2 fails an assert in the frame")
	E.eq(select(2, api.world.shading("fog_color", "vector3", { 1, 2, 0 / 0 })), "bad_value")
	E.eq(select(2, api.world.shading("exposure", "scalar", 1 / 0)), "bad_value")
	E.eq(game.shadings, nil)
end)

test("lightning_needs_a_forest_that_has_it", function()
	local api, game = start()
	E.eq(select(2, api.world.flash()), "wrong_environment", "a desert")
	game.state.generated_level.environment_name = "forest"
	E.eq(api.world.flash(), true)
	E.eq(game.flash.shading_environment, "lightning")
	E.eq(game.flash.target_value, 1)
	E.eq(type(game.flash.fade_out_time), "number", "the game ignores an event without its three times")
	game.state.generated_level.enemy = "illuminates"
	E.eq(select(2, api.world.flash()), "wrong_environment", "their forest has no such setting")
	game.state.generated_level.enemy = "bugs"
	E.eq(select(2, api.world.flash(0)), "out_of_range")
end)

test("a_bus_volume_holds_against_the_mix_states_and_comes_back", function()
	local api, game, e = start()
	E.eq(api.audio.bus("combat_enemy", -30), true)
	e:frames(1)
	E.eq(game.buses.combat_enemy, -30)
	game.buses.combat_enemy = -4
	e:frames(1)
	E.eq(game.buses.combat_enemy, -30, "a state blend wrote it")
	E.eq(api.audio.bus("combat_enemy", false), true)
	E.eq(game.buses.combat_enemy, -4, "what the mix wants")
end)

test("a_master_bus_comes_back_through_the_options", function()
	local api, game, e = start()
	api.audio.bus("sfx_master", -60)
	e:frames(1)
	E.eq(api.audio.bus("sfx_master", nil), true)
	E.eq(game.volumes_set, 1)
end)

test("zero_is_a_volume_not_off", function()
	local api, game, e = start()
	api.audio.bus("vo", 0)
	e:frames(1)
	E.eq(game.buses.vo, 0)
	E.eq(api.effects.is_on("bus_vo"), true)
end)

test("a_bus_is_checked", function()
	local api, game = start()
	E.eq(select(2, api.audio.bus("nothing", 0)), "unknown_bus")
	E.eq(select(2, api.audio.bus("master", 0)), "unknown_bus", "it has no value to give back")
	E.eq(select(2, api.audio.bus("vo", 50)), "out_of_range")
	E.eq(select(2, api.audio.bus("vo", 0 / 0)), "out_of_range")
end)

test("a_sound_plays_only_if_the_game_has_it", function()
	local api, game = start()
	E.eq(api.audio.play("ui_select"), true)
	E.eq(game.played[1].name, "ui_select")
	E.eq(game.played[1].pos, nil)
	E.eq(api.audio.play("ui_select", "there"), true)
	E.eq(game.played[2].pos, "there")
	E.eq(select(2, api.audio.play("made_up")), "unknown_event")
	E.eq(select(2, api.audio.play(5)), "unknown_event")
	E.eq(#game.played, 2)
end)

test("a_sting_goes_through_the_games_cooldown", function()
	local api, game = start()
	E.eq(api.audio.stinger("music_objective_found"), true)
	E.eq(game.stinger, "music_objective_found")
	E.eq(select(2, api.audio.stinger("made_up")), "unknown_event")
	game.state.audio_state_machine = nil
	E.eq(select(2, api.audio.stinger("music_objective_found")), "not_in_mission")
end)

test("the_session_gate_takes_the_view_and_the_sound_off_and_gives_them_back", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	api.world.camera.fov(100)
	api.audio.bus("combat_enemy", -30)
	api.world.shading("exposure", "scalar", 2)
	e:frames(31)
	lobby.members = { "me", "stranger" }
	e:frames(31)
	local cam = rawget(_G, "CameraManager")
	E.eq(math.abs(cam.fov - math.rad(60)) < 0.001, true)
	E.eq(game.buses.combat_enemy, -4)
	E.eq(game.shadings.exposure, nil)
	lobby.members = { "me" }
	e:frames(33)
	E.eq(math.abs(cam.fov - math.rad(100)) < 0.001, true)
	E.eq(game.shadings.exposure.value, 2)
end)

local function on_ship(ui, game)
	ui.state = "menu"
	game.state.active_screens = {}
end

test("a_loadout_goes_in_the_save_and_the_first_preset", function()
	local api, game, e, ui = start()
	on_ship(ui, game)
	E.eq(api.progress.set_loadout({ primary_weapon = "weapon_other", stratagems = { [1] = "s1", [4] = "s4" } }), true)
	local l = game.save.user.loadout
	E.eq(l.primary_weapon, "weapon_other")
	E.eq(l.perk, "perk_cardio", "left out, left alone")
	E.eq(l.stratagems[5], "s1", "after the four fixed slots")
	E.eq(l.stratagems[8], "s4")
	E.eq(l.stratagems[1], "a")
	E.eq(game.save.user.loadouts[1].primary_weapon, "weapon_other", "the Loadout screen starts from it")
	E.eq(game.save.user.loadouts[2].perk, "other")
	E.eq(game.saved, 1, "written out")
	E.eq(api.progress.loadout().primary_weapon, "weapon_other")
end)

test("a_paid_item_doesnt_get_into_a_loadout", function()
	local api, game, e, ui = start()
	on_ship(ui, game)
	E.eq(api.progress.set_loadout({ primary_weapon = "weapon_dlc" }), true)
	E.eq(game.save.user.loadout.primary_weapon, "weapon_rifle")
end)

test("a_loadout_is_changed_only_with_the_screens_closed", function()
	local api, game, e, ui = start()
	E.eq(select(2, api.progress.set_loadout({})), "not_on_ship")
	on_ship(ui, game)
	game.state.active_screens = { {} }
	E.eq(select(2, api.progress.set_loadout({})), "close_screen", "it would write its own copy back")
	game.state.active_screens = {}
	E.eq(select(2, api.progress.set_loadout("rifle")), "bad_type")
	E.eq(select(2, api.progress.set_loadout({ stratagems = { [5] = "s" } })), "out_of_range")
	E.eq(select(2, api.progress.set_loadout({ stratagems = { x = "s" } })), "out_of_range")
	E.eq(game.save.user.loadout.stratagems[5], "e", "nothing written")
end)

test("a_copy_of_the_loadout_doesnt_reach_the_save", function()
	local api, game = start()
	api.progress.loadout().perk = "changed"
	api.progress.upgrades().a = 9
	E.eq(game.save.user.loadout.perk, "perk_cardio")
	E.eq(game.save.user.item_upgrades.a, 1)
end)

test("one_item_can_be_upgraded_in_the_games_range", function()
	local api, game, e, ui = start()
	on_ship(ui, game)
	E.eq(api.progress.upgrade_limit("weapon_rifle"), 3, "two bits")
	E.eq(api.progress.upgrade_limit("stratagem_resupply"), 3, "three levels")
	E.eq(api.progress.upgrade_limit("stratagem_machinegun"), nil)
	E.eq(api.progress.set_upgrade("weapon_rifle", 3), true)
	E.eq(game.save.user.item_upgrades.weapon_rifle, 3)
	E.eq(game.save.user.item_upgrades.a, 1, "merged")
	E.eq(api.progress.upgrades().weapon_rifle, 3)
	E.eq(select(2, api.progress.set_upgrade("weapon_rifle", 4)), "out_of_range")
	E.eq(select(2, api.progress.set_upgrade("weapon_rifle", 0.5)), "out_of_range")
	E.eq(select(2, api.progress.set_upgrade("weapon_dlc", 1)), "not_available", "paid")
	E.eq(select(2, api.progress.set_upgrade("no_item", 1)), "not_available")
	game.state.active_screens = { {} }
	E.eq(select(2, api.progress.set_upgrade("weapon_rifle", 1)), "close_screen")
end)

test("every_new_event_reaches_a_listener", function()
	local api, game, e = start()
	local heard = {}
	for _, name in ipairs(api.events.NAMES) do
		E.eq(api.events.on(name, function(...) heard[name] = select("#", ...) end), true)
	end
	e:frames(1)
	for _, name in ipairs({ "on_killed", "on_dead", "on_health_downed", "on_damage_taken", "on_health_received", "on_stratagem_pre_activated",
		"on_stratagem_used", "on_stratagem_wielded", "on_extract_shuttle_landed", "on_add_to_stat", "on_pause", "on_mission_end" }) do
		game.state.event_delegate:trigger(name, game.me, 2)
		E.eq(heard[name], 2, name)
	end
end)

test("the_game_cheat_detector_stays_out_of_reach", function()
	local api = start()
	E.eq(select(2, api.events.on("on_fusk_detected", function() end)), "unknown_event")
end)

test("every_effect_of_this_release_has_a_way_off", function()
	local api = start()
	api.audio.bus("vo", 0)
	api.world.shading("exposure", "scalar", 1)
	api.player.immune("fire")
	for _, id in ipairs({ "armor", "sprint_speed", "throw_range", "acceleration", "deceleration", "ignore_slow", "no_screen_leash", "hide_model",
		"heavy_armor_look", "outline_color", "camera_fov", "camera_spin", "camera_spectate", "photo_mode", "bus_vo", "shading_exposure", "immune_fire" }) do
		E.eq(type(api.effects.defs[id].remove), "function", id)
	end
end)

test("nothing_of_this_release_writes_a_protected_key", function()
	local api, game, e = start()
	api.effects.set("armor", 100)
	api.effects.set("sprint_speed", 10)
	api.effects.set("throw_range", 2)
	api.effects.set("acceleration", 50)
	api.player.heal_by(5)
	api.player.set_health(50)
	api.enemies.set_hitpoints(game.enemies[1], 5)
	api.enemies.damage(game.enemies[1], 5)
	for _ = 1, 5 do e:frames(1) end
	api.effects.clear()
	no_tampering(game)
end)

test("a_component_this_state_never_registered_reads_as_empty", function()
	local api, game = start({ state = "menu" })
	local function refused() error("attempt to index a nil value") end
	game.state.entity_manager = { has_entities = function() return false end, get_master_entities = refused, get_slave_entities = refused }
	game.state.mission_manager = nil
	E.eq(next(api.game.masters("objective_extract_lz")), nil)
	E.eq(next(api.game.slaves("objective_extract_lz")), nil)
	E.eq(api.enemies.count(), 0, "the count on the Enemies page, on the ship")
	E.eq(api.player.teleport_extraction(), false, "says why, with nothing to go to")
end)

test("the_safe_limit_counts_corpses_and_the_queue", function()
	local api, game = start()
	local base = api.enemies.carried()
	E.eq(base, api.enemies.count(), "only living enemies so far")
	game.enemies[1].slave.health.is_dead = true
	E.eq(api.enemies.carried(), base, "a corpse is still on the map")
	E.eq(api.enemies.count(), base - 1)
	game.state.encounter_manager.num_queued_enemies = 5
	game.state.patrol_manager.num_queued_enemies = 3
	E.eq(api.enemies.carried(), base + 8)
	E.eq(api.enemies.room(), api.enemies.MAX_SAFE - base - 8)
	game.state.encounter_manager.num_queued_enemies = 1000
	E.eq(api.enemies.room(), 0, "never below nothing")
end)

test("the_limit_is_for_the_trainer_and_spawn_keeps_its_power", function()
	local api, game = start()
	game.state.encounter_manager.num_queued_enemies = 1000
	E.eq(api.enemies.spawn("bug_scout", 5), true)
end)

test("one_table_edit_can_be_told_apart_and_undone", function()
	local api = start()
	local t = { a = 1, b = 2 }
	E.eq(api.edits.table_changed(t, "a"), false)
	api.edits.set_table("thing", t, "a", 5)
	api.edits.set_table("thing", t, "b", 6)
	E.eq(api.edits.table_changed(t, "a"), true)
	E.eq(api.edits.revert_table(t, "a"), true)
	E.eq(t.a, 1)
	E.eq(t.b, 6, "the other edit stays")
	E.eq(api.edits.table_changed(t, "a"), false)
	E.eq(api.edits.revert_table(t, "a"), false, "nothing left to undo")
end)

test("what_frozen_enemies_are_sent_keeps_its_values_after_the_game_reuses_the_table", function()
	local api, game = start()
	local entity_aux = rawget(_G, "EntityAux")
	local real = entity_aux.queue_command_master
	api.effects.set("freeze", true)
	local unit = { owned = true, master = {}, slave = {} }
	local temp = { damage = 12, owner_unit = "me" }
	local before = #game.commands
	entity_aux.queue_command_master(unit, "enemy", "damage_taken", temp)
	local queued = game.commands[before + 1]
	temp.damage = nil -- the game's TempTableFactory takes it back at the frame's end
	E.eq(queued.data.damage, 12, "the frozen enemy reads its own copy")
	api.effects.set("freeze", false)
	E.eq(entity_aux.queue_command_master, real, "the game's function is back once they move again")
end)

test("tuning_a_weapon_leaves_others_of_its_type_alone_and_comes_off_again", function()
	local api, game = start()
	local shared = { firerate = 600, power = 100, reload_duration = 2, magazine_capacity = 30, attributes = { armorpiercing = false } }
	local function weapon() return { state = { weapon_info = shared, round_delay = 0.1, reload_duration = 2, magazine_capacity = 30, turn_speed_aim = 1 } } end
	local mine, theirs, slave = weapon(), weapon(), weapon()
	local remade = 0
	local component = { set_protected = function(_, state, key, value) state[key] = value end, remake_raycast = function() remade = remade + 1 end }
	local unit, other = "gun", "their gun"
	local aux = rawget(_G, "EntityAux")
	local real = { get_component = aux.get_component }
	aux.get_component = function(name) return name == "firearm" and component or real.get_component(name) end
	local st = api.game.state()
	local em = st.entity_manager
	local has, masters, slaves = em.has_entities, em.get_master_entities, em.get_slave_entities
	em.has_entities = function(self, name) return name == "firearm" or has(self, name) end
	em.get_master_entities = function(self, name) if name == "firearm" then return { [unit] = mine, [other] = theirs } end return masters(self, name) end
	em.get_slave_entities = function(self, name) if name == "firearm" then return { [unit] = slave } end return slaves(self, name) end

	E.eq(api.gear.tune(unit, "firearm", { firerate = 2, magazine_capacity = 1.5, armorpiercing = true, turn_speed_aim = 1.2 }), true)
	E.eq(mine.state.weapon_info.firerate, 1200)
	E.eq(mine.state.round_delay, 0.05)
	E.eq(mine.state.magazine_capacity, 45)
	E.eq(mine.state.turn_speed_aim, 1.2, "a field only the state holds")
	E.eq(mine.state.weapon_info.attributes.armorpiercing, true)
	E.eq(slave.state.weapon_info.firerate, 1200, "the copy the shots read")
	E.eq(remade, 1, "the hit test takes the new settings")
	E.eq(shared.firerate, 600, "the type's settings are untouched")
	E.eq(shared.attributes.armorpiercing, false)
	E.eq(theirs.state.weapon_info, shared, "another player's weapon is untouched")
	E.eq(mine.state.tuned_base, shared)
	api.gear.tune(unit, "firearm", { power = 1.5 })
	E.eq(mine.state.weapon_info.firerate, 600, "each call replaces the last")
	E.eq(mine.state.weapon_info.power, 150)
	E.eq(mine.state.round_delay, 0.1)
	E.eq(mine.state.turn_speed_aim, 1, "back to the game's default")
	E.eq(select(2, api.gear.tune("nope", "firearm", {})), "unknown")
	E.eq(select(2, api.gear.tune(unit, "firearm", 3)), "bad_type")
	aux.get_component = real.get_component
	em.has_entities, em.get_master_entities, em.get_slave_entities = has, masters, slaves
end)

test("a_weapons_melee_is_tuned_alone_from_its_melee_settings", function()
	local api = start()
	local shared = { power = 50, duration = 0.33, center_distance = 1, radius = 1 }
	local mine, theirs, slave = { state = { melee_info = shared } }, { state = { melee_info = shared } }, { state = { melee_info = shared, melee_time = -1 } }
	local component = { set_protected = function(_, state, key, value) state[key] = value end }
	local unit, other = "gun", "their gun"
	local aux = rawget(_G, "EntityAux")
	local real = aux.get_component
	aux.get_component = function(name) return name == "melee" and component or real(name) end
	local em = api.game.state().entity_manager
	local has, masters, slaves = em.has_entities, em.get_master_entities, em.get_slave_entities
	em.has_entities = function(self, name) return name == "melee" or has(self, name) end
	em.get_master_entities = function(self, name) if name == "melee" then return { [unit] = mine, [other] = theirs } end return masters(self, name) end
	em.get_slave_entities = function(self, name) if name == "melee" then return { [unit] = slave } end return slaves(self, name) end

	E.eq(api.gear.tune(unit, "melee", { power = 1.2, duration = 1 / 1.25, delay = 0.5 }), true)
	E.eq(mine.state.melee_info.power, 60)
	E.eq(math.floor(slave.state.melee_info.duration * 1000 + 0.5), 264, "the copy the hit and the swing's length read")
	E.eq(slave.state.melee_info.delay, nil, "a field its settings leave out stays out")
	E.eq(slave.state.melee_info.attributes, nil, "melee settings have no attributes")
	E.eq(shared.power, 50, "the type's settings are untouched")
	E.eq(theirs.state.melee_info, shared, "another player's weapon is untouched")
	api.gear.tune(unit, "melee", {})
	E.eq(mine.state.melee_info.power, 50, "{} takes it off")
	aux.get_component = real
	em.has_entities, em.get_master_entities, em.get_slave_entities = has, masters, slaves
end)

test("a_screen_of_the_game_shows_a_mods_words_only_while_it_runs", function()
	local api = start()
	local lm = rawget(_G, "LocalizationManager")
	local real = lm.string
	local seen = api.ui.with_texts({ menu_campaign = "SURVIVORS" }, function(extra)
		return lm:string("menu_campaign") .. "|" .. lm:string("weapon_rifle") .. "|" .. extra
	end, "x")
	E.eq(seen, "SURVIVORS|Rifle|x", "the mod's word, the game's others, and fn's arguments")
	E.eq(lm.string, real, "the game's function is back")
	E.eq(lm:string("menu_campaign"), "LOC:menu_campaign")
	local ok, err = pcall(api.ui.with_texts, { a = "b" }, function() error("boom", 0) end)
	E.eq(ok, false)
	E.eq(err, "boom", "an error comes through")
	E.eq(lm.string, real, "and the function is back after it too")
end)

test("a_prop_is_a_unit_on_this_machine_that_moves_grows_and_goes", function()
	local api = start()
	local units, destroyed = {}, 0
	rawset(_G, "World", {
		spawn_unit = function(world, name, pos)
			if name == "missing" then error("unit not found") end
			local u = { world = world, name = name, pos = pos }
			units[#units + 1] = u
			return u
		end,
		destroy_unit = function(_, u) u.alive = false destroyed = destroyed + 1 end,
	})
	local unit = rawget(_G, "Unit")
	unit.teleport_local_position = function(u, _, pos) u.pos = pos end
	unit.set_local_scale = function(u, _, v) u.scale = v.x end
	local p = api.world.prop("gameobjects_spec/gameobject_bug_research_object", { x = 1, y = 2 }, { scale = 1.5 })
	E.eq(units[1].world, "world", "in the mission's world")
	E.eq(units[1].pos.z, 0)
	E.eq(units[1].scale, 1.5)
	E.eq(p:move({ x = 4, y = 5, z = 1 }), true)
	E.eq(units[1].pos.x, 4)
	E.eq(p:scale(2), true)
	E.eq(units[1].scale, 2)
	unit.flow_event = function(u, event) u.flowed = event end
	E.eq(p:flow("on_picked_up"), true)
	E.eq(units[1].flowed, "on_picked_up")
	p:remove()
	E.eq(destroyed, 1)
	E.eq(p:alive(), false)
	E.eq(p:move({ x = 0, y = 0 }), false, "a removed prop does nothing")
	p:remove()
	E.eq(destroyed, 1, "removed once")
	E.eq(select(2, api.world.prop("missing", { x = 0, y = 0 })), "unknown")
	E.eq(select(2, api.world.prop("x", "here")), "bad_type")
	E.eq(select(2, api.world.prop(nil, { x = 0, y = 0 })), "bad_type")
end)

-- The map: its units both ways, the view, pings, opening it from code, and markers.
local function map_world()
	local map_state = {}
	-- As the game's: it leaves when the map input is off or exit_map is set.
	map_state.update = function(unit, ctx)
		if not ctx.state.input.map or ctx.state.exit_map then
			map_state.on_exit(unit, ctx)
			ctx.state.current_state = nil
			ctx.state.exit_map = nil
		end
	end
	map_state.on_exit = function() end
	rawset(_G, "CharacterStateMap", map_state)
	rawset(_G, "CharacterStateMachine", { change_state = function(_, ctx, s) ctx.state.current_state = s end })
	rawset(_G, "Hud", { draw_map = function(_, _, _, _, gui) gui.drawn = (gui.drawn or 0) + 1 end })
	rawset(_G, "Vector2", function(x, y) return { x = x, y = y } end)
	rawset(_G, "Color", function(a, r, g, b) return { a, r, g, b } end)
	-- The loader installs a waiting hook when the game next requires a module.
	rawget(_G, "require")("lua/network/network_global")
end

test("the_map_converts_both_ways_and_its_view_follows_zoom_and_pan", function()
	local api, game = start()
	local V = rawget(_G, "Vector3")
	local x, y = api.map.to_map(V(150, 120, 0))
	local back = api.map.to_world(x, y)
	E.eq(math.abs(back.x - 150) < 1e-6 and math.abs(back.y - 120) < 1e-6, true, "there and back")
	E.eq(math.abs(api.map.to_world(0, 0).x - 96) < 1e-6, true, "the middle of the map is the middle of the level")
	game.state.hud.hud_info = { { map_zoom = 1, map_offset = { x = 0, y = 0 }, map_pan_vel = { x = 3, y = 3 } } }
	E.eq(api.map.zoom(), 1)
	E.eq(api.map.set_zoom(2), true)
	E.eq(api.map.zoom(), 2)
	E.eq(select(2, api.map.set_zoom(9)), "out_of_range")
	E.eq(select(2, api.map.set_zoom("far")), "bad_type")
	E.eq(api.map.look_at(V(150, 120, 0)), true)
	local view = api.map.view()
	E.eq(math.abs(view.x - 150) < 1e-6 and math.abs(view.y - 120) < 1e-6, true, "the crosshair is on the point at any zoom")
	E.eq(game.state.hud.hud_info[1].map_pan_vel.x, 0, "a pan in flight stops")
	E.eq(api.map.cursor, api.map.view)
end)

test("every_players_ping_reads_back_as_a_world_position", function()
	local api = start()
	local V = rawget(_G, "Vector3")
	E.eq(#api.map.pings(), 0, "no ping is the field's default")
	api.ui.ping(V(150, 120, 0))
	local pings = api.map.pings()
	E.eq(#pings, 1)
	E.eq(math.abs(pings[1].position.x - 150) < 1e-6 and math.abs(pings[1].position.y - 120) < 1e-6, true)
	E.eq(pings[1].owned, true)
	api.ui.ping(nil)
	E.eq(#api.map.pings(), 0)
end)

test("opening_the_map_holds_its_input_until_the_player_takes_over", function()
	local api, game = start()
	map_world()
	local ctx = { state = game.me.master.character }
	game.masters.character = { [game.me] = ctx }
	ctx.state.input = { map = false }
	-- The pad manager's live table for this player.
	local pad = { map_pressed = false }
	game.me.master.input = { input_data = pad }
	local map_state = rawget(_G, "CharacterStateMap")
	E.eq(api.map.is_open(), false)
	E.eq(api.map.open(), true)
	E.eq(api.map.is_open(), true)
	map_state.update(game.me, ctx)
	E.eq(api.map.is_open(), true, "held open with the key up")
	-- The player holds the map key: the map is theirs, and letting go closes it.
	pad.map_pressed = true
	map_state.update(game.me, ctx)
	E.eq(api.map.is_open(), true)
	pad.map_pressed = false
	ctx.state.input.map = false
	map_state.update(game.me, ctx)
	E.eq(api.map.is_open(), false)
	-- Closed from code, nothing forced is left behind for the idle state to open it again on.
	api.map.open()
	map_state.update(game.me, ctx)
	E.eq(ctx.state.input.map, true)
	E.eq(api.map.close(), true)
	E.eq(ctx.state.exit_map, true)
	map_state.update(game.me, ctx)
	E.eq(api.map.is_open(), false, "close() leaves the next frame")
	E.eq(ctx.state.input.map, false)
	game.state.players[1].player_go.avatar_unit = nil
	E.eq(select(2, api.map.open()), "no_avatar")
	game.state.players[1].player_go.avatar_unit = game.me
	game.me.master.input = nil
end)

test("a_marker_draws_after_the_map_in_its_units_and_tells_the_clip_its_icon_once", function()
	local api, game, e, ui = start()
	map_world()
	local V = rawget(_G, "Vector3")
	local gui = { images = {}, clippable = {} }
	function gui:image(icon, pos, size, color) self.images[#self.images + 1] = { icon = icon, pos = pos, size = size, color = color } end
	function gui:add_clippable_materials(m) self.clippable[m] = (self.clippable[m] or 0) + 1 end
	local info, hud = { map_zoom = 2 }, { generated_level = game.state.generated_level }
	local draw = rawget(_G, "Hud").draw_map
	local m = api.map.marker(V(96, 96, 0), { size = 10, color = { 255, 1, 2, 3 } })
	draw(hud, 0.016, 0, info, gui)
	E.eq(gui.drawn, 1, "the game's map is drawn")
	E.eq(gui.images[1].icon, "icon_map_pickup_generic")
	E.eq(math.abs(gui.images[1].pos.x + 5) < 1e-6, true, "the middle of the level, less half the size")
	E.eq(gui.images[1].color[2], 1)
	E.eq(gui.clippable.icon_map_pickup_generic, 1)
	draw(hud, 0.016, 0, info, gui)
	E.eq(gui.clippable.icon_map_pickup_generic, 1, "once per gui")
	m:move(V(196, 96, 0))
	gui.images = {}
	draw(hud, 0.016, 0, info, gui)
	local expected = 100 * math.cos(math.rad(45)) * 350 / (10 * 48) * 2 * 2 - 5
	E.eq(math.abs(gui.images[1].pos.x - expected) < 1e-6, true, "scaled by the zoom")
	-- One that follows a unit, and stops when it's gone.
	local follower = api.map.marker(nil, { unit = game.me, icon = "map_marker_ping" })
	gui.images = {}
	draw(hud, 0.016, 0, info, gui)
	E.eq(#gui.images, 2)
	E.eq(gui.clippable.map_marker_ping, 1)
	game.me.alive = false
	gui.images = {}
	draw(hud, 0.016, 0, info, gui)
	E.eq(#gui.images, 1)
	game.me.alive = true
	follower:remove()
	-- A broken marker costs the markers, not the map.
	m:set({ color = 5 })
	gui.drawn = 0
	draw(hud, 0.016, 0, info, gui)
	E.eq(gui.drawn, 1)
	E.has(e:log("obsidianapi"), "map markers:")
	m:remove()
	gui.images = {}
	draw(hud, 0.016, 0, info, gui)
	E.eq(#gui.images, 0)
	E.eq(select(2, api.map.marker(nil)), "bad_type")
	E.eq(select(2, api.map.marker(V(1, 1, 0), { size = -1 })), "out_of_range")
	-- A marker belongs to its mission.
	api.map.marker(V(96, 96, 0))
	local was = ui.state_object
	ui.state_object = { players = {} }
	gui.images = {}
	draw(hud, 0.016, 0, info, gui)
	E.eq(#gui.images, 0)
	ui.state_object = was
end)

-- Bots: three free records past the player's, pad 1 held by a real controller, and units that land
-- when the test says.
local function bot_world(game, e)
	rawset(_G, "KEYBOARD_MOUSE_PAD_NUMBER", 0)
	rawset(_G, "Pads", { 1, 2, 3, 4 })
	rawset(_G, "PadManager", { is_pad_active = function(_, pad) return pad == 1 end })
	rawset(_G, "ControllerSettings", { pad = {
		{ func = "axis", output_name = "move" }, { func = "axis", output_name = "aim" }, { func = "button", output_name = "fire_button" },
		{ func = "button", output_name = "throw" }, { func = "pressed", output_name = "reload_pressed" }, { func = "button", output_name = "sprint_button" },
		{ func = "filter", key = "FilterSticky", output_name = "sprint_pressed" }, { func = "pressed", output_name = "melee" },
		{ func = "pressed", output_name = "switch_weapon_down" }, { func = "filter", key = "FilterVirtualAxis", output_name = "look" },
		{ func = "pressed", output_name = "interact" },
	} })
	local st = game.state
	local function go_for()
		local go = { made = false }
		function go:exists() return self.made end
		function go:owned() return self.made end
		function go:get(k) return self[k] end
		function go:set(k, v) self[k] = v end
		function go:has(k) return self[k] ~= nil end
		function go:create() self.made = true end
		function go:destroy() self.made = false self.avatar_unit = nil end
		return go
	end
	for i = 2, 4 do st.players[i] = { player_go = go_for(), pad_number = 0 } end
	st.rpc_add_local_player_accepted = function(self, peer, pad, index)
		local r = self.players[index]
		r.player_go:set("name", rawget(_G, "UserManager"):get_name(pad))
		r.player_go:create()
		r.pad_number = pad
		game.joined = { peer = peer, pad = pad, index = index }
	end
	st.dropped_pads = {}
	st.despawn_player = function(self, pad)
		for _, r in ipairs(self.players) do
			if r.pad_number == pad and r.player_go:owned() then
				r.player_go:destroy()
				r.pad_number = 0
				self.dropped_pads[pad] = true
			end
		end
	end
	local function body(pos, pad)
		return { alive = true, owned = true, pos = pos or { x = 1, y = 1, z = 0 },
			master = { input = { pad_number = pad, has_focus = false }, character = {}, inventory = {}, customization = {} }, slave = {} }
	end
	game.land = function(index, pos)
		local u = body(pos, st.players[index].pad_number)
		st.players[index].player_go:set("avatar_unit", u)
		return u
	end
	st.spawn_manager = { queue_spawn = function(_, path, pos, _, listener, data)
		game.queued = { path = path, pos = pos, data = data }
		local u = body({ x = pos.x, y = pos.y, z = 0 }, nil)
		game.bare = u
		listener:on_entity_spawned(u)
	end }
	rawget(_G, "CameraManager").follow_spin = 0
	-- The game's input component, which reads its players' input once a frame.
	rawset(_G, "InputComponent", { update_masters = function() end })
	-- An early Hud with an update, which the real script writes over when it loads.
	rawset(_G, "Hud", rawget(_G, "Hud") or { draw_map = function() end })
	rawget(_G, "Hud").update = function() end
	rawget(_G, "require")("lua/network/network_global")
	rawget(_G, "Hud").update = function() end
	e.opts.modules["lua/hud/hud"] = true
	rawget(_G, "require")("lua/hud/hud")
	game.step = function(n)
		for _ = 1, n do
			e:frames(1)
			rawget(_G, "InputComponent").update_masters({}, 0.016)
		end
	end
	return st
end

test("a_bot_joins_as_a_local_player_on_a_free_pad_with_input_of_the_games_types", function()
	local api, game, e = start()
	local st = bot_world(game, e)
	local bot = api.bots.add({ name = "Dummy" })
	E.eq(bot.kind, "player")
	E.eq(game.joined.pad, 2, "pad 0 is the keyboard and pad 1 a controller someone holds")
	E.eq(game.joined.index, 2)
	E.eq(game.joined.peer, "me")
	E.eq(st.players[2].player_go.name, "Dummy")
	E.eq(rawget(_G, "UserManager"):get_name(2), "Tester", "the name is lent only while it joins")
	E.eq(bot:alive(), false, "still in its hellpod")
	local u = game.land(2)
	E.eq(bot:unit(), u)
	bot:move({ x = 1, y = 0 }):fire(true)
	game.step(1)
	local input = u.master.input
	E.eq(input.has_focus, true)
	local t = input.input_data
	E.eq(t.is_active, true)
	E.eq(t.fire_button, 1)
	E.eq(t.throw, 0, "a button is a number")
	E.eq(t.reload_pressed, false, "a press is a boolean")
	E.eq(t.sprint_pressed, false, "so is a filter")
	E.eq(type(t.look), "table", "a virtual axis is a vector")
	E.eq(t.fire_button_primary_vehicle, 0, "a seat reads these")
	E.eq(t.move.x, 1)
	E.eq(t.aim.x, 0)
	bot:reload()
	game.step(1)
	E.eq(t.reload_pressed, true)
	game.step(1)
	E.eq(t.reload_pressed, true, "held two frames")
	game.step(1)
	E.eq(t.reload_pressed, false)
	bot:throw()
	game.step(1)
	E.eq(t.throw, 1)
	game.step(10)
	E.eq(t.throw, 0, "and let go")
	-- The camera turns the sticks: a world direction is turned back for the stick.
	rawget(_G, "CameraManager").follow_spin = math.pi / 2
	bot:move({ x = 0, y = 1 })
	game.step(1)
	E.eq(math.abs(t.move.x - 1) < 1e-6 and math.abs(t.move.y) < 1e-6, true)
	rawget(_G, "CameraManager").follow_spin = 0
	E.eq(select(2, api.bots.add({ name = 5 })), "bad_type")
	E.eq(select(2, api.bots.add({ name = string.rep("x", 33) })), "bad_type")
end)

test("a_bot_fights_and_follows_by_itself", function()
	local api, game, e = start()
	bot_world(game, e)
	local bot = api.bots.add()
	E.eq(bot.name, "Bot 1")
	local u = game.land(2, { x = 20, y = 1, z = 0 })
	bot:follow(nil, 6)
	game.step(1)
	local t = u.master.input.input_data
	E.eq(t.move.x, -1, "walks to the player")
	u.pos = { x = 4, y = 1, z = 0 }
	game.step(1)
	E.eq(t.move.x, 0, "and stops near")
	bot:fight(25)
	game.step(1)
	E.eq(t.fire_button, 1)
	E.eq(math.abs(t.aim.x - (-1 / math.sqrt(2))) < 1e-6, true, "aims at the nearest enemy")
	bot:fight(false)
	game.step(1)
	E.eq(t.fire_button, 0)
end)

test("past_the_records_a_bot_is_a_bare_helldiver_and_bots_go_before_the_mission_ends", function()
	local api, game, e = start()
	local st = bot_world(game, e)
	local a, _, c = api.bots.add(), api.bots.add(), api.bots.add()
	E.eq(c.kind, "player")
	E.eq(game.joined.pad, 4)
	E.eq(select(2, api.bots.add({ bare = false })), "no_slot")
	local bare = api.bots.add({ name = "Dummy" })
	E.eq(bare.kind, "bare")
	E.eq(game.queued.path, "characters/helldiver_private_01/helldiver_private_01")
	E.eq(game.queued.pos.x, 4, "beside the player")
	E.eq(G.command(game, "inventory", "loadout").data.pad_number, 0, "in the main player's loadout")
	E.eq(bare:unit(), game.bare)
	game.step(1)
	-- Every pad of the game is taken: a pad number it doesn't have would fail in its filter code.
	E.eq(game.bare.master.input.pad_number, nil, "no pad, so it stands still")
	E.eq(game.bare.master.input.input_data, nil)
	E.eq(#api.bots.all(), 4)
	E.eq(a:remove(), true)
	E.eq(st.players[2].player_go:exists(), false)
	E.eq(st.dropped_pads[2], nil, "a pad never plugged in isn't waited for")
	E.eq(#api.bots.all(), 3)
	local still = game.bare
	local driven = api.bots.add({ bare = true })
	game.step(1)
	E.eq(game.bare.master.input.pad_number, 2, "the pad a bot left")
	E.eq(game.bare.master.input.input_data.is_active, true)
	E.eq(select(2, api.bots.add({ bare = false })), "no_slot", "a record, but its pad is the bare bot's")
	E.eq(driven.kind, "bare")
	still.alive = false
	E.eq(bare:alive(), false)
	st.game_state = "game_over"
	game.step(1)
	E.eq(#api.bots.all(), 0)
	E.eq(st.players[3].player_go:exists(), false, "gone before the results")
	st.game_state = nil
end)

test("a_bare_bot_wears_its_name_as_a_player_does_and_it_needs_the_input_hook_to_move", function()
	local api, game, e = start()
	local st = bot_world(game, e)
	for _ = 1, 3 do api.bots.add() end
	local bare = api.bots.add({ name = "Dummy" })
	E.eq(bare.kind, "bare")
	local texts = {}
	local gui = { screen_resolution = { width = 1920, height = 1080 }, ui_scale = 1 }
	function gui:set_current_font(f) self.font = f end
	function gui:set_ignore_position_scale() end
	function gui:text(text, pos) texts[#texts + 1] = { text = text, pos = pos, font = self.font } end
	local hud = { hd_gui = gui, plugins_enabled = true, gui_metrics_cacher = { get_text_extents = function() return 60, 20 end } }
	rawget(_G, "CameraManager").get_viewport = function() return { world_to_viewport = function(_, p) return { x = p.x * 10, y = 7, z = p.y * 10 } end } end
	rawget(_G, "Hud").update(hud, 0.016)
	E.eq(#texts, 1, "the players with a record have the game's")
	E.eq(texts[1].text, "Dummy")
	E.eq(texts[1].font, "body")
	E.eq(texts[1].pos.x, 40 + -30 + 6, "centred and a little above, as the HUD puts a name")
	E.eq(texts[1].pos.y, 40 + -10 - 21, "the screen's height is the projection's z; its y is the depth")
	-- With the HUD's plugins off (hide_hud, a mod's full-screen menu), the names go with the game's.
	hud.plugins_enabled = false
	texts = {}
	rawget(_G, "Hud").update(hud, 0.016)
	E.eq(#texts, 0, "hidden with the HUD")
	hud.plugins_enabled = true
	-- Off the screen, no name.
	game.bare.pos = { x = -5, y = 1, z = 0 }
	texts = {}
	rawget(_G, "Hud").update(hud, 0.016)
	E.eq(#texts, 0)
	-- Input is handed over only when the input component reads it: vectors from the frame callback
	-- would be a frame old.
	local first = api.bots.all()[1]
	local u = game.land(2)
	first:move({ x = 1, y = 0 })
	e:frames(1)
	E.eq(u.master.input.input_data, nil, "not from the frame callback")
	game.step(1)
	E.eq(u.master.input.input_data.move.x, 1)
	api.bots.remove_all()
end)

test("only_the_host_in_a_mission_makes_bots", function()
	local api, game, e = start({ host = false })
	bot_world(game, e)
	E.eq(select(2, api.bots.add()), "host_only")
	local api2, game2, e2, ui2 = start()
	bot_world(game2, e2)
	ui2.state = "menu"
	E.eq(select(2, api2.bots.add()), "not_in_mission")
end)

-- The soundtrack: the ship and the main menu swap their one music id, a mission's machine is muted.
local function ship(game, ui)
	local st = { music_timpani_world = "music world" }
	st.menu_music_id = rawget(_G, "TimpaniWorld").trigger_event("music world", "music_bridge")
	ui.state, ui.state_object = "menu", st
	return st
end

local function mission_machine(game)
	local T = rawget(_G, "TimpaniWorld")
	local machine = game.state.audio_state_machine
	machine.timpani_world = "sound world"
	machine.music_threat = T.trigger_event("sound world", "music_threat")
	T.set_parameter("sound world", machine.music_threat, "volume", 1)
	return machine
end

test("a_held_track_takes_the_ships_slot_and_the_ship_gets_its_own_back", function()
	local api, game, e, ui = start()
	local st = ship(game, ui)
	local bridge = st.menu_music_id
	E.eq(api.music.now().place, "ship")
	E.eq(api.music.play("music_credits"), true)
	E.eq(game.playing[bridge], nil, "the bridge music stopped")
	E.eq(game.played[st.menu_music_id].name, "music_credits", "the ship's own fades work on it")
	E.eq(api.music.now().track, "music_credits")
	E.eq(api.music.release(), true)
	E.eq(game.played[st.menu_music_id].name, "music_bridge")
	E.eq(game.playing[st.menu_music_id], true)
	E.eq(api.music.now().held, false)
end)

test("a_held_track_loops_and_comes_back_when_the_screen_swaps_its_own_in", function()
	local api, game, e, ui = start()
	local st = ship(game, ui)
	api.music.play("music_credits")
	local first = st.menu_music_id
	game.playing[first] = nil
	e:frames(1)
	E.eq(game.played[st.menu_music_id].name, "music_credits")
	E.eq(st.menu_music_id ~= first, true, "started again")
	-- As the credits screen does: it stops everything and starts its own.
	st.menu_music_id = rawget(_G, "TimpaniWorld").trigger_event("music world", "music_theme")
	local theirs = st.menu_music_id
	e:frames(1)
	E.eq(game.playing[theirs], nil)
	E.eq(game.played[st.menu_music_id].name, "music_credits")
end)

test("silence_leaves_a_stopped_id_in_the_slot", function()
	local api, game, e, ui = start()
	local st = ship(game, ui)
	local bridge = st.menu_music_id
	E.eq(api.music.stop(), true)
	E.eq(st.menu_music_id, bridge, "the ship sets its volume each frame and never checks for nil")
	E.eq(game.playing[bridge], nil)
	e:frames(3)
	E.eq(st.menu_music_id, bridge)
	E.eq(api.music.now().held, true)
	E.eq(api.music.now().track, nil)
	api.music.release()
	E.eq(game.played[st.menu_music_id].name, "music_bridge")
end)

test("in_a_mission_the_machines_music_goes_quiet_beside_the_held_track", function()
	local api, game, e = start()
	local machine = mission_machine(game)
	E.eq(api.music.now().place, "mission")
	E.eq(api.music.play("music_theme"), true)
	E.eq(game.params[machine.music_threat].volume, 0)
	local mine
	for id, p in pairs(game.played) do
		if p.name == "music_theme" and game.playing[id] then mine = id end
	end
	E.eq(mine ~= nil, true, "the held track plays in the mission's world")
	E.eq(game.params[mine].volume, 1)
	-- The machine starts its action music: quiet from the first frame, through the hook.
	rawset(_G, "AudioStateMachine", { on_enter_state = function(self, name)
		self.music_action = rawget(_G, "TimpaniWorld").trigger_event("sound world", "music_action_bugs")
		rawget(_G, "TimpaniWorld").set_parameter("sound world", self.music_action, "volume", 1)
	end })
	rawget(_G, "require")("lua/network/network_global")
	rawget(_G, "AudioStateMachine").on_enter_state(machine, "action")
	E.eq(game.params[machine.music_action].volume, 0)
	api.music.release()
	E.eq(game.playing[mine], nil)
	E.eq(game.params[machine.music_threat].volume, 1)
	E.eq(game.params[machine.music_action].volume, 1)
end)

test("a_pause_is_real_where_music_has_its_own_world_and_silence_elsewhere", function()
	local api, game, e, ui = start()
	local machine = mission_machine(game)
	api.music.pause()
	E.eq(game.params[machine.music_threat].volume, 0)
	game.params[machine.music_threat].volume = 0.7
	e:frames(1)
	E.eq(game.params[machine.music_threat].volume, 0, "held against the machine's fade")
	E.eq(api.music.now().paused, true)
	-- The pause goes with the player to the ship, where its world pauses.
	ship(game, ui)
	e:frames(1)
	E.eq(game.paused_worlds["music world"], true)
	api.music.resume()
	E.eq(game.paused_worlds["music world"], nil)
end)

test("a_hold_follows_to_the_next_screen", function()
	local api, game, e, ui = start()
	local st = ship(game, ui)
	api.music.play("music_theme")
	ui.state, ui.state_object = "game", game.state
	local machine = mission_machine(game)
	e:frames(1)
	E.eq(game.params[machine.music_threat].volume, 0)
	local found = false
	for id, p in pairs(game.played) do
		if p.name == "music_theme" and p.world == "sound world" and game.playing[id] then found = true end
	end
	E.eq(found, true)
end)

test("only_a_track_the_screen_has_plays", function()
	local api, game, e, ui = start()
	E.eq(select(2, api.music.play("music_bridge")), "no_music", "no music in this state")
	ship(game, ui)
	local list = api.music.tracks()
	E.eq(#list, 3)
	E.eq(list[1], "music_theme")
	E.eq(select(2, api.music.play("music_threat")), "unknown_event")
	E.eq(select(2, api.music.play(5)), "unknown_event")
	E.eq(api.music.volume(-12), true)
	e:frames(1)
	E.eq(game.buses.music_master, -12)
end)

test("a_client_mod_holds_its_own_music", function()
	local api, game, e, ui = start({ scope = "client" })
	ship(game, ui)
	E.eq(api.music.play("music_credits"), true)
	E.eq(api.music.pause(), true)
end)

test("a_bot_goes_to_a_point_and_stops_there_and_faces_where_its_told", function()
	local api, game, e = start()
	bot_world(game, e)
	local bot = api.bots.add()
	local u = game.land(2, { x = 1, y = 1, z = 0 })
	bot:go_to(rawget(_G, "Vector3")(11, 1, 0))
	game.step(1)
	local t = u.master.input.input_data
	E.eq(t.move.x, 1, "walks there")
	u.pos = { x = 10, y = 1, z = 0 }
	game.step(1)
	E.eq(t.move.x, 0, "and stops within 1.5 metres")
	u.pos = { x = 1, y = 1, z = 0 }
	game.step(1)
	E.eq(t.move.x, 0, "arrived once, it stays")
	bot:aim_at({ x = 1, y = 9 })
	game.step(1)
	E.eq(t.aim.y, 1)
	bot:follow(nil, 6)
	bot:go_to({ x = 30, y = 1 })
	game.step(1)
	E.eq(t.move.x, 1, "go_to takes over from follow")
	bot:hold()
	game.step(1)
	E.eq(t.move.x, 0)
end)

test("a_bot_is_given_gear_and_help_as_the_player_is", function()
	local api, game, e = start()
	bot_world(game, e)
	local bot = api.bots.add()
	E.eq(select(2, bot:give_weapon("primary_weapon", "equipment/wep_assaultrifle_01")), "no_avatar", "still in its hellpod")
	local u = game.land(2)
	game.commands = {}
	E.eq(bot:give_weapon("primary_weapon", "equipment/wep_assaultrifle_01", 2), true)
	local c = G.command(game, "inventory", "set_weapon")
	E.eq(c.unit, u, "to the bot, not the player")
	E.eq(c.data.upgrade_value, 2)
	u.master.health, u.master.status = {}, {}
	E.eq(bot:heal(), true)
	E.eq(G.command(game, "health", "reset").unit, u)
	E.eq(bot:refill() ~= nil, true)
	E.eq(type(bot:weapons()), "table")
	no_tampering(game)
end)

-- Custom stratagems: a copy of a base in the game's list, with its own words, and the base on the wire.
local function wire_world(game)
	game.sent = {}
	rawset(_G, "NetworkRouter", {
		write_message_params = function(_, name, ...) game.sent[#game.sent + 1] = { name = name, args = { ... } } end,
		transmit_to = function(self, peer, name, ...)
			game.sent_to = peer
			self:write_message_params(name, ...)
		end,
		transmit_to_all_others = function(self, name, ...) self:write_message_params(name, ...) end,
		transmit_to_all = function(self, name, ...) self:write_message_params(name, ...) end,
	})
	local fields = {}
	rawset(_G, "GameObject", {
		set = function(self, field, value) fields[field] = value end,
		get = function(self, field) return fields[field] end,
	})
	rawget(_G, "LoadoutSettings").get_upgraded_stratagem = function(id) return rawget(_G, "LoadoutSettings").stratagems[id] end
	return fields
end

local PITCHFORK = { name = "test_pitchfork", base = "stratagem_machinegun", title = { en = "Pitchfork", fr = "Fourche" },
	description = "Three barrels.", code = { "down", "left", "up" }, icon = "stratagem_lasercannon", stats = { cooldown_time = 30 },
	upgrades = { { title = "Pitchfork Light", description = "Move while firing.", cost = 3, stats = { cooldown_time = 20 } } } }

test("a_custom_stratagem_is_a_copy_of_its_base_with_its_own_words", function()
	local api, game = start()
	wire_world(game)
	local s = rawget(_G, "LoadoutSettings")
	local count = #s.stratagems
	local made = api.stratagems.define(PITCHFORK)
	E.eq(made.id, count + 1)
	local e = s.stratagems[made.id]
	E.eq(e.id, made.id)
	E.eq(e.name, "test_pitchfork")
	E.eq(e.unit, "gameobjects/crate", "the base's")
	E.eq(e.icon, "stratagem_lasercannon")
	E.eq(e.combo[2], "left")
	E.eq(e.cooldown_time, 30)
	E.eq(e.stratagem_subtypes[1], "support_weapon")
	E.eq(e.upgrades[1].cooldown_time, 20)
	E.eq(e.upgrades[1].cost, 3)
	E.eq(s.stratagems[2].cooldown_time, 0, "the base is left alone")
	local lm = rawget(_G, "LocalizationManager")
	E.eq(lm:string("test_pitchfork"), "Pitchfork")
	E.eq(lm:string(e.description), "Three barrels.")
	E.eq(lm:string(e.upgrades[1].upgrade_name), "Pitchfork Light")
	lm.current_language = "fr"
	E.eq(lm:string("test_pitchfork"), "Fourche")
	E.eq(api.catalog.text("test_pitchfork"), "Fourche")
	E.eq(lm:string("weapon_rifle"), "Rifle", "other texts are the game's")
	E.eq(api.stratagems.custom()[1].base, "stratagem_machinegun")
	E.eq(api.catalog.stratagem("test_pitchfork") ~= nil, true, "listed with the rest")
end)

test("a_custom_stratagem_without_upgrades_takes_the_bases_renamed", function()
	local api, game = start()
	wire_world(game)
	local made = api.stratagems.define({ name = "test_supply", base = "stratagem_resupply", title = "Big Supply" })
	local e = rawget(_G, "LoadoutSettings").stratagems[made.id]
	E.eq(#e.upgrades, 3)
	E.eq(e.upgrades[1].cooldown_time, 90)
	E.eq(rawget(_G, "LocalizationManager"):string(e.upgrades[2].upgrade_name), "Big Supply Mk3")
	E.eq(e.combo, nil, "the base's code, which this one has none of")
end)

test("a_custom_stratagem_unlocks_by_its_own_rule_or_its_base", function()
	local api, game, e = start_locked()
	wire_world(game)
	local P = rawget(_G, "ProgressionSettings")
	-- The game's answer by its rank lists: none of these is on one.
	P.can_use = function() return false end
	hooked(e)
	-- The machine gun is locked in this save.
	api.stratagems.define({ name = "test_base", base = "stratagem_machinegun", title = "Base" })
	api.stratagems.define({ name = "test_free", base = "stratagem_machinegun", title = "Free", unlock = "free" })
	api.stratagems.define({ name = "test_rank", base = "stratagem_machinegun", title = "Rank", unlock = { rank = 2 } })
	api.stratagems.define({ name = "test_manual", base = "stratagem_machinegun", title = "Manual", unlock = "manual" })
	E.eq(api.catalog.unlocked("test_base"), false, "as the base")
	E.eq(api.catalog.unlocked("test_free"), true)
	E.eq(api.catalog.unlocked("test_rank"), false, "rank 1")
	E.eq(api.catalog.unlocked("test_manual"), false)
	E.eq(P.can_use("test_free", "stratagems", 0), true, "the Loadout screen's question")
	E.eq(P.can_use("test_rank", "stratagems", 4999), false)
	E.eq(P.can_use("test_rank", "stratagems", 5000), true, "rank 2's XP")
	game.save.user.player_xp = 5000
	E.eq(api.catalog.unlocked("test_rank"), true)
	E.eq(api.stratagems.unlock("test_manual"), true)
	E.eq(api.stratagems.is_unlocked("test_manual"), true)
	E.eq(P.can_use("test_manual", "stratagems", 0), true)
	E.eq(api.stratagems.unlock("stratagem_resupply"), false, "only a custom one")
	E.eq(select(2, api.stratagems.define({ name = "test_bad", base = "stratagem_machinegun", title = "x", unlock = { rank = 0 } })), "bad_type")
	E.eq(select(2, api.stratagems.define({ name = "test_bad", base = "stratagem_machinegun", title = "x", unlock = "sometime" })), "bad_type")
end)

test("an_unlock_from_a_mod_is_kept_by_the_library_and_never_in_the_save", function()
	local api, game, e = start_locked()
	wire_world(game)
	hooked(e)
	api.stratagems.define({ name = "test_manual", base = "stratagem_machinegun", title = "Manual", unlock = "manual" })
	local core = e:core()
	local own = core.current
	core.current = { id = "dev.remix", mod = { options = {} } }
	E.eq(api.stratagems.unlock("test_manual"), true)
	core.current = own
	local storage = rawget(_G, "obsidian").storage
	E.eq(storage.get("stratagem_unlocks"), nil, "not yet: the running mod's storage isn't the library's")
	e:frames(1)
	E.eq(storage.get("stratagem_unlocks"), "test_manual", "kept from the library's frame")
	for _, name in ipairs(game.save.user.item_unlocks) do E.eq(name ~= "test_manual", true, "the save is left alone") end
	E.eq(api.stratagems.lock("test_manual"), true)
	E.eq(storage.get("stratagem_unlocks"), "", "the library's own call is kept at once")
	E.eq(api.catalog.unlocked("test_manual"), false)
end)

test("a_definition_is_checked", function()
	local api, game = start()
	wire_world(game)
	E.eq(select(2, api.stratagems.define({ name = "stratagem_resupply", base = "stratagem_resupply", title = "x" })), "taken")
	E.eq(select(2, api.stratagems.define({ name = "test_a", base = "stratagem_nothing", title = "x" })), "not_available")
	E.eq(select(2, api.stratagems.define({ name = "test a", base = "stratagem_resupply", title = "x" })), "bad_name")
	E.eq(select(2, api.stratagems.define({ name = "test_a", base = "stratagem_resupply" })), "bad_type", "no title")
	E.eq(select(2, api.stratagems.define({ name = "test_a", base = "stratagem_resupply", title = "x", code = { "up", "sideways" } })), "bad_code")
	E.eq(select(2, api.stratagems.define({ name = "test_a", base = "stratagem_resupply", title = "x", stats = { cooldown_time = "slow" } })), "bad_type")
	E.eq(select(2, api.stratagems.define({ name = "test_a", base = "stratagem_resupply", title = "x", stats = { unit = "gameobjects/other" } })), "bad_type", "unit has its own field")
	E.eq(select(2, api.stratagems.define({ name = "test_a", base = "stratagem_resupply", title = "x", type = "weird" })), "bad_type")
	E.eq(api.stratagems.define({ name = "test_a", base = "stratagem_resupply", title = "x" }) ~= nil, true)
	E.eq(select(2, api.stratagems.define({ name = "test_a", base = "stratagem_resupply", title = "x" })), "taken", "once")
end)

test("on_the_wire_a_custom_stratagem_is_its_base", function()
	local api, game = start()
	local fields = wire_world(game)
	local made = api.stratagems.define(PITCHFORK)
	local router = rawget(_G, "NetworkRouter")
	router:transmit_to("friend", "rpc_activate_stratagem", "pos", made.id, 0, "unit")
	E.eq(game.sent[1].args[2], 2, "the machine gun's id")
	router:transmit_to("me", "rpc_activate_stratagem", "pos", made.id, 0, "unit")
	E.eq(game.sent[2].args[2], made.id, "this machine keeps the real one")
	router:transmit_to_all("rpc_stratagem_info", "unit", 1, made.id, 0, 0)
	E.eq(game.sent[3].args[3], 2, "to the others")
	E.eq(game.sent[4].args[3], made.id, "and to itself")
	router:transmit_to("friend", "rpc_activate_stratagem", "pos", 5, 0, "unit")
	E.eq(game.sent[5].args[2], 5, "a stratagem of the game's goes as it is")
	local go = rawget(_G, "GameObject")
	go.set({}, "stratagem3_index", made.id)
	E.eq(fields.stratagem3_index, 2)
	local mine = {}
	go.set(mine, "stratagem4_index", made.id)
	E.eq(go.get(mine, "stratagem4_index"), made.id, "its own reads get the real one")
	go.set(mine, "stratagem4_index", 2)
	E.eq(go.get(mine, "stratagem4_index"), 2, "and the base when it's the base")
	go.set(mine, "hitpoints", made.id)
	E.eq(fields.hitpoints, made.id, "other fields are left alone")
end)

test("a_custom_stratagem_runs_its_mods_function_when_called_down", function()
	local spawned = 0
	rawset(_G, "StratagemComponent", { spawn_stratagem_hellpod = function() spawned = spawned + 1 end })
	local api, game = start()
	wire_world(game)
	rawget(_G, "require")("lua/network/network_global")
	local seen
	local def = {}
	for k, v in pairs(PITCHFORK) do def[k] = v end
	def.on_activate = function(ctx)
		seen = ctx
		return ctx.upgrade ~= 1
	end
	local made = api.stratagems.define(def)
	local c = rawget(_G, "StratagemComponent")
	c.spawn_stratagem_hellpod(c, "me", { x = 1, y = 2, z = 3 }, nil, made.id, 0, "thrower")
	E.eq(seen.name, "test_pitchfork")
	E.eq(seen.position.y, 2)
	E.eq(seen.owner, "thrower")
	E.eq(spawned, 1, "the base's drop too")
	c.spawn_stratagem_hellpod(c, "me", { x = 1, y = 2, z = 3 }, nil, made.id, 1, "thrower")
	E.eq(spawned, 1, "false drops nothing")
	c.spawn_stratagem_hellpod(c, "me", { x = 1, y = 2, z = 3 }, nil, 2, 0, "thrower")
	E.eq(spawned, 2)
end)

test("a_custom_stratagem_follows_its_base_for_packs_and_ranks", function()
	local api, game = start()
	wire_world(game)
	api.stratagems.define({ name = "test_paid", base = "stratagem_dlc", title = "Paid" })
	rawget(_G, "require")("lua/network/network_global")
	E.eq(api.catalog.usable("test_paid"), false, "a paid base stays paid")
	api.stratagems.define({ name = "test_free", base = "stratagem_resupply", title = "Free" })
	local progression = rawget(_G, "ProgressionSettings")
	progression.can_use = function(name) return name == "stratagem_resupply" end
	rawget(_G, "require")("lua/network/network_global")
	E.eq(progression.can_use("test_free", "stratagems", 0), true)
	E.eq(api.catalog.unlocked("test_free"), api.catalog.unlocked("stratagem_resupply"), "unlocked with its base")
	E.eq(api.catalog.unlocked("test_paid"), false)
end)

test("in_a_lobby_that_isnt_cleared_a_custom_stratagem_is_its_base", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local api, game, e = start({ lobby = lobby })
	wire_world(game)
	local made = api.stratagems.define(PITCHFORK)
	local s = rawget(_G, "LoadoutSettings")
	E.eq(s.get_upgraded_stratagem(made.id, 0).name, "test_pitchfork")
	e:frames(31)
	lobby.members = { "me", "stranger" }
	e:frames(31)
	E.eq(s.get_upgraded_stratagem(made.id, 0).name, "stratagem_machinegun")
end)

test("a_client_mod_cant_make_one", function()
	local api = start({ scope = "client" })
	E.eq(select(2, api.stratagems.define(PITCHFORK)), "needs_session")
end)

-- Another mod calling: obsidian.module looks in the mod that's running, so the API mustn't load its own
-- files during a call (found live: a bot's gear looked for lib/gear.lua in the caller).
test("calls_from_another_mod_find_the_apis_own_files", function()
	local api, game, e = start()
	bot_world(game, e)
	local bot = api.bots.add()
	local u = game.land(2)
	u.master.health, u.master.status = {}, {}
	api.stratagems.define({ name = "test_other", base = "stratagem_resupply", title = "Other" })
	local core = e:core()
	core.current = { id = "someone.else", mod = { scope = "session" }, errors = 0 }
	local ok, err = pcall(function()
		E.eq(bot:give_weapon("primary_weapon", "equipment/wep_assaultrifle_01"), true)
		E.eq(bot:heal(), true)
		E.eq(type(bot:weapons()), "table")
		E.eq(api.catalog.unlocked("test_other"), api.catalog.unlocked("stratagem_resupply"))
	end)
	core.current = core.mods_by_id["obsidian.api"]
	E.eq(ok, true, tostring(err))
end)

-- The pause menu and the game's Settings screen, as far as Mod Settings uses them.
local function fake_menus()
	local lm = rawget(_G, "LocalizationManager")
	local Base = { was_option_selected = function(_, input) return input.select == true end }
	local Options = setmetatable({}, { __call = function(cls, parent) return setmetatable({ parent = parent }, { __index = cls }) end })
	function Options.update(self)
		self.title, self.drawn = lm:string("menu_options_title"), {}
		for i, o in ipairs(self.options) do
			local v = o.options and o.options[o.options.selected_index]
			self.drawn[i] = lm:string(o.name) .. (v and (" = " .. (o.ignore_loc_option_names and v.name or lm:string(v.name))) or "")
		end
	end
	function Options.handle_input(self, input)
		local row = self.options[self.selected_option]
		if input.down then self.selected_option = self.selected_option % #self.options + 1 end
		if input.right and row.options then row.options.selected_index = row.options.selected_index % #row.options + 1 end
		if Base.was_option_selected(self, input) and row.on_pressed then row.on_pressed() end
		if input.back then self.parent:pop_menu() end
	end
	function Options.play_sound_accept() end
	local Main = {}
	function Main.on_enter(self)
		self.options = { { name = "menu_keybindings" }, { name = "menu_settings" }, { name = "menu_exit_to_menu" } }
		self.selected_option, self.states, self.state_stack = 1, self.states or {}, {}
	end
	function Main.update(self)
		self.drawn = {}
		for i, o in ipairs(self.options) do self.drawn[i] = lm:string(o.name) end
		local top = self.state_stack[#self.state_stack]
		if top then top:update(0, 0) end
	end
	function Main.handle_input(self, input)
		local top = self.state_stack[#self.state_stack]
		if top then return top:handle_input(input) end
		if Base.was_option_selected(self, input) then self.chose = self.options[self.selected_option].name end
	end
	function Main.push_menu(self, name, args)
		local s = assert(self.states[name], name)
		self.state_stack[#self.state_stack + 1] = s
		s:on_enter(args)
	end
	function Main.pop_menu(self)
		local gone = table.remove(self.state_stack)
		local top = self.state_stack[#self.state_stack]
		if top then top:on_enter(gone) end
	end
	function Main.play_sound_accept() end
	rawset(_G, "MenuScreenBase", Base)
	rawset(_G, "MenuScreenOptions", Options)
	rawset(_G, "MenuScreenMain", Main)
	return Main
end

test("mod_settings_opens_from_the_pause_menu_and_keeps_its_values", function()
	local api, _, e = start()
	local Main = fake_menus()
	-- Hooks on classes defined after boot go in at the next require.
	rawget(_G, "require")("lua/network/network_global")
	local core = e:core()
	local own = core.current
	-- Another mod's page, made from its own code.
	core.current = { id = "dev.hud", mod = { options = {} } }
	local changes = {}
	local page = api.mod_settings.page("Better HUD")
	E.eq(page:toggle("ammo", { label = "Ammo counter", default = true, on_change = function(v) changes[#changes + 1] = v end }), true)
	page:choice("size", { label = "Bar size", choices = { { id = "small", label = "Small" }, { id = "large", label = "Large" } }, default = "large" })
	page:number("scale", { label = "Scale", min = 0.5, max = 2, step = 0.5, default = 1 })
	E.eq(page:set("scale", 1.5), true)
	E.eq(page:set("scale", 9), false, "out of range")
	core.current = own
	e:frames(1)
	E.eq(rawget(_G, "obsidian").storage.get("mod_settings/dev.hud/scale"), 1.5, "a mod's own change is kept from the frame")

	local main = setmetatable({}, { __index = Main })
	main:on_enter()
	E.eq(main.options[3].name, "obsidian_mod_settings", "under the game's Settings")
	main:update()
	E.eq(main.drawn[3], "MODS", "in capitals, as the game's rows")
	E.eq(main.drawn[4], "LOC:menu_exit_to_menu", "the game's own words stay")
	main.selected_option = 3
	main:handle_input({ select = true })
	E.eq(main.chose, nil, "the pause menu didn't act on it")
	main:update()
	local list = main.state_stack[1]
	E.eq(list.title, "MODS")
	E.eq(list.drawn[1], "BETTER HUD")
	main:handle_input({ select = true })
	main:update()
	local screen = main.state_stack[2]
	E.eq(screen.title, "BETTER HUD")
	E.eq(screen.drawn[1], "AMMO COUNTER = LOC:options_yes")
	E.eq(screen.drawn[2], "BAR SIZE = LARGE")
	E.eq(screen.drawn[3], "SCALE = 1.5")
	main:handle_input({ right = true })
	E.eq(page:get("ammo"), false)
	E.eq(changes[1], false)
	E.eq(rawget(_G, "obsidian").storage.get("mod_settings/dev.hud/ammo"), false, "a change in the menu is kept at once")
	main:handle_input({ back = true })
	E.eq(#main.state_stack, 1, "back to the list")
	-- With no rows anywhere, the pause menu has no Mod Settings.
	page.rows = {}
	main:on_enter()
	E.eq(#main.options, 3)
end)

-- A row bound to one of the mod's obsidian.json options starts from the value set in Obsidian, and a
-- change is kept as the pending option Obsidian takes; that pending value wins until it's taken.
-- Words by language follow the game's language ("bp" is pt-BR), in capitals with the accents.
test("mod_settings_rows_bound_to_options_start_from_obsidian_and_keep_changes_for_it", function()
	local api, _, e = start()
	local Main = fake_menus()
	rawget(_G, "require")("lua/network/network_global")
	local core = e:core()
	local own = core.current
	local options = { { id = "pause", type = "toggle", default = true }, { id = "speed", type = "choice", default = "off", choices = { "off", "fast" } } }
	local qol = { id = "dev.qol", mod = { options = options } }
	core.current = qol
	e.loaded["obsidian/loader/settings"].values["dev.qol"] = { speed = "fast" }
	E.eq(api.mod_settings.BINDS, true)
	local page = api.mod_settings.page({ en = "Quality", ["pt-BR"] = "Qualidade" })
	local pause = { option = true, label = { en = "Pause", fr = "Pause", ["pt-BR"] = "Pausa" } }
	E.eq(page:toggle("pause", pause), true, "the option's default")
	E.eq(page:choice("speed", { option = true, label = "Speed", choices = { { id = "off", label = { en = "Off", ["pt-BR"] = "Desligado" } }, { id = "fast", label = { en = "Fast", ["pt-BR"] = "Rápido" } } } }), "fast", "the value set in Obsidian")
	E.eq(page:set("pause", false), true)
	core.current = own
	e:frames(1)
	local storage = rawget(_G, "obsidian").storage
	E.eq(storage.get("option/dev.qol/pause"), false, "kept for Obsidian to take")
	E.eq(storage.get("mod_settings/dev.qol/pause"), nil, "not as an in-game setting")

	local lm = rawget(_G, "LocalizationManager")
	local was = lm.current_language
	lm.current_language = "bp"
	local main = setmetatable({}, { __index = Main })
	main:on_enter()
	main.selected_option = 3
	main:handle_input({ select = true })
	main:update()
	E.eq(main.state_stack[1].drawn[1], "QUALIDADE")
	main:handle_input({ select = true })
	main:update()
	local screen = main.state_stack[2]
	E.eq(screen.drawn[1], "PAUSA = LOC:options_no")
	E.eq(screen.drawn[2], "SPEED = RÁPIDO")
	main:handle_input({ right = true })
	lm.current_language = was
	E.eq(storage.get("option/dev.qol/pause"), true, "a change in the menu is kept at once")
	core.current = qol
	E.eq(page:toggle("pause", pause), true, "made again, the row starts from the pending value")
	core.current = own
end)

-- Obsidian QoL's page: the four options that can change in game, each bound to its option.
test("obsidian_qol_binds_four_of_its_options_to_the_mods_page", function()
	local e = G.start({ "obsidian.api", "obsidian.qol" }, { scope = "client" })
	e:core().current = e:core().mods_by_id["obsidian.api"]
	local page = rawget(_G, "obsidian").import("obsidian.api").mod_settings.pages["obsidian.qol"]
	E.eq(page ~= nil, true, "QoL made its page")
	local ids = {}
	for _, row in ipairs(page.rows) do
		E.eq(row.option, true, row.id)
		ids[#ids + 1] = row.id
	end
	E.eq(table.concat(ids, ","), "expedite_mission_end,solo_pause,aim_cursor,toggle_map")
	E.eq(page.rows[1].kind, "choice")
	E.eq(type(page.rows[2].label), "table", "words by language")
end)

return T
