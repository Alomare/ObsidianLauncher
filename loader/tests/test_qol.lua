-- The bundled Obsidian Patches mod (id obsidian.qol), run from bundled/obsidian.qol/main.lua as a client mod.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local IDS = { "skip_intro", "skip_login", "skip_war_briefs", "skip_planet_scanning", "solo_pause", "widescreen", "aim_cursor", "toggle_map", "recoil_fix", "dive_sprint_fix" }

-- off: option ids switched off in settings.txt; every other option keeps the default (on).
-- expedite: the mission end choice, off when not given.
local function engine(off, expedite)
	local options, lines = {}, {}
	for _, id in ipairs(IDS) do options[#options + 1] = { id = id, type = "toggle", default = true } end
	options[#options + 1] = { id = "expedite_mission_end", type = "choice", default = "off", choices = { "off", "faster", "instant" } }
	for _, id in ipairs(off or {}) do lines[#lines + 1] = "option\tobsidian.qol\t" .. id .. "\ttoggle\t0\n" end
	if expedite then lines[#lines + 1] = "option\tobsidian.qol\texpedite_mission_end\tchoice\t" .. expedite .. "\n" end
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("obsidian.qol", { options = options }) } }),
		mods = { ["obsidian/mods/obsidian.qol/main"] = E.read("bundled/obsidian.qol/main.lua") },
		settings = table.concat(lines),
	})
	e:boot()
	e:start_frames()
	return e
end

local function load_states(e)
	local seen = {}
	rawset(_G, "StateMainMenu", {
		NETWORK_SETUP_LOGIN = 3,
		update = function(self, dt) seen.updates = (seen.updates or 0) + 1 end,
	})
	rawset(_G, "StateMenu", { on_enter = function(self, params) seen.recap = params and params.show_recap end })
	e.opts.modules["lua/states/state_main_menu"] = true
	rawget(_G, "require")("lua/states/state_main_menu")
	return seen
end

local function main_menu(showing, step)
	local sm = { prepared = nil }
	function sm:is_prepared(name) return self.prepared == name end
	function sm:is_preparing() return false end
	function sm:is_any_preparing() return false end
	function sm:unprepare() self.prepared = nil end
	function sm:prepare(name) self.prepared = name end
	return setmetatable({ showing_menu = showing, network_setup_state = step, state_machine = sm }, { __index = rawget(_G, "StateMainMenu") })
end

-- The login click lands once the menu takes input (ready), as a player's click would.
local function clicking_menu(ready)
	rawset(_G, "KEYBOARD_MOUSE_PAD_NUMBER", 7)
	local menu = main_menu(false, 1)
	menu.ready = ready
	function menu:handle_input(input, inputs)
		self.inputs_seen = (self.inputs_seen or 0) + 1
		if self.ready and input.select then
			self.showing_menu = true
			self.main_pad = inputs[1].pad_number
		end
	end
	return menu
end

local function update(menu) rawget(_G, "StateMainMenu").update(menu, 0.016) end

test("skip_intro_sets_skip_splash", function()
	local e = engine()
	E.eq(rawget(_G, "ARGUMENTS").skip_splash, true, "ARGUMENTS")
	rawset(_G, "Boot", { arguments = {} })
	e:frames(1)
	E.eq(rawget(_G, "Boot").arguments.skip_splash, true, "Boot.arguments")
end)

test("skip_login_clicks_then_goes_online_once", function()
	local e = engine()
	local seen = load_states(e)
	local menu = clicking_menu(false)
	update(menu)
	E.eq(menu.showing_menu, false, "logo still showing")
	menu.ready = true
	update(menu)
	E.eq(menu.showing_menu, true, "menu took the click")
	E.eq(menu.main_pad, 7, "keyboard and mouse")
	local count = menu.inputs_seen
	update(menu)
	E.eq(menu.inputs_seen, count, "no more clicks once the menu shows")
	menu.network_setup_state = 3
	update(menu)
	E.eq(seen.updates, 4, "original update once per frame")
	E.eq(menu.wants_exit, true, "wants_exit")
	E.eq(menu.wanted_state, "menu", "wanted_state")
	E.eq(menu.backend_type, "STEAM", "backend_type")
	E.eq(menu.state_machine.prepared, "menu", "prepared state")
	E.eq(rawget(_G, "ARGUMENTS").autohost, nil, "autohost never set")
	local again = main_menu(true, 3)
	update(again)
	E.eq(again.wants_exit, nil, "second visit to the main menu")
	E.has(e:log("obsidianqol"), "skip login: going online to the ship")
end)

test("skip_login_waits_for_the_login_step", function()
	local e = engine()
	load_states(e)
	local early = main_menu(true, 1)
	update(early)
	E.eq(early.wants_exit, nil, "before the login step")
	early.network_setup_state = 3
	update(early)
	E.eq(early.backend_type, "STEAM", "once the login step comes")
end)

test("skip_login_error_is_logged_and_left_to_the_player", function()
	local e = engine()
	local seen = load_states(e)
	local menu = clicking_menu(true)
	function menu:handle_input() error("boom") end
	update(menu)
	update(menu)
	E.eq(seen.updates, 2, "original update once per frame")
	E.eq(E.count(e:log("obsidianqol"), "skip login: click failed"), 1, "logged once")
end)

test("skip_war_briefs_skips_the_recap", function()
	local e = engine()
	local seen = load_states(e)
	rawget(_G, "StateMenu").on_enter({}, { show_recap = true })
	E.eq(seen.recap, false)
end)

-- The galaxy map's recap screen, as the game defines it when the campaign menu loads. params.events
-- gives the screen something to play; params.war_over makes it the war's end; params.from_backend
-- leaves the snapshots to come later.
local function load_recap(e)
	local recap = {}
	function recap.on_enter(self, params)
		self.params, self.wants_exit, self.snapshots = params, false, {}
		if params.war_over then self.season_end_params = { season = 3 } end
		if params.events then self.snapshots = { "event" } elseif not params.from_backend then self.wants_exit = true end
	end
	function recap.recieve_snapshots(self) self.snapshots = { "event" } end
	local campaign = { recap = setmetatable({}, { __index = recap }) }
	function campaign.go_to_recap(self) self.recap:on_enter({ next_state = "region", events = true }) end
	rawset(_G, "MenuScreenCampaignRecap", recap)
	rawset(_G, "MenuScreenCampaign", campaign)
	e.opts.modules["lua/menu/menu_screen_campaign"] = true
	rawget(_G, "require")("lua/menu/menu_screen_campaign")
	return setmetatable({}, { __index = recap }), campaign
end

test("skip_war_briefs_leaves_the_galaxy_maps_brief_at_once", function()
	local e = engine()
	local screen = load_recap(e)
	screen:on_enter({ next_state = "start", events = true })
	E.eq(screen.wants_exit, true, "the brief of what changed")
	E.has(e:log("obsidianqol"), "skip war briefs: brief skipped")
end)

test("skip_war_briefs_plays_the_recap_the_player_asks_for_and_the_wars_end", function()
	local e = engine()
	local screen, campaign = load_recap(e)
	campaign:go_to_recap()
	E.eq(campaign.recap.wants_exit, false, "asked for")
	screen:on_enter({ next_state = "start", events = true })
	E.eq(screen.wants_exit, true, "the next one is skipped again")
	local over = load_recap(e)
	over:on_enter({ next_state = "start", events = true, war_over = true })
	E.eq(over.wants_exit, false, "the war's end goes on to the season end")
end)

test("skip_war_briefs_waits_for_the_snapshots_from_the_backend", function()
	local e = engine()
	local screen = load_recap(e)
	screen:on_enter({ from_backend = true })
	E.eq(screen.wants_exit, false, "nothing to decide yet")
	screen:recieve_snapshots({}, { 1 })
	E.eq(screen.wants_exit, true)
end)

-- A mission alone or with others: the game state's component manager, the lobby and the pause menu.
local function mission(e, members)
	local cm = { component_name_to_index = { navmesh_mover = 1, enemy = 2, hud_thing = 3 }, active_components = { 1, 2, 3 } }
	function cm:pause_component(name)
		local index = self.component_name_to_index[name]
		for i, x in ipairs(self.active_components) do
			if x == index then table.remove(self.active_components, i) return end
		end
	end
	function cm:resume_component(name) table.insert(self.active_components, self.component_name_to_index[name]) end
	local game = { component_manager = cm }
	rawset(_G, "Boot", { state_machine = { get_current_state = function() return game end, get_current_state_name = function() return "game" end } })
	local lobby = { lobby = {}, list = members }
	function lobby:members() return self.list end
	rawset(_G, "NetworkGlobal", { lobby = lobby })
	local sent = {}
	rawset(_G, "EntityAux", { queue_command_master = function(unit, component, command, data) sent[#sent + 1] = data end })
	rawset(_G, "MenuScreenMain", { set_active_pause = function() end })
	e.opts.modules["lua/menu/menu_screen_main"] = true
	rawget(_G, "require")("lua/menu/menu_screen_main")
	local function running(name)
		for _, x in ipairs(cm.active_components) do
			if x == cm.component_name_to_index[name] then return true end
		end
		return false
	end
	return running, lobby, sent, cm
end

test("solo_pause_freezes_a_mission_alone_and_goes_on_when_someone_joins", function()
	local e = engine()
	local running, lobby, sent = mission(e, { "me" })
	local menu = rawget(_G, "MenuScreenMain")
	menu.set_active_pause({}, true)
	E.eq(rawget(_G, "PAUSED"), true, "the scripts' clock")
	E.eq(running("enemy") or running("navmesh_mover"), false, "enemies and movement wait")
	E.eq(running("hud_thing"), true, "the rest runs")
	local data = { damage = 5 }
	rawget(_G, "EntityAux").queue_command_master("unit", "enemy", "damage_taken", data)
	E.eq(sent[1] ~= data and sent[1].damage == 5, true, "a waiting enemy's command keeps a copy")
	lobby.list = { "me", "friend" }
	e:frames(1)
	E.eq(rawget(_G, "PAUSED"), false, "someone joined")
	E.eq(running("enemy") and running("navmesh_mover"), true)
	E.has(e:log("obsidianqol"), "solo pause: someone joined")
end)

test("solo_pause_ends_with_the_menu_and_stays_off_with_company", function()
	local e = engine()
	local running, lobby = mission(e, { "me" })
	local menu = rawget(_G, "MenuScreenMain")
	menu.set_active_pause({}, true)
	menu.set_active_pause({}, false)
	E.eq(rawget(_G, "PAUSED"), false)
	E.eq(running("enemy"), true)
	lobby.list = { "me", "friend" }
	menu.set_active_pause({}, true)
	E.eq(rawget(_G, "PAUSED"), false, "with another player the game goes on, as it always did")
	E.eq(running("enemy"), true)
end)

test("solo_pause_leaves_a_freeze_it_didnt_make", function()
	local e = engine()
	local running, _, _, cm = mission(e, { "me" })
	cm:pause_component("enemy")
	local menu = rawget(_G, "MenuScreenMain")
	menu.set_active_pause({}, true)
	menu.set_active_pause({}, false)
	E.eq(running("enemy"), false, "another mod's freeze stays")
	E.eq(running("navmesh_mover"), true)
end)

test("skip_planet_scanning_shortens_every_step", function()
	local e = engine()
	rawset(_G, "CampaignSettings", { planet_scanning_states = { { name = "wait", time = 1 }, { name = "first_scan", time = 0.75 } } })
	e:frames(1)
	for _, step in ipairs(rawget(_G, "CampaignSettings").planet_scanning_states) do E.eq(step.time, 0.01, step.name) end
	rawset(_G, "CampaignSettings", { planet_scanning_states = { { name = "wait", time = 1 } } })
	e:frames(1)
	E.eq(rawget(_G, "CampaignSettings").planet_scanning_states[1].time, 0.01, "a replaced table")
end)

test("widescreen_lifts_the_aspect_ratio_limit", function()
	rawset(_G, "GameSettings", { max_aspect_ratio = 16 / 9 })
	engine()
	E.eq(rawget(_G, "GameSettings").max_aspect_ratio, 10)
end)

-- A small StateMissionEnd: change_state and update as the game has them, sub-states with step
-- lists like the result screens, and one local and one remote player.
local function mission_end(e)
	local entered = {}
	local function screen(name, steps)
		local s = { name = name }
		function s.on_enter(self)
			entered[#entered + 1] = name
			if steps then
				local list = {}
				for i, t in ipairs(steps) do list[i] = { name = "step" .. i, time = t } end
				self.sub_state_states, self.sub_state_state = list, 1
			end
		end
		function s.update(self, dt) self.seen_dt = dt end
		rawset(_G, name, s)
		return s
	end
	rawset(_G, "StateMissionEnd", {
		change_state = function(self, sub)
			self.current_state = sub
			sub.on_enter(self)
		end,
		update = function(self, dt) self.current_state.update(self, dt) end,
	})
	screen("MissionEndStateWaitingForData")
	screen("MissionEndStateMissionResult", { 0.5, 2, 1.5 })
	screen("MissionEndStateMissionScore")
	screen("MissionEndStatePrePlanetPause")
	screen("MissionEndStateDone")
	e.opts.modules["lua/states/state_mission_end"] = true
	rawget(_G, "require")("lua/states/state_mission_end")
	local function player(owned)
		local go = { flags = {} }
		function go:owned() return owned end
		function go:set(k, v) self.flags[k] = v end
		return { player_go = go }
	end
	local state = setmetatable({ players = { player(true), player(false) } }, { __index = rawget(_G, "StateMissionEnd") })
	return state, entered
end

local function go(state, name) state:change_state(rawget(_G, name)) end

test("expedite_instant_goes_from_waiting_to_done", function()
	local e = engine(nil, "instant")
	local state, entered = mission_end(e)
	go(state, "MissionEndStateWaitingForData")
	go(state, "MissionEndStateMissionResult")
	E.eq(table.concat(entered, " "), "MissionEndStateWaitingForData MissionEndStateDone")
	local mine, theirs = state.players[1].player_go.flags, state.players[2].player_go.flags
	E.eq(mine.is_ready, true, "ready on Done")
	E.eq(mine.is_ready_planet, true, "ready for a host still on the planet pause")
	E.eq(mine.is_ready_genocide, true, "ready for a host still on the genocide results")
	E.eq(theirs.is_ready, nil, "other players press their own")
	E.has(e:log("obsidianqol"), "expedite mission end: results skipped")
end)

test("expedite_faster_fits_the_steps_in_a_second", function()
	local e = engine(nil, "faster")
	local state = mission_end(e)
	go(state, "MissionEndStateMissionResult")
	state:update(0.1)
	E.eq(state.seen_dt, 0.4, "4 s of steps in 1 s")
	go(state, "MissionEndStateWaitingForData")
	state:update(0.1)
	E.eq(state.seen_dt, 0.1, "waiting for data keeps the real time")
end)

test("expedite_faster_speeds_up_a_long_screen_and_presses_after_a_second", function()
	local e = engine(nil, "faster")
	local state = mission_end(e)
	go(state, "MissionEndStateMissionScore")
	state:update(0.5)
	E.eq(state.seen_dt, 1, "twice as fast without a step list")
	state:update(0.6)
	local after = state.seen_dt
	state:update(0.1)
	E.eq(state.seen_dt > after / 0.6 * 0.1, true, "faster after a second")
	go(state, "MissionEndStatePrePlanetPause")
	state:update(0.5)
	E.eq(state.players[1].player_go.flags.is_ready_planet, nil, "not before a second")
	state:update(0.6)
	E.eq(state.players[1].player_go.flags.is_ready_planet, true, "pressed after a second")
	E.eq(state.players[1].player_go.flags.is_ready, nil, "only for this screen")
	go(state, "MissionEndStateDone")
	state:update(0.5)
	E.eq(state.seen_dt, 0.5, "Done keeps the real time")
end)

-- The game's input code and cursor, and a save with the crosshair option off.
local function cursor_game(e)
	local saved = { crosshair_always_visible = false }
	local seen = {}
	rawset(_G, "SaveManager", { get = function(_, key) return saved[key] end })
	rawset(_G, "CursorManager", { set_cursor = function(_, name) seen.cursor = name end })
	rawset(_G, "InputComponent", { update_masters = function()
		seen.option = rawget(_G, "SaveManager"):get("crosshair_always_visible")
		rawget(_G, "CursorManager"):set_cursor("hud/cursors/game_inactive")
	end })
	e.opts.modules["lua/components/input_component"] = true
	rawget(_G, "require")("lua/components/input_component")
	return seen
end

test("aim_cursor_keeps_the_aiming_cursor_for_the_input_code_alone", function()
	local e = engine()
	local seen = cursor_game(e)
	rawget(_G, "InputComponent").update_masters()
	E.eq(seen.option, true, "the input code sees the crosshair option on")
	E.eq(seen.cursor, "hud/cursors/game", "the aiming cursor, not the dimmed one")
	E.eq(rawget(_G, "SaveManager"):get("crosshair_always_visible"), false, "the option in the menu stays as set")
	rawget(_G, "CursorManager"):set_cursor("hud/cursors/game_inactive")
	E.eq(seen.cursor, "hud/cursors/game_inactive", "anyone else gets the cursor they asked for")
end)

test("aim_cursor_stops_answering_when_the_input_code_fails", function()
	local e = engine()
	rawset(_G, "SaveManager", { get = function() return false end })
	rawset(_G, "InputComponent", { update_masters = function() error("broken") end })
	e.opts.modules["lua/components/input_component"] = true
	rawget(_G, "require")("lua/components/input_component")
	pcall(rawget(_G, "InputComponent").update_masters)
	E.eq(rawget(_G, "SaveManager"):get("crosshair_always_visible"), false)
end)

-- The player's bindings, with the map held.
local function bindings_game(e)
	local map = { func = "filter", key = "FilterSticky", output_name = "map_pressed", input = { button = "map_button", start_sticky = false } }
	local other = { func = "filter", key = "FilterSticky", output_name = "sprint_pressed", input = { button = "sprint_button", start_sticky = false } }
	local bindings = { { func = "button", output_name = "map_button" }, map, other }
	rawset(_G, "InputRebinder", { get_or_default = function() return bindings end })
	e.opts.modules["lua/utility/input_rebinder"] = true
	rawget(_G, "require")("lua/utility/input_rebinder")
	return map, other
end

local function sticky(list, name)
	for _, binding in ipairs(list) do
		if binding.output_name == name then return binding.input.start_sticky end
	end
end

test("toggle_map_makes_the_map_binding_sticky_on_a_copy", function()
	local e = engine()
	local map, other = bindings_game(e)
	local rebinder = rawget(_G, "InputRebinder")
	local list = rebinder:get_or_default("keyboard_mouse")
	E.eq(sticky(list, "map_pressed"), true, "the input code gets a toggle")
	E.eq(sticky(list, "sprint_pressed"), false, "only the map's")
	E.eq(map.input.start_sticky, false, "the player's saved binding stays")
	E.eq(other.input.start_sticky, false)
	E.eq(#list, 3)
end)

-- A unit whose animation state is a list of numbers, and the character and inventory states the fix reads.
local function recoil_game()
	local g = { unit = { alive = true, pose = { 1, 2, 3, 4, 5, 7, 9 } }, set = 0, flows = {}, cs = {}, inv = { current_weapon = "rifle" } }
	rawset(_G, "Unit", {
		alive = function(u) return u.alive end,
		animation_get_state = function(u) return unpack(u.pose) end,
		animation_set_state = function(u, ...)
			u.pose = { ... }
			g.set = g.set + 1
		end,
		flow_event = function(u, name) g.flows[#g.flows + 1] = { u, name } end,
	})
	rawset(_G, "EntityAux", { state = function(_, component) return component == "inventory" and g.inv or g.cs end })
	for _, n in ipairs({ "CharacterStateIdle", "CharacterStateWalk", "CharacterStateFire", "CharacterStateStratagemsThrow", "CharacterStateSwitchWeapon" }) do rawset(_G, n, { name = n }) end
	rawset(_G, "CharacterStateStratagems", { on_enter = function() end })
	rawset(_G, "CharacterStateInteract", { on_enter = function() end })
	return g
end

test("recoil_fix_puts_the_weapon_pose_back_after_a_stratagem", function()
	local g = recoil_game()
	local e = engine()
	local S, idle = rawget(_G, "CharacterStateStratagems"), rawget(_G, "CharacterStateIdle")
	S.on_enter(g.unit, {}, idle)
	g.unit.pose[6] = 12
	g.cs.current_state = rawget(_G, "CharacterStateStratagemsThrow")
	e:frames(30)
	E.eq(g.set, 0, "nothing while throwing")
	g.cs.current_state = idle
	e:frames(19)
	E.eq(g.set, 0, "the weapon comes back up first")
	e:frames(2)
	E.eq(g.set, 1)
	E.eq(g.unit.pose[6], 7, "the weapon layer from before the menu")
	E.eq(g.unit.pose[5], 5, "every other layer as it was")
	E.eq(#g.unit.pose, 7)
	e:frames(30)
	E.eq(g.set, 1, "once")
	-- A layer the throw left alone stays alone, and a weapon switch on the way drops the check.
	S.on_enter(g.unit, {}, idle)
	e:frames(30)
	E.eq(g.set, 1, "nothing to put back")
	S.on_enter(g.unit, {}, idle)
	g.unit.pose[6], g.inv.current_weapon = 12, "pistol"
	g.cs.current_state = rawget(_G, "CharacterStateSwitchWeapon")
	e:frames(2)
	g.cs.current_state = idle
	e:frames(30)
	E.eq(g.set, 1, "switched by the player")
	E.eq(#g.flows, 0)
	-- A pose kept mid-shot would fire the weapon again.
	g.inv.current_weapon = "rifle"
	S.on_enter(g.unit, {}, rawget(_G, "CharacterStateFire"))
	e:frames(30)
	E.eq(g.set, 1, "opened while firing")
	E.eq(E.count(e:log("obsidianqol"), "recoil fix: weapon pose put back"), 1)
end)

test("recoil_fix_sets_up_a_weapon_picked_up_from_the_ground", function()
	local g = recoil_game()
	local e = engine()
	local idle = rawget(_G, "CharacterStateIdle")
	g.cs.current_state = idle
	rawget(_G, "CharacterStateInteract").on_enter(g.unit, {}, idle)
	g.cs.current_state = nil
	g.inv.current_weapon = "machinegun"
	e:frames(10)
	E.eq(#g.flows, 0, "not while picking it up")
	g.cs.current_state = idle
	e:frames(21)
	E.eq(#g.flows, 1)
	E.eq(g.flows[1][1], "machinegun", "the weapon picked up")
	E.eq(g.flows[1][2], "on_wielded")
	E.eq(g.set, 0, "no pose of another weapon put on it")
	-- An interaction that keeps the weapon puts the pose back, as after a stratagem.
	rawget(_G, "CharacterStateInteract").on_enter(g.unit, {}, idle)
	g.unit.pose[6] = 12
	e:frames(25)
	E.eq(g.set, 1)
	E.eq(g.unit.pose[6], 7)
	E.eq(#g.flows, 1)
end)

test("recoil_fix_off_leaves_the_pose", function()
	local g = recoil_game()
	local e = engine({ "recoil_fix" })
	rawget(_G, "CharacterStateStratagems").on_enter(g.unit, {}, rawget(_G, "CharacterStateIdle"))
	g.unit.pose[6] = 12
	g.cs.current_state = rawget(_G, "CharacterStateIdle")
	e:frames(30)
	E.eq(g.set, 0)
end)

-- The character states, and the animation events a unit gets.
local function sprint_game()
	local g = { unit = {}, events = {} }
	rawset(_G, "Unit", {
		animation_event = function(u, name) g.events[#g.events + 1] = name end,
		has_animation_event = function() return true end,
	})
	for _, n in ipairs({ "CharacterStateIdle", "CharacterStateWalk", "CharacterStateFire", "CharacterStateWalkFire",
		"CharacterStateSwitchWeapon", "CharacterStateReload", "CharacterStateDodge" }) do rawset(_G, n, { name = n }) end
	rawset(_G, "CharacterStateSprint", { on_enter = function() end })
	return g
end

test("dive_sprint_fix_frees_the_upper_body_when_a_sprint_starts_from_a_held_pose", function()
	local g = sprint_game()
	engine()
	local sprint = rawget(_G, "CharacterStateSprint")
	for _, from in ipairs({ "CharacterStateIdle", "CharacterStateFire", "CharacterStateWalkFire", "CharacterStateSwitchWeapon" }) do
		sprint.on_enter(g.unit, {}, rawget(_G, from))
	end
	E.eq(table.concat(g.events, ","), "no_animation,no_animation,no_animation,no_animation", "standing (after a dive), firing, a weapon switch")
	-- A walk frees the layer itself; a reload and anything else are left alone.
	sprint.on_enter(g.unit, {}, rawget(_G, "CharacterStateWalk"))
	sprint.on_enter(g.unit, {}, rawget(_G, "CharacterStateReload"))
	sprint.on_enter(g.unit, {}, nil)
	E.eq(#g.events, 4)
end)

test("dive_sprint_fix_off_sends_nothing", function()
	local g = sprint_game()
	engine({ "dive_sprint_fix" })
	rawget(_G, "CharacterStateSprint").on_enter(g.unit, {}, rawget(_G, "CharacterStateIdle"))
	E.eq(#g.events, 0)
end)

test("options_off_change_nothing", function()
	rawset(_G, "GameSettings", { max_aspect_ratio = 16 / 9 })
	local e = engine(IDS)
	local seen = load_states(e)
	rawset(_G, "CampaignSettings", { planet_scanning_states = { { name = "wait", time = 1 } } })
	e:frames(1)
	E.eq(rawget(_G, "ARGUMENTS").skip_splash, nil, "intro")
	local menu = clicking_menu(true)
	update(menu)
	E.eq(menu.showing_menu, false, "login")
	rawget(_G, "StateMenu").on_enter({}, { show_recap = true })
	E.eq(seen.recap, true, "war briefs")
	E.eq(rawget(_G, "CampaignSettings").planet_scanning_states[1].time, 1, "planet scanning")
	E.eq(rawget(_G, "GameSettings").max_aspect_ratio, 16 / 9, "widescreen")
	local running = mission(e, { "me" })
	rawget(_G, "MenuScreenMain").set_active_pause({}, true)
	E.eq(rawget(_G, "PAUSED"), nil, "solo pause")
	E.eq(running("enemy"), true)
	local seen = cursor_game(e)
	rawget(_G, "InputComponent").update_masters()
	E.eq(seen.option, false, "aim cursor")
	E.eq(seen.cursor, "hud/cursors/game_inactive")
	bindings_game(e)
	E.eq(sticky(rawget(_G, "InputRebinder"):get_or_default("keyboard_mouse"), "map_pressed"), false, "toggle map")
end)


-- Dynamic input switch: a keyboard player, a gamepad nobody plays on, and the parts of the game the
-- option reads and writes.
local function dynamic(on)
	local options = { { id = "dynamic_input", type = "toggle", default = false } }
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("obsidian.qol", { options = options }) } }),
		mods = { ["obsidian/mods/obsidian.qol/main"] = E.read("bundled/obsidian.qol/main.lua") },
		settings = on and "option\tobsidian.qol\tdynamic_input\ttoggle\t1\n" or nil,
	})
	e:boot()
	e:start_frames()
	local game = { commands = {}, sets = {}, sent = {}, menus = {}, buttons = {}, pressed = {}, keys = false, stick = 0, set = "keyboard_mouse" }
	rawset(_G, "KEYBOARD_MOUSE_PAD_NUMBER", 0)
	local unit = { alive = true }
	game.unit = unit
	local pad = {
		any_pressed = function() return next(game.pressed) ~= nil end,
		axis_index = function(n) return n end,
		axis = function(n) return { x = n == "left" and game.stick or 0, y = 0 } end,
		button_index = function(n) return n end,
		button = function(n) return game.buttons[n] and 1 or 0 end,
		pressed = function(n) return game.pressed[n] == true end,
	}
	rawset(_G, "PadManager", { get_active_pads = function() return { { pad_number = 1, pad = pad } } end })
	rawset(_G, "Keyboard", { any_pressed = function() return game.keys end, button_index = function() return nil end, pressed = function() return false end })
	rawset(_G, "Mouse", false)
	rawset(_G, "Unit", { alive = function(u) return u and u.alive end })
	rawset(_G, "EntityAux", { queue_command_master = function(u, c, cmd, v) game.commands[#game.commands + 1] = { c, cmd, v } end })
	rawset(_G, "InputUtility", { set_localization_modifier_set_for = function(n)
		game.sets[#game.sets + 1] = n
		game.set = n == 0 and "keyboard_mouse" or ("pad" .. n)
	end })
	rawset(_G, "LocalizationManager", { get_current_modifier_set = function() return game.set end,
		string = function(_, key) return key == "press_cross_to_join" and "PRESS [A] TO JOIN" or key end })
	rawset(_G, "HotjoinManager", { can_pad_join = function() return true end })
	rawset(_G, "NetworkGlobal", { lobby = { lobby_host = function() return "host" end } })
	local Router = {}
	Router.__index = Router
	function Router.transmit_to(_, peer, message, ...)
		local pad_number, index = ...
		game.sent[#game.sent + 1] = { peer, message, pad_number, index, count = select("#", ...) }
	end
	rawset(_G, "NetworkRouter", Router)
	local go = { exists = function() return true end, owned = function() return true end, get = function(_, k) return k == "avatar_unit" and unit or nil end }
	local st = { players = { { pad_number = 0, player_go = go } }, network_router = setmetatable({}, Router),
		get_active_players = function() return { 1 } end, network_session_handler = { is_peer_in_session = function() return true end },
		show_menu = function(_, name, args) game.menus[#game.menus + 1] = args.user_pad_number end }
	rawget(_G, "Boot").state_machine = { get_current_state_name = function() return game.state or "game" end, get_current_state = function() return st end }
	-- Hooks on classes the game defines later go in at the next require.
	e.opts.modules["lua/states/state_game"] = true
	rawget(_G, "require")("lua/states/state_game")
	game.st = st
	return e, game
end

local function last_command(game)
	local c = game.commands[#game.commands - 1]
	return c and c[3]
end

test("dynamic_input_hands_the_first_helldiver_to_the_last_device_used", function()
	local e, game = dynamic(true)
	e:frames(1)
	game.stick = 1
	e:frames(1)
	game.stick = 0
	E.eq(last_command(game), 1, "the gamepad drives the Helldiver")
	E.eq(game.set, "pad1", "and the prompts show its buttons")
	game.keys = true
	e:frames(1)
	game.keys = false
	E.eq(last_command(game), 0, "back to the keyboard")
	E.eq(game.set, "keyboard_mouse")
end)

test("dynamic_input_joins_only_after_a_held_button", function()
	local e, game = dynamic(true)
	game.st.network_router:transmit_to("host", "rpc_add_local_player_request", 1, 1)
	E.eq(#game.sent, 0, "the game's join at a press is held back")
	game.st.network_router:transmit_to("host", "rpc_other", 1)
	E.eq(#game.sent, 1, "other messages go")
	game.buttons.a, game.pressed.a = true, true
	e:frames(60)
	E.eq(#game.sent, 1, "not yet")
	e:frames(40)
	E.eq(#game.sent, 2, "joined after 1.5 s")
	E.eq(game.sent[2][2], "rpc_add_local_player_request")
	E.eq(game.sent[2][3], 1)
	E.eq(last_command(game), 0, "the first Helldiver is back on the keyboard")
	e:frames(200)
	E.eq(#game.sent, 2, "once per hold")
end)

test("dynamic_input_rewords_the_prompt_and_opens_the_pause_menu_for_the_gamepad", function()
	local e, game = dynamic(true)
	E.eq(rawget(_G, "LocalizationManager"):string("press_cross_to_join"), "HOLD [A] TO JOIN")
	game.stick = 1
	e:frames(1)
	game.pressed.start = true
	e:frames(1)
	E.eq(game.menus[1], 1, "Start on the gamepad")
end)

test("dynamic_input_off_leaves_joining_and_input_alone", function()
	local e, game = dynamic(false)
	game.stick = 1
	e:frames(5)
	E.eq(#game.commands, 0)
	game.st.network_router:transmit_to("host", "rpc_add_local_player_request", 1, 1)
	E.eq(#game.sent, 1, "a press joins as in the game")
	E.eq(rawget(_G, "LocalizationManager"):string("press_cross_to_join"), "PRESS [A] TO JOIN")
end)

-- The game's network messages keep their number of values, option on or off: one value too many ends
-- the online session (1.9.0 did, at login).
test("network_messages_go_on_with_the_values_they_came_with", function()
	for _, on in ipairs({ true, false }) do
		local _, game = dynamic(on)
		game.st.network_router:transmit_to("host", "rpc_two")
		game.st.network_router:transmit_to("host", "rpc_three", 1, nil, nil)
		E.eq(game.sent[1].count, 0, "nothing added")
		E.eq(game.sent[2].count, 3, "trailing nils kept")
	end
end)

return T
