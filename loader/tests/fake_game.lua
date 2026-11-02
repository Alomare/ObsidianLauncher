-- A small mission of the game, for testing Obsidian API and the trainer. It copies the behaviour
-- that makes mistakes costly: an unknown queued command asserts, a command on a unit this peer
-- doesn't own asserts, and a protected value changed outside set_protected is recorded as
-- tampering, which is what the game's check would catch.
local E = require "fake_engine"
local F = require "fake_ui"

local G = {}

local PROTECTED = {
	hitpoints_real = true, armor_min = true, armor_max = true, rounds = true, num_mags = true, reload_time = true,
	magazine_capacity = true, num_grenades = true, cooldown_timer = true, uses_left = true, wanted_move_speed = true,
	defend_timer = true, enemies_killed = true, enemies_to_kill = true, current_wave = true, extra_lives = true,
}

-- Commands the game's components accept. Anything else is the game's assert("Unrecognized message").
local KNOWN = {}
for component, list in pairs({
	health = "reset do_heal set_hitpoints armor set_invincible set_ignore_death_plane set_health_decay do_damage",
	status = "remove_all remove_status remove_statuses_by_element add_immunity_status add_immunity_element",
	shield = "set_shieldpoints set_shield_activated set_shield_info reset_shield_info",
	character = "infinite_stamina move_speed_sprint_max do_knockdown drop_carry exit_interact_objective set_throw_range_multiplier",
	mover = "position position_teleport move_speed acceleration deceleration ignore_slow keep_inside_screen destroy enable",
	inventory = "ammo_pickup set_grenades set_grenade_path set_stratagem_info broadcast_stratagem_cooldown set_stratagem_cooldown_multiplier receive_stratagem set_weapon set_perk set_item_upgrades set_backpack",
	customization = "set_customization communication_event enable_heavy_armor disable_heavy_armor set_visible set_silhouette_color",
	enemy_target = "enable disable",
	enemy = "remove_from_group damage_taken",
	research_sample_pickup = "interact",
	show_on_map = "show",
	stratagem = "set_thrown_direction",
	exosuit_inventory = "set_full_ammo",
	equipment = "wielder wield holster unwield",
	drone_station = "set_uses set_drone ammo_pickup backpack_used drone_reload set_force_dock",
}) do
	KNOWN[component] = {}
	for word in list:gmatch("%S+") do KNOWN[component][word] = true end
end

local function guarded(game, real)
	return setmetatable({}, {
		__index = real,
		__newindex = function(_, k, v)
			if PROTECTED[k] and not game.in_setter then game.tampered[#game.tampered + 1] = k end
			real[k] = v
		end,
		__len = function() return #real end,
	})
end

local function unit(game, parts)
	local u = { alive = true, master = {}, slave = {}, owned = parts.owned ~= false, pos = parts.pos or { x = 0, y = 0, z = 0 }, path = parts.path }
	for name, state in pairs(parts.components or {}) do
		u.master[name] = guarded(game, state)
		u.slave[name] = guarded(game, state)
	end
	return u
end

local function component(game, name, plugin)
	return {
		name = name,
		set_protected = function(_, state, key, value)
			game.in_setter = true
			state[key] = value
			game.in_setter = false
			game.protected[#game.protected + 1] = key
		end,
		setup_console_plugin = plugin and function() return plugin end or nil,
	}
end

-- Starts the engine with the bundled mods given, then a mission. opts: lobby, state, host, dlc.
function G.start(ids, opts)
	opts = opts or {}
	local mods, deploy_mods = {}, {}
	for _, id in ipairs(ids) do
		local manifest = E.read("bundled/" .. id .. "/obsidian.json")
		mods["obsidian/mods/" .. id .. "/main"] = E.read("bundled/" .. id .. "/main.lua")
		local list = manifest:match('"modules":%s*%[(.-)%]') or ""
		for path in list:gmatch('"([^"]+)%.lua"') do
			mods["obsidian/mods/" .. id .. "/" .. path] = assert(E.read("bundled/" .. id .. "/" .. path .. ".lua"), path)
		end
		local texts = {}
		for _, lang in ipairs({ "en", "fr", "es", "pt-BR" }) do
			local raw = E.read("bundled/" .. id .. "/locales/" .. lang .. ".json")
			if raw then
				texts[lang] = {}
				for k, v in raw:gmatch('"([^"]+)": "(.-)",?\r?\n') do texts[lang][k] = v end
			end
		end
		deploy_mods[#deploy_mods + 1] = E.mod(id, { scope = opts.scope or "session", texts = texts,
			requires = id == "obsidian.trainer" and { "obsidian.api" } or nil,
			options = { { id = "keep_edits", type = "toggle", default = false } },
			keybinds = { { id = "open", default = "f6" } } })
	end
	local e = E.new({ deploy = E.deploy({ mods = deploy_mods, has_session = opts.scope ~= "client", texts = { en = {
		menu_title = "Obsidian", menu_empty = "No pages", menu_failed = "Page failed", menu_unavailable = "Not now",
		menu_filter = "Filter", menu_keys = "Keys",
	} } }), mods = mods, lobby = opts.lobby })

	-- string and table helpers the game adds, then the rest of the game.
	local game = { commands = {}, tampered = {}, protected = {}, sent = {}, rpc = {}, spawned = {}, saved = 0 }
	e.game = game
	rawset(string, "split_command", nil)

	e:boot()
	e:start_frames()
	local ui = F.install(e, { state = opts.state or "game" })
	game.ui = ui

	rawset(_G, "Unit", {
		alive = function(u) return u ~= nil and u.alive ~= false end,
		world_position = function(u) return u.pos end,
		local_rotation = function() return "rotation" end,
		mover = function(u) return u.master.mover ~= nil end,
		mover_fits_at = function() return true end,
		get_data = function(u, key) return key == "unit_path" and u.path or nil end,
	})
	local v3 = setmetatable({
		normalize = function(v) return v end,
		distance = function(a, b) return math.sqrt((a.x - b.x) ^ 2 + (a.y - b.y) ^ 2) end,
		forward = function() return { x = 0, y = 1, z = 0 } end,
	}, { __call = function(_, x, y, z) return setmetatable({ x = x, y = y, z = z }, G.vector) end })
	rawset(_G, "Vector3", v3)
	-- A turn about z, enough for the map sums.
	rawset(_G, "Quaternion", setmetatable({ forward = function() return v3(0, 1, 0) end, identity = function() return "identity" end,
		rotate = function(angle, v) return v3(v.x * math.cos(angle) - v.y * math.sin(angle), v.x * math.sin(angle) + v.y * math.cos(angle), v.z) end,
	}, { __call = function(_, axis, angle) return axis.z < 0 and -angle or angle end }))
	rawset(_G, "Vector2Aux", { unbox = function(t) return t end })
	rawset(_G, "Vector3Aux", {
		box = function(t, v) t.x, t.y, t.z = v.x, v.y, v.z return t end,
		unbox = function(t) return v3(t.x, t.y, t.z) end,
	})
	rawset(_G, "Camera", { world_position = function() return v3(0, 0, 20) end })
	rawset(_G, "PhysicsWorld", {
		immediate_raycast = function() return true, v3(5, 5, 0) end,
		set_gravity = function(_, g) game.gravity = g end,
	})
	game.buses = {}
	rawset(_G, "Timpani", { set_bus_volume = function(bus, v)
		game.bus = { bus, v }
		game.buses[bus] = v
	end })
	-- The sounds this mission has, and what it was asked to play.
	game.played = {}
	rawset(_G, "TimpaniWorld", {
		has_event = function(world, name) return world == "sound world" and (name == "ui_select" or name == "music_objective_found") end,
		trigger_event = function(_, name, pos) game.played[#game.played + 1] = { name = name, pos = pos } end,
	})
	rawset(_G, "Network", { peer_id = function() return "me" end, ping = function(peer)
		if peer == "lost" then error("no connection") end
		return 0.042
	end })
	rawset(_G, "NetworkGlobal", rawget(_G, "NetworkGlobal") or { lobby = { is_host = function() return opts.host ~= false end,
		lobby = {}, state = function() return "joined" end, members = function() return { "me" } end, member_data = function() end, set_member_data = function() end, my_id = "me" } })
	local lobby = rawget(_G, "NetworkGlobal").lobby
	lobby.is_host = function() return opts.host ~= false end
	lobby.lobby_host = function() return opts.host ~= false and "me" or "friend" end

	local dlc_items = opts.dlc or { weapon_dlc = true, stratagem_dlc = true, cus_helmet_dlc = true, stratagem_pack_dlc = true }
	rawset(_G, "DlcSettings", {
		is_item_dlc = function(name) return dlc_items[name] == true end,
		can_use = function() return false end,
	})
	rawset(_G, "LocalizationManager", { current_language = opts.language or "en", string = function(_, key)
		if key == "weapon_rifle" then return "Rifle" end
		return "LOC:" .. key
	end })
	rawset(_G, "GameSettings", { get_main_pad_number = function() return 0 end, MAX_NUM_ALLOWED_ENEMIES = 16, MAX_NUM_RESEARCH_SAMPLES = 3168,
		player_colors = { { x = 255, y = 10, z = 20, w = 30 } } })
	rawset(_G, "CameraManager", { change_settings = function(_, ...) game.camera = { ... } end, set_ignore_level_bounds = function() end,
		add_shake = function() game.shakes = (game.shakes or 0) + 1 end, targets = {}, fov = math.rad(60), state = "follow",
		set_fov = function(self, v) self.fov = v end, set_state = function(self, s) self.state = s end,
		add_target = function(self, u) self.targets[u] = {} end, remove_target = function(self, u) self.targets[u] = nil end,
		set_position = function(self, p) self.at = p end, set_rotation = function(self, r) self.turn = r end })
	rawset(_G, "QuaternionAux", { box_elements = function(t, a, r, g, b) t.x, t.y, t.z, t.w = a, r, g, b return t end })
	rawset(_G, "UserManager", { get_user_id = function() return "user" end, get_name = function() return "Tester" end,
		unlock_trophy = function(_, name) game.trophy = name end })

	local save = { user = { player_xp = 1000, research_samples = 50, item_unlocks = { "owned_item" }, new_unlocks = {}, item_upgrades = { a = 1 },
		loadout = { perk = "perk_cardio", primary_weapon = "weapon_rifle", sidearm_weapon = "weapon_pistol", stratagems = { "a", "b", "c", "d", "e", "f", "g", "h" } },
		loadouts = { { perk = "perk_cardio", primary_weapon = "weapon_rifle" }, { perk = "other" } } }, common = {} }
	game.save = save
	rawset(_G, "SaveManager", {
		get = function(_, k) return save.common[k] end,
		set = function(_, k, v) save.common[k] = v end,
		user_get = function(_, k) return save.user[k] end,
		user_set = function(_, k, v) save.user[k] = v end,
		save = function() game.saved = game.saved + 1 end,
		user_save = function() game.saved = game.saved + 1 end,
	})
	rawset(_G, "SaveDataHelper", { save_all = function() game.saved = game.saved + 1 end })
	rawset(_G, "PlanetRewardSettings", { planet_rewards = { bugs = { xp_boost = { { name = "boost" } }, weapon = { { name = "reward_item" }, { name = "weapon_dlc" } } } } })
	game.starting_items = {}
	rawset(_G, "ProgressionSettings", { ranks = { { required_xp = 0, unlocks = { weapon = { "rank_item" }, starting = game.starting_items } }, { required_xp = 5000, unlocks = { weapon = { "late_item" } } } },
		get_rank = function() return { name = "rank_private" } end })
	rawset(_G, "CustomizationSettings", { helmet = { { name = "cus_helmet_a" }, { name = "cus_helmet_dlc" } }, body = { { name = "cus_body_a" } }, cape = { { name = "cus_cape_tutorial" } }, gender = { { name = "x" } } })
	rawset(_G, "LoadoutSettings", {
		primary_weapon = { { name = "weapon_rifle", unit_path = "equipment/rifle", upgrades = { {}, {} } }, { name = "weapon_dlc", unit_path = "equipment/dlc", upgrades = { {} } } },
		default_loadout = { perk = "perk_lasersight", primary_weapon = "weapon_rifle", sidearm_weapon = "weapon_pistol", stratagems = { "a", "b", "c", "d" } },
		static_stratagems = { "stratagem_reinforce", "b", "c", "d" }, nr_of_stratagem_slots = 8,
		get_max_upgrade_value = function(item, category) return category == "stratagems" and #item.upgrades or 2 ^ #item.upgrades - 1 end,
		sidearm_weapon = { { name = "weapon_pistol", unit_path = "equipment/pistol" } },
		perks = { { name = "perk_cardio" }, { name = "perk_lasersight" } },
		stratagems = {
			{ id = 1, name = "stratagem_resupply", cooldown_time = 120, activation_time = 5, stratagem_subtypes = {},
				upgrades = { { cost = 1, cooldown_time = 90 }, { cost = 2 }, { cost = 3, cooldown_time = 60 } } },
			{ id = 2, name = "stratagem_machinegun", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "support_weapon", "weapon" }, unit = "gameobjects/crate" },
			{ id = 3, name = "stratagem_dlc", cooldown_time = 0, activation_time = 3 },
			{ id = 4, name = "stratagem_reinforce", cooldown_time = 0, activation_time = 3 },
			{ id = 5, name = "stratagem_mortar", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "weapon", "anti_tank" }, unit = "gameobjects/crate_mortar" },
			{ id = 6, name = "stratagem_jumppack", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "support" }, unit = "gameobjects/crate_pack" },
			{ id = 7, name = "stratagem_exosuit", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "vehicle" }, unit = "gameobjects/mech" },
			{ id = 8, name = "stratagem_mannable_turret", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "turret" }, unit = "gameobjects/mannable" },
			{ id = 9, name = "stratagem_airstrike", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "strike" }, unit = "gameobjects/airstrike_01" },
			{ id = 10, name = "stratagem_turret", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "turret" }, unit = "gameobjects/sentry" },
			{ id = 11, name = "stratagem_minedrop", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "mine" }, unit = "gameobjects/minedrop_01" },
			{ id = 12, name = "stratagem_pack_dlc", cooldown_time = 0, activation_time = 3, stratagem_subtypes = { "support" }, unit = "gameobjects/crate_pack_dlc" },
		},
		find_stratagem = function() return "from the arrows" end,
		upgrade_all = function(t) t.all = 7 end,
		get_upgraded_settings = function(u) return u.settings end,
	})
	local locked = {}
	for _, name in ipairs(opts.locked or {}) do locked[name] = true end
	local function own(list)
		for _, item in ipairs(list or {}) do
			if item.name and not locked[item.name] then game.starting_items[#game.starting_items + 1] = item.name end
		end
	end
	local loadouts, looks = rawget(_G, "LoadoutSettings"), rawget(_G, "CustomizationSettings")
	own(loadouts.primary_weapon)
	own(loadouts.sidearm_weapon)
	own(loadouts.perks)
	own(loadouts.stratagems)
	for _, list in pairs(looks) do
		if type(list) == "table" then own(list) end
	end
	game.perk_calls = {}
	local function perk_call(name) return function() game.perk_calls[#game.perk_calls + 1] = name end end
	rawset(_G, "PerkSettings", {
		perk_cardio = { setup_master = perk_call("cardio setup_master"), remove_master = perk_call("cardio remove_master") },
		perk_lasersight = { setup_master = perk_call("laser setup_master"), setup_slave = perk_call("laser setup_slave"), remove_slave = perk_call("laser remove_slave") },
	})
	rawset(_G, "StatusTemplateSettings", { heavy_burn = { power = 5, duration = 3, element = "fire" }, flashbang_stun = { duration = 3, status_name = "stun" }, flashbang_slow = { amount = 0.5, duration = 3, status_name = "slow" } })
	rawset(_G, "EnemyGroupSettings", { bug_scout_x2 = { units = { { unit_path = "characters/bug_scout/bug_scout", amount = 2 } } },
		cyborg_x1 = { units = { { unit_path = "characters/cyborg_soldier/cyborg_soldier", amount = 1 } } } })
	rawset(_G, "EncounterSettings", { bugs = {
		{ name = "bug_wave_scouts", cost = 2, events = { { delay = 0, groups = { { delay = 0, group = "bug_scout_x2" } } }, { delay = 3, groups = { { delay = 0, group = "bug_scout_x2" } } } } },
		{ name = "bug_wave_strange", cost = 4, events = { { delay = 0, groups = { { delay = 0, group = "cyborg_x1" } } } } },
	} })
	rawset(_G, "HintSettings", { hint_alarm = { audio = "ui_select", duration = 7, icon = "icon_hint", text = "hint_alarm" } })
	rawset(_G, "SpatialSpawnUtlity", { generate_points_in_donut = function(at) return { at } end })
	-- The game's damage sums, called as functions on the victim, with the formula's shape.
	rawset(_G, "HealthComponent", { calculate_damage = function(u, power)
		local state = u.slave.health
		return power, 1, state.hitpoints - power <= state.constitution
	end })
	rawset(_G, "ShieldComponent", { calculate_damage = function(u, power)
		return power, u.slave.shield.shieldpoints - power <= 0
	end })
	rawset(_G, "StratagemsDisabledAreaComponent", { is_unit_inside_any_area = function() return true end })

	-- Settings files: one weapon with an upgrade clone, the way the game builds them.
	local rifle = { weapon_info = { power = 75, firerate = 650, magazine_capacity = 30, magazines = 6, magazines_max = 12, reload_duration = 2.5, reload_duration_fast = 1.25,
		attributes = {} }, melee_info = { power = 50, radius = 1, center_distance = 1, duration = 0.5 },
		upgrade_info = { type = "bitmask", { weapon_info = { magazine_capacity = 45 } } } }
	local rifle_upgraded = { weapon_info = { power = 75, firerate = 650, magazine_capacity = 45, magazines = 6, magazines_max = 12, reload_duration = 2.5, reload_duration_fast = 1.25,
		attributes = {} }, melee_info = { power = 50, radius = 1, center_distance = 1, duration = 0.5 },
		upgrade_info = { type = "bitmask", { weapon_info = { magazine_capacity = 45 } } } }
	game.rifle, game.rifle_upgraded = rifle, rifle_upgraded
	local helldiver = { character_info = { move_speed_jog = 4, move_speed_sprint_max = 7, sprint_stamina = 5, dive_duration = 0.8, get_up_presses_needed = 30 }, health_info = { health = 100, armor_max = 35 } }
	local scout = { enemy_info = { movement_speed = 4, threat_calculation = function() return 1 end, aggro_range = 80 }, health_info = { health = 30, armor_min = 15, armor_max = 25 } }
	local crate = { supply_crate_info = { content = { { unit_path = "equipment/mg_pickup" } } } }
	local mortar_crate = { supply_crate_info = { content = { { unit_path = "equipment/mortar_pickup" } } } }
	local pack = { backpack_info = { num_uses = 60, num_uses_max = 60 }, pickup_info = { backpack_path = "equipment/pack", item_name = "pickup_jumppack", pickup_path = "equipment/pack_pickup" } }
	local pack_better = { backpack_info = { num_uses = 90, num_uses_max = 90 }, pickup_info = pack.pickup_info }
	game.pack = pack
	local pack_crate = { supply_crate_info = { content = { { unit_path = "equipment/pack_pickup" }, { unit_path = "equipment/pack_pickup" } } } }
	local dlc_pack = { backpack_info = { num_uses = 1 }, pickup_info = { backpack_path = "equipment/pack_dlc", item_name = "pickup_dlc" } }
	local dlc_crate = { supply_crate_info = { content = { { unit_path = "equipment/pack_dlc_pickup" } } } }
	local mech = { vehicle_info = { vehicle_type = "exosuit", move_speed = 4, turn_speed = 0.5, acceleration = 9, deceleration = 9 }, health_info = { health = 1000, armor_min = 0, armor_max = 1000 },
		weapon_info = { power = 75, magazine_capacity = 1000, magazines = 0, magazines_max = 0, reload_duration = 2.5, reload_duration_fast = 1.25 }, melee_info = { power = 350, radius = 1.25, center_distance = 2.25, duration = 0.8 } }
	game.mech_settings = mech
	local mannable = { vehicle_info = { vehicle_type = "mannable_turret", move_speed = 0, turn_speed = 0.6 }, health_info = { health = 1000 } }
	local sentry = { turret_info = { rounds = 100, check_for_enemies_radius = 20, turn_speed_target = 0.1 }, weapon_info = { power = 50 }, ttl_info = { time_until_dead = 60 }, health_info = { health = 100 } }
	local airstrike = { airstrike_info = { num_bombs = 5, bomb_area_length = 20, time_until_bombs = 2 }, explosive_info = { center_power = 500, blast_power = 100, center_radius = 2, blast_radius = 5, force = 10 } }
	local minedrop = { minedrop_info = { num_mines = 10 } }
	local mine = { mine_info = { time_until_armed = 1 } }
	local grenade = { explosive_info = { time_until_explosion = 3, center_power = 300, blast_power = 100, center_radius = 2, blast_radius = 5, force = 10 }, throwable_info = { throw_distance_max = 20 } }
	local by_settings = {
		["equipment/rifle"] = rifle, ["characters/helldiver_private_01/helldiver_private_01"] = helldiver,
		["characters/bug_scout/bug_scout"] = scout, ["gameobjects/crate"] = crate, ["equipment/mg"] = { weapon_info = { power = 40, magazine_capacity = 100, magazines = 3, magazines_max = 6, reload_duration = 5, reload_duration_fast = 4 } },
		["gameobjects/crate_mortar"] = mortar_crate, ["equipment/mortar"] = { weapon_info = { power = 300, magazine_capacity = 6, magazines = 2, magazines_max = 3, reload_duration = 3, reload_duration_fast = 2 } },
		["gameobjects/crate_pack"] = pack_crate, ["equipment/pack"] = pack, ["gameobjects/crate_pack_dlc"] = dlc_crate, ["equipment/pack_dlc"] = dlc_pack,
		["gameobjects/mech"] = mech, ["gameobjects/mannable"] = mannable, ["gameobjects/sentry"] = sentry, ["gameobjects/airstrike_01"] = airstrike,
		["gameobjects/minedrop_01"] = minedrop, ["gameobjects/mine_01"] = mine, ["gameobjects/mine_stun"] = mine,
		["equipment/equipment_grenade_01"] = grenade, ["equipment/equipment_grenade_incendiary"] = grenade, ["equipment/equipment_grenade_stun"] = grenade, ["equipment/equipment_grenade_smoke"] = grenade,
	}
	local unit_to_settings = { ["equipment/rifle"] = "equipment/rifle", ["equipment/mg_pickup"] = "equipment/mg", ["gameobjects/crate"] = "gameobjects/crate",
		["gameobjects/crate_mortar"] = "gameobjects/crate_mortar", ["equipment/mortar_pickup"] = "equipment/mortar",
		["gameobjects/crate_pack"] = "gameobjects/crate_pack", ["equipment/pack_pickup"] = "equipment/pack", ["equipment/pack"] = "equipment/pack",
		["gameobjects/crate_pack_dlc"] = "gameobjects/crate_pack_dlc", ["equipment/pack_dlc_pickup"] = "equipment/pack_dlc",
		["gameobjects/mech"] = "gameobjects/mech", ["gameobjects/mannable"] = "gameobjects/mannable", ["gameobjects/sentry"] = "gameobjects/sentry",
		["gameobjects/airstrike_01"] = "gameobjects/airstrike_01", ["gameobjects/minedrop_01"] = "gameobjects/minedrop_01",
		["gameobjects/mine_01"] = "gameobjects/mine_01", ["gameobjects/mine_stun"] = "gameobjects/mine_stun",
		["equipment/equipment_grenade_01"] = "equipment/equipment_grenade_01", ["equipment/equipment_grenade_incendiary"] = "equipment/equipment_grenade_incendiary",
		["equipment/equipment_grenade_stun"] = "equipment/equipment_grenade_stun", ["equipment/equipment_grenade_smoke"] = "equipment/equipment_grenade_smoke",
		["characters/helldiver_private_01/helldiver_private_01"] = "characters/helldiver_private_01/helldiver_private_01", ["characters/bug_scout/bug_scout"] = "characters/bug_scout/bug_scout" }
	rawset(_G, "LuaSettingsManager", {
		unit_path_to_modified_settings = { ["equipment/rifle"] = { [1] = rifle_upgraded } },
		get_settings_by_settings_path = function(_, p) return by_settings[p] end,
		get_settings_by_unit_path = function(_, p) return by_settings[unit_to_settings[p]] end,
		get_settings_path_by_unit_path = function(_, p) return unit_to_settings[p] end,
		get_settings_by_unit = function() return helldiver end,
		get_modified_settings = function(self, path, value)
			if path == "equipment/pack" and value == 1 then return pack_better end
			return self:get_settings_by_unit_path(path)
		end,
		unit_path_to_settings_path_iterator = function() return pairs(unit_to_settings) end,
		all_settings_iterator = function() return pairs(by_settings) end,
	})
	local function clone(t)
		local out = {}
		for k, v in pairs(t) do out[k] = type(v) == "table" and clone(v) or v end
		return out
	end
	rawset(table, "clone", clone)
	local function override(t, o)
		for k, v in pairs(o) do
			if type(v) == "table" and type(t[k]) == "table" then override(t[k], v) else t[k] = v end
		end
	end
	rawset(_G, "SettingsAux", { recursive_override = override })

	-- The mission.
	local inventory = { stratagems = {
		guarded(game, { id = 1, cooldown_timer = 30, uses_left = 1, loadout_info = { num_uses = 4 } }),
	}, num_grenades = 1, perk = 1, perk_context = {}, item_upgrades = { weapon_rifle = 5, stratagem_machinegun = 2 } }
	local weapon = unit(game, { components = { firearm = { rounds = 3, num_mags = 2, is_reloading = true, reload_time = 2 } } })
	inventory.current_weapon, inventory.primary_weapon = weapon, weapon
	local me = unit(game, { pos = { x = 1, y = 1, z = 0 }, components = {
		health = { hitpoints = 100, constitution = 0, is_downed = false, is_dead = false, hitpoints_real = 100, armor_min = 15, armor_max = 35 },
		status = { immunities_elements = {}, immunities_statuses = {} }, shield = { shieldpoints = 50 },
		character = { sprint_stamina = 0, input = {}, move_speed_sprint_max = 7, throw_range_multiplier = 1 }, mover = { position = { x = 1, y = 1, z = 0 }, acceleration = 20, deceleration = 30 },
		inventory = inventory, customization = { visible = true, heavy_armor = 1 }, enemy_target = {},
	} })
	game.me = me
	game.weapon = weapon
	local function enemy(owned, x)
		return unit(game, { owned = owned, pos = { x = x, y = 0, z = 0 }, path = "characters/bug_scout/bug_scout", components = { enemy = {},
			health = { is_dead = false, constitution = -10, hitpoints = 30, max_hitpoints = 30 }, shield = { shieldpoints = 20 } } })
	end
	game.enemies = { enemy(true, 3), enemy(false, 100) }

	-- An exosuit with a gun that's a unit of its own, and a seat the avatar isn't in yet.
	local arm = unit(game, { components = { firearm = {} } })
	game.arm = arm
	game.mech = unit(game, { components = {
		vehicle = { vehicle_info = mech.vehicle_info, rotation_speed = 0.5 }, mover = {},
		health = { hitpoints = 1000, constitution = 500, invincible = false, armor_min = 0, armor_max = 1000 },
		seat = { seat_weapon_units = { arm } }, exosuit_inventory = { num_grenades = 3 },
	} })
	game.mech.settings = mech
	game.apc = unit(game, { components = {
		vehicle = { vehicle_info = { vehicle_type = "apc", move_speed = 5, turn_speed = 55 }, rotation_speed = 55 }, mover = {},
		health = { hitpoints = 1000, constitution = 500 }, seat = { seat_weapon_units = {} },
	} })
	game.apc.settings = { health_info = { armor_min = 20, armor_max = 2000 } }

	game.convoy = unit(game, { components = { health = { invincible = false, hitpoints = 500, constitution = 0 } } })
	game.escorted = unit(game, { components = { health = { invincible = false, hitpoints = 100, constitution = 0 } } })

	game.slave_commands = {}
	local comps = { health = component(game, "health"), status = component(game, "status"), inventory = component(game, "inventory"),
		firearm = component(game, "firearm"), stratagem = component(game, "stratagem"), launcher = component(game, "launcher"),
		flameweapon = component(game, "flameweapon"), beamweapon = component(game, "beamweapon"), throwable_weapon = component(game, "throwable_weapon") }
	comps.stratagem.spawn_stratagem_hellpod = function(_, ...) game.pods = { ... } end
	comps.stratagem.trigger_rpc_event = function() end
	comps.inventory.setup_console_plugin = function()
		return { give_stratagem = function(args) game.given = args[2] end }
	end
	game.objective_runs = {}
	for _, name in ipairs({ "objective_destroy_object", "objective_interact_object" }) do
		comps[name] = component(game, name, {
			complete_objectives = function() game.objective_runs[#game.objective_runs + 1] = name end,
			restart_objectives = function() game.objective_runs[#game.objective_runs + 1] = "restart " .. name end,
		})
	end
	comps.objective_destroy_object.setup_console_plugin = function()
		return { complete_objectives = function() error("one objective failed") end }
	end
	game.components = comps

	local masters = { enemy = {}, objective_destroy_object = { { state = {} } }, objective_interact_object = { { state = {} } },
		objective_convoy = { [game.convoy] = { state = { mission_state = "pending", enabled = false } } },
		npc_escort = { [game.escorted] = { state = { in_goal = false } } } }
	local slaves = { enemy = {}, objective_extract_lz = {} }
	for _, u in ipairs(game.enemies) do
		slaves.enemy[u] = { state = u.slave.health }
		if u.owned then masters.enemy[u] = { state = u.master.health } end
	end
	game.masters = masters

	rawset(_G, "EntityAux", {
		get_component = function(name) return comps[name] end,
		has_component_master = function(u, c) return u.master[c] ~= nil end,
		has_component = function(u, c) return u.master[c] ~= nil end,
		has_interface = function(u, i) return i == "weapon" and u.master.firearm ~= nil end,
		state_master = function(u, c) return u.master[c] end,
		state_master_interface = function() return nil end,
		state = function(u, c) return u.slave[c] end,
		owned = function(u) return u.owned end,
		owner = function() return "peer2" end,
		queue_command_master = function(u, c, cmd, data)
			assert(u.owned, "command on a unit this peer doesn't own")
			assert(KNOWN[c] and KNOWN[c][cmd], "Unrecognized message " .. tostring(c) .. " " .. tostring(cmd))
			game.commands[#game.commands + 1] = { unit = u, component = c, command = cmd, data = data }
		end,
		queue_command_master_interface = function(u, i, cmd, data)
			game.commands[#game.commands + 1] = { unit = u, interface = i, command = cmd, data = data }
		end,
		-- From any peer: the game sends it on to the owner.
		queue_command_interface = function(u, i, cmd, data)
			game.commands[#game.commands + 1] = { unit = u, interface = i, command = cmd, data = data }
		end,
		queue_command_slave = function(u, c, cmd, data)
			assert(u.slave[c], "no such component on the unit")
			game.slave_commands[#game.slave_commands + 1] = { unit = u, component = c, command = cmd, data = data }
		end,
	})

	local player_go = { avatar_unit = me, research_samples = 0, name = "Tester", map_ping = { -1000, -1000 } }
	function player_go:owned() return true end
	function player_go:owner() return "me" end
	function player_go:exists() return true end
	function player_go:get(k) return self[k] end
	function player_go:set(k, v) self[k] = v end
	function player_go:has(k) return self[k] ~= nil end
	local record = { player_go = player_go, pad_number = 0, extra_lives = 1 }
	game.record = record
	local mission_manager = { get_objectives = function()
		return { { name = "objective_a", type = "objective_interact_object", group = 1, state = "pending", params = { 1, 3 }, position = { { x = 9, y = 9, z = 0 } } },
			{ name = "mission_objective_extract", state = "pending", position = { { x = 7, y = 7, z = 0 } } } }
	end, set_mission_completed = function() game.completed = true end, set_extraction_done = function() game.extraction_done = true end,
		set_display_objectives = function(self, on) self.display_objectives = on end }
	local state = {
		players = { record }, mission_manager = mission_manager, mission_seed = 12345, event_delegate = G.delegate(),
		respawn_position = { x = 2, y = 2, z = 0 },
		entity_manager = {
			has_entities = function(_, n) return next(masters[n] or {}) ~= nil or next(slaves[n] or {}) ~= nil end,
			-- As in the game: a component the state never registered has no table to read.
			get_master_entities = function(_, n)
				if not (masters[n] or slaves[n]) then error("entity_manager: attempt to index a nil value") end
				return masters[n] or {}
			end,
			get_slave_entities = function(_, n)
				if not (masters[n] or slaves[n]) then error("entity_manager: attempt to index a nil value") end
				return slaves[n] or {}
			end,
		},
		network_router = { transmit_to = function(_, ...) game.rpc[#game.rpc + 1] = { ... } end },
		encounter_manager = {
			max_num_enemies = 8, mission_difficulty = 5, mission_enemy = "bugs",
			spawn_enemy = function(_, path) game.spawned[#game.spawned + 1] = path end,
			spawn_group = function(_, pos, g, _, cost) game.spawned[#game.spawned + 1] = g.units[1].amount game.group_cost = cost game.spawned_at = pos end,
			set_disabled = function(_, v) game.encounters_off = v end,
			start_encounter = function() game.alarm = true end,
		},
		patrol_manager = { max_num_patrols = 6, patrol_spawn_interval = 8, set_disabled = function(_, v) game.patrols_off = v end, add_patrol_on_spawner = function() game.patrol = true end },
		entity_spawner = { spawn_entity = function(_, p, pos, rotation, _, setup)
			game.spawned[#game.spawned + 1] = p
			game.spawn_setup, game.spawn_pos = setup, pos
			-- The flare has its stratagem component the moment it exists, as in the game.
			if p == "gameobjects/flare_01" then
				local flare = unit(game, { components = { stratagem = {} } })
				game.flare = flare
				return flare
			end
		end, despawn_entity = function() end },
		component_manager = { component_name_to_index = { navmesh_mover = 1, enemy = 2 }, active_components = { 1, 2 },
			pause_component = function(_, n) game.paused_component = n end, resume_component = function() end },
		explosion_manager = { do_explosion = function() game.boom = true end },
		viewport = { set_shading_environment_variable = function(_, n, kind, value)
				game.shading = n
				game.shadings = game.shadings or {}
				game.shadings[n] = { kind = kind, value = value }
			end, remove_shading_environment_variable = function(_, n)
				if game.shadings then game.shadings[n] = nil end
			end,
			set_shading_environment_settings = function(_, name) game.shading_setting = name end, get_camera = function() return "camera" end,
			screen_to_world = function() return rawget(_G, "Vector3")(1, 1, -1) end },
		world_proxy = { get_physics_world = function() return "physics" end, get_world = function() return "world" end, get_timpani_world = function() return "sound world" end },
		hud = {},
		audio_state_machine = { bus_volumes = { combat_enemy = -4 }, play_stinger = function(_, name) game.stinger = name end },
		environment_area_manager = { set_shading_environment_event = function(_, event) game.flash = event end },
		hint_manager = { hints_on = opts.hints ~= false, set_active = function(self, on) self.active = on end, add_hint = function(self, name, override, params, _, seconds)
			if not (self.hints_on or override) then return end
			local hint = rawget(_G, "HintSettings")[name]
			local text = "text of " .. hint.text
			-- As LocalizationManager.string_format goes through its pairs.
			if params then text = string.gsub(text, params[1], params[2]) end
			game.hint = { name = name, text = text, seconds = seconds or hint.duration }
		end },
		generated_level = { shading_environment_setting = "dusk", environment_name = "desert", enemy = "bugs", width = 8, height = 8, map_size = { x = 10, y = 10 }, get_closest_position_on_level = function() return rawget(_G, "Vector3")(0, 0, 0) end },
		set_extra_lives = function(_, _, n) game.lives = n end,
		do_abort_mission = function() game.aborted = true end,
		spawn_research_samples_on_all_markers = function() game.samples = true end,
		get_active_players = function() return { 1 } end,
		state_machine = { change_state = function(_, name, params) game.changed = { name, params } end },
	}
	game.state = state
	ui.state_object = state
	game.packages = { ["resource_packages/race_bugs"] = true }
	local boot = rawget(_G, "Boot")
	boot.package_manager = { is_package_online = function(_, name) return game.packages[name] == true end,
		load_package = function(_, name, flush) game.packages[name], game.flushed = true, flush end,
		unload_package = function(_, name) game.packages[name] = nil end }
	boot.update_volume = function() game.volumes_set = (game.volumes_set or 0) + 1 end
	-- As the real one: unknown or paid items go back to the default, and it hands back what it got.
	boot.ensure_loadout = function(_, t)
		if t.primary_weapon == "weapon_dlc" then t.primary_weapon = "weapon_rifle" end
		return t
	end
	rawset(_G, "DifficultySettings", { get_lookup_mission_params = function() return { difficulty = 5, enemy = "bugs" } end,
		modulate_seed = function(...) game.seed_parts = { ... } return 777 end })

	return e, game, ui
end

-- The event delegate: no pcall, and registering twice is an assert.
function G.delegate()
	local d = { events = {} }
	function d:register(listener, ...)
		for i = 1, select("#", ...) do
			local name = select(i, ...)
			assert(listener[name], "Callback object does not implement function")
			self.events[name] = self.events[name] or {}
			assert(not self.events[name][listener], "Callback object is already registered")
			self.events[name][listener] = true
		end
	end
	function d:trigger(name, ...)
		for listener in pairs(self.events[name] or {}) do listener[name](listener, ...) end
	end
	return d
end

G.vector = {
	__add = function(a, b) return setmetatable({ x = a.x + b.x, y = a.y + b.y, z = a.z + b.z }, G.vector) end,
	__sub = function(a, b) return setmetatable({ x = a.x - b.x, y = a.y - b.y, z = a.z - b.z }, G.vector) end,
	__mul = function(a, n) return setmetatable({ x = a.x * n, y = a.y * n, z = a.z * n }, G.vector) end,
}

function G.command(game, component, command)
	for _, c in ipairs(game.commands) do
		if c.component == component and c.command == command then return c end
	end
	return nil
end

return G
