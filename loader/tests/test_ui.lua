-- obsidian.ui against small stand-ins for the game's two drawing classes and a few of its screens.
local E = require "fake_engine"
local F = require "fake_ui"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

-- HdGui and GuiDrawer only record, so a test can tell what reached them. The screens draw inside
-- their update and keep a count, which is the logic a mute must not stop.
local function install_game(e, ui)
	local game = { hd = {}, drawer = {}, logic = 0, hud_logic = 0, map_input = 0 }
	e.game = game
	local HdGui = {}
	HdGui.__index = HdGui
	for _, name in ipairs({ "rect", "rotated_rect", "image", "rotated_image", "text" }) do
		HdGui[name] = function(_, what) game.hd[#game.hd + 1] = name .. ":" .. tostring(what) end
	end
	function HdGui.create_text(_, text, _, color) game.retained = { text = text, color = color } return "handle" end
	function HdGui.update_text(_, id, text, _, color) game.retained = { id = id, text = text, color = color } end
	function HdGui.create_image(_, name, _, _, color) game.image = { name = name, color = color } return "image" end
	function HdGui:isa(class) return class == HdGui end
	function HdGui.calculate_pos(_, pos, size) return { x = pos.x * 2, y = pos.y * 2 }, { x = size.x * 2, y = size.y * 2 } end
	rawset(_G, "HdGui", HdGui)
	local gui = setmetatable({}, HdGui)
	game.gui = gui
	local GuiDrawer = {}
	GuiDrawer.__index = GuiDrawer
	for _, name in ipairs({ "draw_rect", "draw_text", "draw_texture" }) do
		GuiDrawer[name] = function(_, what) game.drawer[#game.drawer + 1] = name .. ":" .. tostring(what) end
	end
	rawset(_G, "GuiDrawer", GuiDrawer)
	local drawer = setmetatable({}, GuiDrawer)
	game.gui_drawer = drawer

	rawset(_G, "MenuScreenLoadout", { update = function()
		game.logic = game.logic + 1
		gui:image("panel")
		gui:text("name")
		drawer:draw_text("title")
	end })
	rawset(_G, "Hud", {
		draw_hud = function(self)
			game.hud_logic = game.hud_logic + 1
			gui:image("health")
			rawget(_G, "Hud").draw_map(self)
			gui:text("ammo")
		end,
		draw_map = function()
			game.map_input = game.map_input + 1
			gui:image("map")
		end,
	})
	rawset(_G, "KEYBOARD_MOUSE_PAD_NUMBER", 9)
	rawset(_G, "Vector3Aux", { unbox = function(t) return t end })
	rawset(_G, "MenuScreenBase", { is_box_selected_by_cursor = function(self) game.tested = self return "the game's answer" end })
	local boot = rawget(_G, "Boot")
	boot.package_manager = { is_package_online = function(_, name) return not (ui.offline and ui.offline[name]) end }
	ui.state_object = {}

	game.loadout_screen = { class_name = "MenuScreenLoadout", option_menus = {} }
	return game
end

local function start(source, opts)
	opts = opts or {}
	local e = E.new({ deploy = E.deploy({ font_files = opts.font_files, font_paths = opts.font_paths, mods = opts.mods or { E.mod("m", { scope = opts.scope or "client" }) }, texts = { en = {} } }),
		mods = opts.sources or { ["obsidian/mods/m/main"] = source }, lobby = opts.lobby })
	e:boot()
	e:start_frames()
	local ui = F.install(e, opts)
	local game = install_game(e, ui)
	if opts.loadout then ui.state_object = { active_screens = { game.loadout_screen } } end
	return e, ui, game
end

-- The module itself, for the switches a probe run turns on.
local function module(e)
	return e.loaded["obsidian/loader/ui"]
end

local function on_screen(ui, id)
	ui.state = "menu"
	ui.state_object = { active_screens = { { class_name = id } } }
end

test("nothing_runs_until_a_mod_asks", function()
	local e, ui, game = start("return true")
	local rect, base = rawget(_G, "HdGui").rect, rawget(_G, "MenuScreenBase").is_box_selected_by_cursor
	ui.state = "menu"
	game.gui:rect("before")
	e:frames(5)
	E.eq(#ui.guis, 0, "no gui")
	E.eq(rawget(_G, "HdGui").rect, rect, "HdGui stays the game's")
	E.eq(rawget(_G, "MenuScreenBase").is_box_selected_by_cursor, base, "and the hit test")
	E.eq(module(e).wanted, false)
	E.lacks(e:log(), "ui:")
end)

test("the_table_is_there_for_any_mod_with_no_dependency", function()
	local e = start([[ RESULT = obsidian.capabilities.ui and type(obsidian.ui.canvas) ]], { scope = "client" })
	E.eq(rawget(_G, "RESULT"), "function")
	E.has(e:log(), "started m")
end)

test("the_current_screen_comes_from_the_games_stack", function()
	local e, ui = start("return true")
	local current = rawget(_G, "obsidian").ui.current
	ui.state = "menu"
	ui.state_object = { active_screens = { { class_name = "MenuScreenMain" }, { class_name = "MenuScreenArmoryProfile" } } }
	E.eq(current().state, "menu")
	E.eq(current().screen, "armory_profile", "the top of the stack")
	ui.state_object = { active_screens = {} }
	E.eq(current().screen, "ship")
	ui.state = "game"
	ui.state_object = {}
	E.eq(current().screen, "hud")
	ui.state_object = { active_screen = { class_name = "MenuScreenMain" } }
	E.eq(current().screen, "main", "the pause menu")
	ui.state = "main_menu"
	ui.state_object = {}
	E.eq(current().screen, "title")
	ui.state = "mission_end"
	ui.state_object = { current_state = { name = "mission_score" } }
	E.eq(current().screen, "mission_end")
	E.eq(current().sub, "mission_score")
	ui.state_object = { active_screens = { { class_name = "MenuScreenCampaign", active_state = { name = "recap" } } } }
	ui.state = "menu"
	E.eq(current().sub, "recap")
end)

local SCREEN = [[
	LOG = {}
	obsidian.ui.on("loadout", {
		enter = function(screen) LOG[#LOG + 1] = "enter " .. screen.screen end,
		draw = function(kit, data, dt)
			LOG[#LOG + 1] = "draw"
			LOG.dt = dt
			kit:rect(1, 2, 3, 4, { 10, 20, 30 })
		end,
		exit = function() LOG[#LOG + 1] = "exit" end,
	})
]]

test("a_screen_enters_draws_and_exits_with_the_game", function()
	local e, ui = start(SCREEN)
	local log = rawget(_G, "LOG")
	ui.state = "menu"
	ui.frame()
	E.eq(#log, 0, "not up yet")
	on_screen(ui, "MenuScreenLoadout")
	ui.frame()
	E.eq(log[1], "enter loadout")
	E.eq(log[2], "draw")
	E.eq(log.dt, 0.016)
	E.eq(ui.rects[1].pos.x, 1)
	E.eq(ui.rects[1].gui, "gui fonts/fonts", "shapes and text share a gui once the fonts are there")
	E.eq(ui.gui_args[1], "brightness")
	ui.state_object = { active_screens = {} }
	ui.frame()
	E.eq(log[3], "exit")
	E.eq(#ui.rects, 0)
end)

test("a_mute_cuts_the_output_and_the_logic_keeps_running", function()
	local e, ui, game = start([[ obsidian.ui.mute("loadout") ]], { loadout = true })
	ui.state = "menu"
	e:frames(2)
	game.hd, game.drawer = {}, {}
	rawget(_G, "MenuScreenLoadout").update()
	E.eq(game.logic, 1, "update ran")
	E.eq(#game.hd, 0, "HdGui drew nothing")
	E.eq(#game.drawer, 0, "the shared GuiManager neither")
	game.gui:image("something else")
	game.gui_drawer:draw_text("the fade")
	E.eq(#game.hd, 1, "only what that screen draws is cut")
	E.eq(#game.drawer, 1)
end)

test("unmute_gives_the_screen_back", function()
	local e, ui, game = start([[ obsidian.ui.mute("loadout") ]])
	e:frames(2)
	e:core().current = e:core().mods_by_id.m
	rawget(_G, "obsidian").ui.unmute("loadout")
	e:frames(1)
	rawget(_G, "MenuScreenLoadout").update()
	E.eq(#game.hd, 2)
end)

test("a_retained_object_stays_the_games_but_draws_with_no_alpha", function()
	local e, ui, game = start([[ obsidian.ui.scope("mine", "Widget", "update") obsidian.ui.mute("mine") ]])
	rawset(_G, "Widget", { update = function()
		game.handle = game.gui:create_text("hello", nil, "white")
		game.gui:update_text("handle", "hello", nil, "white")
		game.gui:create_image("box", nil, nil, "white")
	end })
	e:frames(2)
	rawget(_G, "Widget").update()
	E.eq(game.handle, "handle", "the game still gets its handle")
	E.eq(game.retained.text, "hello")
	E.eq(game.retained.color[1], 0, "update_text color")
	E.eq(game.image.color[1], 0, "create_image color")
	e:core().current = e:core().mods_by_id.m
	rawget(_G, "obsidian").ui.unmute("mine")
	e:frames(1)
	rawget(_G, "Widget").update()
	E.eq(game.retained.color, "white")
end)

test("a_part_inside_a_muted_part_stays_unless_it_is_muted_too", function()
	local e, ui, game = start([[ obsidian.ui.mute("hud.panels") ]])
	e:frames(2)
	rawget(_G, "Hud").draw_hud({})
	E.eq(table.concat(game.hd, ","), "image:map")
	E.eq(game.hud_logic, 1)
	E.eq(game.map_input, 1, "map input lives inside draw_map")
end)

test("hud_mutes_every_part_and_the_map_input_still_runs", function()
	local e, ui, game = start([[ obsidian.ui.mute("hud") ]])
	e:frames(2)
	rawget(_G, "Hud").draw_hud({})
	E.eq(#game.hd, 0)
	E.eq(game.map_input, 1)
end)

test("a_mute_change_makes_the_hud_rebuild_its_retained_objects", function()
	local e, ui = start([[ obsidian.ui.mute("hud.panels") ]])
	ui.state_object = { hud = { hd_gui = {} } }
	e:frames(2)
	E.eq(ui.state_object.hud.hd_gui.is_dirty, true, "as a resolution change does")
	ui.state_object.hud.hd_gui.is_dirty = false
	e:frames(1)
	E.eq(ui.state_object.hud.hd_gui.is_dirty, false, "once")
	e:core().current = e:core().mods_by_id.m
	rawget(_G, "obsidian").ui.unmute("hud.panels")
	e:frames(1)
	E.eq(ui.state_object.hud.hd_gui.is_dirty, true)
end)

test("a_mute_of_a_class_the_game_loads_later_waits_for_it", function()
	local e, ui, game = start([[ obsidian.ui.mute("armory_profile") ]])
	e:frames(2)
	rawset(_G, "MenuScreenArmoryProfile", { update = function() game.gui:rect("row") end })
	e:frames(1)
	rawget(_G, "MenuScreenArmoryProfile").update()
	E.eq(#game.hd, 0)
end)

test("a_function_any_mod_names_can_be_muted", function()
	local e, ui, game = start([[ obsidian.ui.scope("mine", "Hud", "draw_map") obsidian.ui.mute("mine") ]])
	e:frames(2)
	rawget(_G, "Hud").draw_map()
	E.eq(#game.hd, 0)
	E.eq(game.map_input, 1)
end)

test("a_screen_can_hold_a_mute_only_while_it_is_up", function()
	local e, ui, game = start([[ obsidian.ui.on("loadout", { mute = { "loadout", "hud.map" }, draw = function() end }) ]])
	ui.state = "menu"
	e:frames(2)
	rawget(_G, "MenuScreenLoadout").update()
	E.eq(#game.hd, 2, "not up")
	game.hd = {}
	on_screen(ui, "MenuScreenLoadout")
	e:frames(2)
	rawget(_G, "MenuScreenLoadout").update()
	E.eq(#game.hd, 0, "up")
	rawget(_G, "Hud").draw_map()
	E.eq(#game.hd, 0, "the list")
	ui.state_object = { active_screens = {} }
	e:frames(2)
	rawget(_G, "MenuScreenLoadout").update()
	E.eq(#game.hd, 2, "gone with the screen")
end)

test("a_screen_that_fails_gives_the_game_its_screen_back", function()
	local e, ui, game = start([[
		RUNS = 0
		obsidian.ui.mute("loadout")
		obsidian.ui.mute("hud.panels")
		obsidian.ui.on("loadout", { mute = true, draw = function()
			RUNS = RUNS + 1
			error("broken screen")
		end })
	]])
	on_screen(ui, "MenuScreenLoadout")
	e:frames(1)
	E.eq(rawget(_G, "RUNS"), 1)
	e:frames(5)
	E.eq(rawget(_G, "RUNS"), 1, "it stops")
	rawget(_G, "MenuScreenLoadout").update()
	E.eq(#game.hd, 2, "the game draws it again")
	E.eq(game.logic, 1)
	E.has(e:log(), "gave the screen back")
	E.has(e:log(), "broken screen")
	rawget(_G, "Hud").draw_hud({})
	game.hd = {}
	rawget(_G, "Hud").draw_hud({})
	E.eq(table.concat(game.hd, ","), "image:map", "another screen of the mod is left alone")
end)

test("a_failed_screen_cant_be_muted_again", function()
	local e, ui, game = start([[
		obsidian.ui.on("loadout", { mute = true, enter = function() error("no") end })
		MOD_MUTE = function() return obsidian.ui.mute("loadout") end
	]])
	on_screen(ui, "MenuScreenLoadout")
	e:frames(2)
	e:core().current = e:core().mods_by_id.m
	local ok, why = rawget(_G, "MOD_MUTE")()
	E.eq(ok, false)
	E.eq(why, "failed")
end)

test("a_failing_screen_counts_against_the_mod_and_never_reaches_the_game", function()
	local e, ui = start([[ obsidian.ui.on("loadout", { draw = function() error("x") end }) ]])
	on_screen(ui, "MenuScreenLoadout")
	e:frames(3)
	E.eq(e:core().mods_by_id.m.errors, 1)
	E.eq(rawget(_G, "FRAMES"), 3, "the game's update carries on")
end)

test("a_session_mod_stops_and_unmutes_while_the_lobby_isnt_cleared", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e, ui, game = start([[
		DRAWS, EXITS = 0, 0
		obsidian.ui.mute("hud.map")
		obsidian.ui.on("loadout", { mute = true, draw = function() DRAWS = DRAWS + 1 end, exit = function() EXITS = EXITS + 1 end })
	]], { scope = "session", lobby = lobby })
	on_screen(ui, "MenuScreenLoadout")
	e:frames(32)
	local before = rawget(_G, "DRAWS")
	E.eq(before > 0, true)
	rawget(_G, "MenuScreenLoadout").update()
	rawget(_G, "Hud").draw_map()
	E.eq(#game.hd, 0, "muted while cleared")
	lobby.members = { "me", "stranger" }
	e:frames(32)
	local stopped = rawget(_G, "DRAWS")
	e:frames(5)
	E.eq(rawget(_G, "DRAWS"), stopped, "no more draws")
	E.eq(rawget(_G, "EXITS"), 0, "no mod code runs while gated")
	rawget(_G, "MenuScreenLoadout").update()
	rawget(_G, "Hud").draw_map()
	E.eq(#game.hd, 3, "the game draws again")
	lobby.members = { "me" }
	e:frames(32)
	E.eq(rawget(_G, "DRAWS") > stopped, true, "back with the lobby")
end)

test("a_client_mod_runs_in_any_lobby", function()
	local lobby = { my_id = "me", members = { "me", "stranger" }, data = {} }
	local e, ui = start([[ DRAWS = 0 obsidian.ui.on("loadout", { draw = function() DRAWS = DRAWS + 1 end }) ]], { lobby = lobby })
	on_screen(ui, "MenuScreenLoadout")
	e:frames(32)
	E.eq(rawget(_G, "DRAWS"), 32)
end)

test("a_mod_that_was_switched_off_gives_up_its_mutes", function()
	local e, ui, game = start([[ obsidian.ui.mute("loadout") ]])
	e:frames(2)
	e:core().mods_by_id.m.off = true
	e:frames(1)
	rawget(_G, "MenuScreenLoadout").update()
	E.eq(#game.hd, 2)
end)

test("the_canvas_can_be_held_back_from_a_mission", function()
	local e, ui, game = start([[
		DRAWS, ENTERED = 0, 0
		obsidian.ui.on("hud", { mute = true, enter = function() ENTERED = ENTERED + 1 end, draw = function(kit) DRAWS = DRAWS + 1 kit:rect(0, 0, 1, 1) end })
		KIT = obsidian.ui.canvas("direct")
	]])
	module(e).cleared.mission_draw = false
	ui.state = "game"
	e:frames(3)
	E.eq(rawget(_G, "ENTERED"), 1)
	E.eq(rawget(_G, "DRAWS"), 0, "no draw call in a mission")
	rawget(_G, "KIT"):rect(0, 0, 5, 5)
	rawget(_G, "KIT"):poly({ 0, 0, 1, 0, 1, 1 })
	E.eq(#ui.rects, 0, "nor from a canvas of its own")
	E.eq(#ui.triangles, 0)
	E.eq(#ui.guis, 0)
	rawget(_G, "Hud").draw_hud({})
	E.eq(#game.hd, 3, "with nothing to replace it, the HUD isn't muted")
	module(e).cleared.mission_draw = true
	e:frames(2)
	E.eq(rawget(_G, "DRAWS") > 0, true, "the run turns it on")
end)

local KIT = [[ if FRAME_W then obsidian.ui.frame(FRAME_W, FRAME_H) end KIT = obsidian.ui.canvas("t", OPTS) ]]

local function kit_for(opts)
	opts = opts or {}
	rawset(_G, "FRAME_W", opts.w or false)
	rawset(_G, "FRAME_H", opts.h or false)
	rawset(_G, "OPTS", opts.kit or {})
	local e, ui, game = start(KIT, opts)
	ui.state = "menu"
	ui.height, ui.width = opts.height, opts.width
	e:core().current = e:core().mods_by_id.m
	e:frames(2)
	return rawget(_G, "KIT"), ui, e, game
end

test("a_design_space_scales_and_centres_as_the_game_does", function()
	local kit, ui = kit_for({ w = 1920, h = 1080, width = 2560, height = 1080 })
	kit:rect(0, 0, 100, 50)
	local r = ui.rects[1]
	E.eq(r.pos.x, (2560 - 1920) / 2, "centred on a wide screen")
	E.eq(r.pos.y, 0)
	E.eq(r.size.x, 100)
	ui.rects = {}
	ui.width, ui.height = 3840, 2160
	ui.frame()
	kit:rect(10, 10, 100, 50)
	r = ui.rects[1]
	E.eq(r.pos.x, 20, "a new size next frame")
	E.eq(r.size.y, 100)
end)

test("the_design_space_uses_the_viewport_the_game_letterboxes", function()
	local kit, ui, e = kit_for({ w = 1280, h = 720 })
	ui.state_object = { world_proxy = { default_viewport = function() return { window = function() return 1000, 800, 100, 50 end } end } }
	ui.frame()
	kit:rect(0, 0, 10, 10)
	E.eq(math.abs(ui.rects[1].pos.x - 100) < 0.001, true)
	E.eq(math.abs(ui.rects[1].pos.y - (50 + (800 - 720 * (1000 / 1280)) * 0.5)) < 0.001, true)
end)

test("the_players_ui_scale_counts", function()
	local kit, ui, e = kit_for({ w = 1280, h = 720, width = 1280, height = 720 })
	rawset(_G, "SaveManager", { get = function(_, key) return key == "ui_scale" and 0.5 or nil end })
	ui.frame()
	kit:rect(0, 0, 100, 100)
	local r = ui.rects[1]
	E.eq(r.size.x, 50)
	E.eq(r.pos.x, 320, "x centres the smaller panel")
	E.eq(r.pos.y, 0, "y keeps the game's sum, which divides the scale back out")
end)

test("safe_is_the_area_of_the_games_menus_in_the_mods_coordinates", function()
	local kit, ui, e = kit_for({ width = 1280, height = 720 })
	local safe = rawget(_G, "obsidian").ui.safe()
	E.eq(safe.x, 96)
	E.eq(safe.y, 86)
	E.eq(safe.w, 1088)
	E.eq(safe.h, 548)
	rawget(_G, "obsidian").ui.frame(2560, 1440)
	safe = rawget(_G, "obsidian").ui.safe()
	E.eq(safe.x, 192, "in design units")
	E.eq(safe.w, 2176)
end)

test("a_shape_goes_to_the_engine_as_the_run_showed_it_draws", function()
	local kit, ui = kit_for()
	kit:poly({ 100, 100, 200, 100, 150, 200 }, "#ff0000")
	local t = ui.triangles[1]
	E.eq(#ui.triangles, 1, "points as (x, 0, y)")
	E.eq(t.a.y, 0)
	E.eq(t.a.z, 100)
	E.eq(t.layer, 700)
	E.eq(t.color[2], 255)
	E.eq(t.color[1], 255, "alpha first")
end)

test("a_clockwise_shape_is_sent_counter_clockwise", function()
	local kit, ui = kit_for()
	kit:poly({ 100, 100, 150, 200, 200, 100 })
	local t = ui.triangles[1]
	local area = (t.b.x - t.a.x) * (t.c.z - t.a.z) - (t.c.x - t.a.x) * (t.b.z - t.a.z)
	E.eq(area > 0, true)
end)

test("a_poly_is_a_fan", function()
	local kit, ui = kit_for()
	kit:poly({ 0, 0, 10, 0, 10, 10, 0, 10 })
	E.eq(#ui.triangles, 2)
	kit:hex(50, 50, 10)
	E.eq(#ui.triangles, 6, "four more")
end)

test("a_panel_cuts_the_corners_asked_for", function()
	local kit, ui = kit_for()
	kit:panel({ 0, 0, 100, 50, cut = 10, corners = "tr bl", fill = "#38383e" })
	E.eq(#ui.triangles, 4, "six points")
	local xs = {}
	for _, t in ipairs(ui.triangles) do
		xs[t.b.x .. "," .. t.b.z] = true
		xs[t.c.x .. "," .. t.c.z] = true
		xs[t.a.x .. "," .. t.a.z] = true
	end
	E.eq(xs["0,10"], true, "bottom left cut")
	E.eq(xs["10,0"], true)
	E.eq(xs["100,40"], true, "top right cut")
	E.eq(xs["90,50"], true)
	E.eq(xs["100,0"], true, "bottom right whole")
	E.eq(xs["0,0"], nil)
end)

test("a_panel_line_sits_inside_it_on_the_next_layer", function()
	local kit, ui = kit_for()
	kit:panel({ 0, 0, 100, 50, line = { 255, 255, 255 }, line_width = 2 })
	E.eq(#ui.triangles, 8, "two per edge")
	E.eq(ui.triangles[1].layer, 701)
	for _, t in ipairs(ui.triangles) do
		for _, p in ipairs({ t.a, t.b, t.c }) do
			E.eq(p.x >= 0 and p.x <= 100 and p.z >= 0 and p.z <= 50, true, "inside")
		end
	end
	E.eq(kit:layer(), 700, "the layer comes back")
end)

test("a_clip_cuts_shapes_exactly", function()
	local kit, ui = kit_for()
	kit:clip(0, 0, 50, 50, function()
		kit:rect(25, 25, 100, 100)
		kit:rect(60, 60, 10, 10)
		kit:poly({ 40, 0, 80, 0, 80, 40, 40, 40 })
	end)
	E.eq(#ui.rects, 1, "one was outside")
	E.eq(ui.rects[1].size.x, 25)
	for _, t in ipairs(ui.triangles) do
		E.eq(t.a.x <= 50 and t.b.x <= 50 and t.c.x <= 50, true)
	end
	E.eq(#ui.triangles > 0, true)
	kit:rect(60, 60, 10, 10)
	E.eq(#ui.rects, 2, "the clip ended")
end)

test("a_clip_inside_a_clip_is_the_part_they_share", function()
	local kit, ui = kit_for()
	kit:clip(0, 0, 50, 50, function()
		kit:clip(40, 40, 50, 50, function() kit:rect(0, 0, 200, 200) end)
	end)
	E.eq(ui.rects[1].pos.x, 40)
	E.eq(ui.rects[1].size.x, 10)
end)

test("a_clip_cuts_text_at_whole_letters_and_lines", function()
	local kit, ui = kit_for()
	kit:clip(0, 0, 50, 50, function()
		kit:text("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 0, 10, { font = "debug" })
		kit:text("out", 0, 100, { font = "debug" })
	end)
	E.eq(#ui.texts, 1)
	local shown = ui.texts[1].text
	E.eq(#shown < 26 and #shown > 0, true)
	E.eq(#shown * 8 <= 50, true, "what's left fits")
end)

test("an_error_in_a_clip_ends_the_clip_and_goes_on_to_the_screen_handler", function()
	local kit, ui = kit_for()
	local ok, err = pcall(function() kit:clip(0, 0, 5, 5, function() error("inner", 0) end) end)
	E.eq(ok, false)
	E.eq(err, "inner")
	kit:rect(60, 60, 10, 10)
	E.eq(#ui.rects, 1)
end)

test("text_takes_its_font_size_tracking_and_alignment", function()
	local kit, ui = kit_for()
	kit:text("HELLDIVERS", 300, 200, { font = "heading", size = 48, tracking = 4, align = "center", color = "#ffc433" })
	local t = ui.texts[1]
	E.eq(t.font, "fonts/insignia_headliner")
	E.eq(t.material, "insignia_headliner")
	E.eq(t.gui, "gui fonts/fonts")
	E.eq(t.size, 48)
	E.eq(t.option, "letter_spacing")
	E.eq(t.value, 4)
	local width = 10 * 48 * 0.5 + 9 * 4
	E.eq(t.pos.x, 300 - width / 2)
	E.eq(t.color[2], 255)
	E.eq(t.pos.z, 701, "over a panel of the same layer")
end)

test("text_with_no_tracking_stays_the_seven_argument_call", function()
	local kit, ui = kit_for()
	kit:text("plain", 0, 0)
	E.eq(ui.texts[1].option, nil)
	E.eq(ui.texts[1].font, "fonts/handel_small", "body by default")
end)

test("a_line_break_starts_a_line_below_and_right_alignment_is_per_line", function()
	local kit, ui = kit_for()
	kit:text("one\ntwo", 100, 100, { font = "debug", align = "right", leading = 20 })
	E.eq(ui.texts[1].text, "one")
	E.eq(ui.texts[2].text, "two")
	E.eq(ui.texts[2].pos.y, 80)
	E.eq(ui.texts[1].pos.x, 100 - 3 * 16 * 0.5)
end)

test("measuring_counts_tracking_and_lines_and_uses_the_design_space", function()
	local kit, ui = kit_for()
	local w, h = kit:measure("LOADOUT", { font = "debug", size = 16 })
	E.eq(w, 7 * 8)
	E.eq(h, 16)
	local w2 = kit:measure("LOADOUT", { font = "debug", size = 16, tracking = 3 })
	E.eq(w2, 7 * 8 + 18)
	local _, h2 = kit:measure("a\nb", { font = "debug", size = 16 })
	E.eq(h2, 32)
	rawget(_G, "obsidian").ui.frame(960, 540)
	ui.frame()
	local w3 = kit:measure("LOADOUT", { font = "debug", size = 16 })
	E.eq(math.abs(w3 - 56) < 0.001, true, "the same in its own units at twice the scale")
end)

test("text_falls_back_to_debug_until_the_game_fonts_are_loaded", function()
	local kit, ui = kit_for()
	ui.offline = { ["resource_packages/localization"] = true }
	kit:text("early", 0, 0, { font = "heading" })
	E.eq(ui.texts[1].font, "foundation/fonts/debug_small")
	for _, material in ipairs(ui.guis) do E.eq(material ~= "fonts/fonts", true, "a gui never lists what isn't loaded") end
	ui.offline = nil
	kit:text("late", 0, 0, { font = "heading" })
	E.eq(ui.texts[2].font, "fonts/insignia_headliner")
end)

test("a_font_with_no_such_letters_gives_way_to_the_chat_font", function()
	local kit, ui = kit_for()
	ui.missing_glyphs = "CJK"
	kit:text("name CJK", 0, 0, { font = "body_large" })
	E.eq(ui.texts[1].font, "fonts/chat")
	kit:text("name", 0, 0, { font = "body_large" })
	E.eq(ui.texts[2].font, "fonts/handel_normal")
end)

test("an_unknown_font_never_reaches_the_engine", function()
	local kit, ui, e = kit_for()
	kit:text("x", 0, 0, { font = "comic_sans" })
	E.eq(ui.texts[1].font, "foundation/fonts/debug_small")
	E.has(e:log(), "no font named comic_sans")
end)

test("text_that_isnt_valid_utf8_is_dropped", function()
	local kit, ui = kit_for()
	rawset(_G, "Utf8", { valid = function(s) return s ~= "bad" end })
	kit:text("bad", 0, 0)
	E.eq(#ui.texts, 0)
	E.eq(select(1, kit:measure("bad")), 0)
end)

local BAKED = { font_files = { "obsidian/fonts/fonts", "m/fonts/fonts" }, font_paths = { "obsidian/fonts/body", "obsidian/fonts/heading", "m/fonts/title" } }

test("the_guis_list_the_fonts_the_deploy_has", function()
	local e, ui = start([[
		KIT = obsidian.ui.canvas("direct")
	]], BAKED)
	e:frames(2)
	rawget(_G, "KIT"):text("Hello", 10, 10, { font = "ui" })
	E.eq(ui.texts[1].font, "obsidian/fonts/body", "the loader's own")
	E.eq(ui.texts[1].material, "body")
	local listed = table.concat(ui.gui_args, " ")
	E.has(listed, "foundation/gui/gui", "shapes share the gui")
	E.has(listed, "fonts/fonts")
	E.has(listed, "obsidian/fonts/fonts")
	E.has(listed, "m/fonts/fonts", "a mod's too")
end)

test("a_deploy_without_baked_fonts_uses_the_games", function()
	local e, ui = start([[
		KIT = obsidian.ui.canvas("direct")
	]])
	e:frames(2)
	rawget(_G, "KIT"):text("Hello", 10, 10, { font = "ui" })
	rawget(_G, "KIT"):text("Hello", 10, 10, { font = "ui_heading" })
	E.eq(ui.texts[1].font, "fonts/handel_small")
	E.eq(ui.texts[2].font, "fonts/insignia_headliner")
end)

test("a_letter_a_baked_font_lacks_comes_from_the_game", function()
	local e, ui = start([[
		KIT = obsidian.ui.canvas("direct")
	]], BAKED)
	e:frames(2)
	ui.missing_glyphs = "x"
	rawget(_G, "KIT"):text("x", 10, 10, { font = "ui" })
	E.eq(ui.texts[1].font, "fonts/chat")
end)

test("a_mod_adds_a_font_it_ships", function()
	local e, ui = start([[
		OK = obsidian.ui.font("title", { path = "m/fonts/title", size = 48 })
		KIT = obsidian.ui.canvas("direct")
	]], BAKED)
	e:frames(2)
	E.eq(rawget(_G, "OK"), true)
	rawget(_G, "KIT"):text("Hello", 10, 10, { font = "title" })
	E.eq(ui.texts[1].font, "m/fonts/title")
	E.eq(ui.texts[1].material, "title", "the bake names it")
	E.eq(ui.texts[1].size, 48)
end)

test("a_font_the_deploy_doesnt_have_never_reaches_the_engine", function()
	local e, ui = start([[
		ONE = obsidian.ui.font("a", { path = "m/fonts/other" })
		TWO = obsidian.ui.font("b", { path = "nothing" })
		THREE = obsidian.ui.font("ui", { path = "m/fonts/title" })
		FOUR = obsidian.ui.font("c")
		KIT = obsidian.ui.canvas("direct")
	]], BAKED)
	e:frames(2)
	E.eq(rawget(_G, "ONE"), false, "not deployed")
	E.eq(rawget(_G, "TWO"), false, "no folder, so no material file")
	E.eq(rawget(_G, "THREE"), false, "a name that's taken")
	E.eq(rawget(_G, "FOUR"), false)
	rawget(_G, "KIT"):text("Hello", 10, 10, { font = "a" })
	E.eq(ui.texts[1].font, "foundation/fonts/debug_small", "an unknown name falls back")
end)

test("icons_can_be_held_back", function()
	local kit, ui, e = kit_for()
	module(e).cleared.icons = false
	E.eq(kit:icon("icon_arrow_01", 0, 0, 10, 10), false)
	E.eq(#ui.bitmaps, 0)
	for _, material in ipairs(ui.guis) do E.eq(material ~= "hud/hud", true) end
	kit:band(0, 0, 10, 10, { 255, 0, 0 })
	E.eq(#ui.bitmaps, 0)
	E.eq(#ui.rects, 1, "a flat tint meanwhile")
	E.eq(math.floor(ui.rects[1].color[1]), 127)
end)

test("an_icon_draws_by_the_shipped_call_and_a_name_it_cant_vouch_for_never_reaches_it", function()
	local kit, ui, e = kit_for()
	module(e).cleared.icons = true
	E.eq(kit:icon("icon_arrow_01", 5, 6, 10, 20, "#ffffff"), true)
	local b = ui.bitmaps[1]
	E.eq(b.gui, "gui hud/hud")
	E.eq(b.name, "icon_arrow_01")
	E.eq(b.pos.x, 5)
	E.eq(b.size.y, 20)
	E.eq(b.uv1.x, 1)
	E.eq(kit:icon("no_such_icon", 0, 0, 1, 1), false)
	E.eq(#ui.bitmaps, 1)
	E.has(e:log(), "no icon named no_such_icon")
	rawset(_G, "LoadoutSettings", { stratagems = { { icon = "stratagem_resupply_icon" } } })
	E.eq(kit:icon("stratagem_resupply_icon", 0, 0, 1, 1), true, "the game's own table vouches for it")
	local names = table.concat(rawget(_G, "obsidian").ui.icons(), " ")
	E.has(names, "stratagem_resupply_icon")
	E.has(names, "hud_icon_color_gradient_01")
end)

test("a_clip_moves_an_icons_uv_with_it", function()
	local kit, ui, e = kit_for()
	module(e).cleared.icons = true
	kit:clip(0, 0, 50, 50, function() kit:icon("icon_arrow_01", 0, 0, 100, 100) end)
	local b = ui.bitmaps[1]
	E.eq(b.size.x, 50)
	E.eq(b.uv1.x, 0.5)
	E.eq(b.uv0.y, 0.5, "v runs down the image, so the bottom half is kept")
	E.eq(b.uv1.y, 1)
end)

test("the_band_is_the_games_gradient_once_it_can_draw", function()
	local kit, ui, e = kit_for()
	module(e).cleared.icons = true
	kit:band(0, 0, 10, 10, { 255, 0, 0 })
	E.eq(ui.bitmaps[1].name, "hud_icon_color_gradient_01")
	E.eq(#ui.rects, 0)
end)

test("a_bar_fills_by_its_value", function()
	local kit, ui = kit_for()
	kit:bar(10, 10, 200, 8, 0.25, "#ff0000", "#000000")
	E.eq(#ui.rects, 2)
	E.eq(ui.rects[2].size.x, 50)
	E.eq(ui.rects[2].pos.z, 701)
	ui.rects = {}
	kit:bar(10, 10, 200, 8, 3, "#ff0000")
	E.eq(ui.rects[1].size.x, 200, "no further than full")
	ui.rects = {}
	kit:bar(10, 10, 200, 8, 0, "#ff0000")
	E.eq(#ui.rects, 0)
end)

test("colors_take_tables_and_hex_with_alpha", function()
	local kit, ui = kit_for()
	kit:rect(0, 0, 1, 1, { 1, 2, 3, 40 })
	kit:rect(0, 0, 1, 1, "#ffc43380")
	E.eq(ui.rects[1].color[1], 40)
	E.eq(ui.rects[2].color[1], 128)
	E.eq(ui.rects[2].color[3], 196)
	local ok = pcall(function() kit:rect(0, 0, 1, 1, "orange") end)
	E.eq(ok, false, "an error for the screen handler, not an engine call")
end)

test("a_kit_fades_with_the_game_unless_asked_not_to", function()
	local kit, ui, e = kit_for()
	ui.state_object = { fade_manager = { current = 0.75 } }
	ui.frame()
	kit:rect(0, 0, 1, 1, { 1, 2, 3 })
	E.eq(ui.rects[1].color[1], 255 * 0.25)
	local over = rawget(_G, "obsidian").ui.canvas("over", { over_fades = true })
	over:rect(0, 0, 1, 1, { 1, 2, 3 })
	E.eq(ui.rects[2].color[1], 255)
end)

test("opacity_and_layer_stay_in_range", function()
	local kit, ui = kit_for()
	kit:opacity(0.5)
	kit:layer(5000)
	kit:rect(0, 0, 1, 1)
	E.eq(ui.rects[1].color[1], 127.5)
	E.eq(ui.rects[1].pos.z, 875, "under the Obsidian menu")
	E.eq(kit:layer(-3), 0)
end)

test("the_guis_go_before_the_world_at_shutdown", function()
	local kit, ui, e = kit_for()
	kit:rect(0, 0, 1, 1)
	kit:text("x", 0, 0, { font = "heading" })
	E.eq(#ui.guis, 1, "shapes and text share one")
	rawget(_G, "shutdown")()
	E.eq(ui.destroyed, 1)
	E.eq(rawget(_G, "SHUTDOWNS"), 1)
end)

test("hits_move_with_the_layout", function()
	local e, ui, game = start([[
		obsidian.ui.remap_hits("loadout", function(x, y, w, h)
			if x == 2000 then return false end
			if x == 6000 then return nil end
			return x + 100, y, w, h
		end)
	]])
	local base = rawget(_G, "MenuScreenBase")
	e:frames(1)
	local screen = { class_name = "MenuScreenLoadout", user_pad_number = 9, cursor_position = { x = 125, y = 10 } }
	local box = { x = 10, y = 0 }
	local size = { x = 10, y = 10 }
	E.eq(base.is_box_selected_by_cursor(screen, game.gui, box, size), true, "pixels from calculate_pos, then moved")
	screen.cursor_position.x = 25
	E.eq(base.is_box_selected_by_cursor(screen, game.gui, box, size), false, "not where it was")
	E.eq(base.is_box_selected_by_cursor(screen, game.gui, { x = 1000, y = 0 }, size), false, "false is nowhere")
	screen.cursor_position.x = 6005
	E.eq(base.is_box_selected_by_cursor(screen, game.gui, { x = 3000, y = 0 }, size), true, "nil leaves it")
	screen.user_pad_number = 1
	E.eq(base.is_box_selected_by_cursor(screen, game.gui, box, size), false, "a pad has no cursor")
	E.eq(base.is_box_selected_by_cursor({ class_name = "MenuScreenMain" }, game.gui, box, size), "the game's answer", "other screens are the game's")
end)

test("a_failing_remap_goes_back_to_the_games_test", function()
	local e, ui, game = start([[ obsidian.ui.remap_hits("loadout", function() error("bad") end) ]])
	e:frames(1)
	local base = rawget(_G, "MenuScreenBase")
	local screen = { class_name = "MenuScreenLoadout", user_pad_number = 9, cursor_position = { x = 1, y = 1 } }
	E.eq(base.is_box_selected_by_cursor(screen, game.gui, { x = 0, y = 0 }, { x = 1, y = 1 }), "the game's answer")
end)

test("a_prompt_gives_the_button_of_that_players_device", function()
	local e, ui = start("return true")
	rawset(_G, "GameSettings", { get_main_pad_number = function() return 2 end })
	rawset(_G, "LocalizationManager", {
		modifier_sets = {
			keyboard_mouse = { { modifier = "#cross", value = "\194\172Enter\194\172" }, { modifier = "#mod_fire", value = "LMB" } },
			pad2 = { { modifier = "#cross", value = "A" }, { modifier = "#mod_fire", value = "RT" } },
		},
		string = function(_, key, _, set) return key .. " in " .. tostring(set) end,
	})
	local prompt = rawget(_G, "obsidian").ui.prompt
	E.eq(prompt("select"), "A", "the main player's pad")
	E.eq(prompt("select", 9), "Enter", "the keyboard, without the overlay marks")
	E.eq(prompt("fire", 2), "RT")
	E.eq(prompt("#mod_fire", 9), "LMB")
	E.eq(prompt("menu_loadout_select", 2), "menu_loadout_select in pad2", "a text key, with its buttons filled in")
	E.eq(prompt("#no_such_button", 2), nil)
	E.eq(prompt(""), nil)
end)

-- A player as the game keeps one, and its panel.
local function with_players(ui, game)
	local go = { name = "Tester", xp = 950, index = 2 }
	function go:owned() return true end
	function go:exists() return true end
	function go:get(k) return self[k] end
	function go:has(k) return self[k] ~= nil end
	go.map_ping = { 10, 20 }
	rawset(_G, "GameSettings", { get_main_pad_number = function() return 0 end, player_colors = { [2] = { x = 255, y = 1, z = 2, w = 3 } } })
	rawset(_G, "ProgressionSettings", { get_rank = function() return { name = "rank_3", title = "title_rank_03", icon = "icon_vanity_03" } end })
	local unit = { at = { x = 1, y = 2, z = 3 } }
	rawset(_G, "Unit", { world_position = function(u) return u.at end })
	local health = { hitpoints = 40, max_hitpoints = 100, nested = { secret = 1 } }
	local inventory = { num_grenades = 2, stratagems = { { id = 1, cooldown_timer = 3 } } }
	local st = {
		players = { { player_go = go, pad_number = 0 } },
		hud = { hud_info = { { state = "map", health_state = health, shield_state = { shieldpoints = 5 }, inventory_state = inventory,
			character_state = { sprint_stamina = 2 }, stratagem_cooldowns = { 4 }, respawn_countdown = 7, map_zoom = 2, map_offset = { x = 1, y = 2 } } } },
		generated_level = { map_size = { x = 10, y = 12 } },
		entity_manager = { has_entities = function(_, name) return name == "show_on_map" end, get_slave_entities = function() return { [unit] = {} } end },
		mission_manager = { get_objectives = function() return { { name = "o", type = "t", state = "pending", group = 1, params = { 1, 3 } } } end },
		mission_seed = 77, game_over_timer = 5,
	}
	rawset(_G, "DifficultySettings", { get_lookup_mission_params = function() return { enemy = "bugs", difficulty = 5, environment = "desert" } end,
		get_mission_type = function() return "mission" end })
	ui.state_object = st
	return st, health
end

test("players_are_copies_with_rank_and_color", function()
	local e, ui, game = start("return true")
	ui.state = "game"
	with_players(ui, game)
	local p = rawget(_G, "obsidian").ui.data.players()[1]
	E.eq(p.name, "Tester")
	E.eq(p.title, "title_rank_03")
	E.eq(p.rank_icon, "icon_vanity_03")
	E.eq(p.color[3], 3)
	E.eq(p.is_local, true)
	E.eq(p.pad, 0)
	E.eq(p.slot, 1)
end)

test("the_hud_data_is_a_copy_of_plain_values", function()
	local e, ui, game = start("return true")
	ui.state = "game"
	local _, health = with_players(ui, game)
	local data = rawget(_G, "obsidian").ui.data
	local h = data.hud()
	E.eq(h.state, "map")
	E.eq(h.health.hitpoints, 40)
	E.eq(h.health.nested, nil, "plain values only")
	E.eq(h.shield.shieldpoints, 5)
	E.eq(h.stratagems[1].cooldown_timer, 3)
	E.eq(h.respawn, 7)
	h.health.hitpoints = 999
	E.eq(health.hitpoints, 40, "a write stays in the copy")
	local ok = pcall(function() data.hud = 1 end)
	E.eq(ok, false, "the table itself can't be changed")
end)

test("the_map_data_has_zoom_pings_and_markers", function()
	local e, ui, game = start("return true")
	ui.state = "game"
	with_players(ui, game)
	local m = rawget(_G, "obsidian").ui.data.map()
	E.eq(m.zoom, 2)
	E.eq(m.offset.y, 2)
	E.eq(m.size.y, 12)
	E.eq(m.pings[1].x, 10)
	E.eq(m.markers[1].z, 3)
end)

test("a_ping_of_nothing_isnt_listed", function()
	local e, ui, game = start("return true")
	ui.state = "game"
	local st = with_players(ui, game)
	st.players[1].player_go.map_ping = { -1000, -1000 }
	E.eq(#rawget(_G, "obsidian").ui.data.map().pings, 0)
end)

test("the_mission_data_has_objectives_and_the_seed_says_the_rest", function()
	local e, ui, game = start("return true")
	ui.state = "game"
	with_players(ui, game)
	local m = rawget(_G, "obsidian").ui.data.mission()
	E.eq(m.objectives[1].total, 3)
	E.eq(m.enemy, "bugs")
	E.eq(m.difficulty, 5)
	E.eq(m.environment, "desert")
	E.eq(m.type, "mission")
	E.eq(m.game_over, 5)
end)

test("the_loadout_data_mirrors_the_menu_of_that_player", function()
	local e, ui, game = start("return true", { loadout = true })
	ui.state = "menu"
	local go = { }
	function go:owned() return true end
	game.loadout_screen.option_menus[1] = {
		loadout_menu = { { type = "perks", item = { name = "perk_cardio", icon = "perk_icon" } }, { type = "stratagems", id = 2, index = 7, upgrade_value = 1, item = { name = "s", icon = "i" } } },
		available_items = { perks = { groups = { { name = "g", item_index = 1, items = { { name = "perk_cardio", icon = "perk_icon", secret = {} } } } } } },
		current_selection_index = 2, submenu = "stratagems", selected_loadout_slot = 3, enabled = false, random_loadout = true, loadout_index = 2,
		loadouts = { { perk = "a" } }, new_unlocks = { "x" }, item_upgrades = { y = 1 },
	}
	ui.state_object.players = { { player_go = go, pad_number = 0 } }
	rawset(_G, "GameSettings", { get_main_pad_number = function() return 0 end })
	local d = rawget(_G, "obsidian").ui.data.loadout()
	E.eq(d.selection, 2)
	E.eq(d.submenu, "stratagems")
	E.eq(d.slot, 3)
	E.eq(d.ready, true, "enabled goes false at READY")
	E.eq(d.random, true)
	E.eq(d.items[2].upgrade, 1)
	E.eq(d.items[1].icon, "perk_icon")
	E.eq(d.available.perks[1].items[1].name, "perk_cardio")
	E.eq(d.available.perks[1].items[1].secret, nil)
	E.eq(d.presets[1].perk, "a")
	E.eq(rawget(_G, "obsidian").ui.data.loadout(3), nil, "an empty slot")
	ui.state_object.active_screens = {}
	E.eq(rawget(_G, "obsidian").ui.data.loadout(), nil, "off the screen")
end)

test("the_campaign_data_is_nil_until_the_game_has_it", function()
	local e, ui = start("return true")
	local data = rawget(_G, "obsidian").ui.data
	E.eq(data.campaign(), nil)
	rawset(_G, "CampaignManager", { is_valid = function() return false end })
	E.eq(data.campaign(), nil)
	rawset(_G, "CampaignManager", { is_valid = function() return true end, get_campaign_status = function() return { planets = { { region = 2 } } } end,
		get_campaign_statistics = function() return { kills = 5 } end, get_current_season = function() return 7 end })
	local c = data.campaign()
	E.eq(c.status.planets[1].region, 2)
	E.eq(c.statistics.kills, 5)
	E.eq(c.season, 7)
end)

test("the_mission_end_data_names_the_step_and_copies_the_values", function()
	local e, ui = start("return true")
	local data = rawget(_G, "obsidian").ui.data
	E.eq(data.mission_end(), nil)
	ui.state = "mission_end"
	ui.state_object = { current_state = { name = "score" }, extracted_players = 3, total_players = 4 }
	local d = data.mission_end()
	E.eq(d.sub, "score")
	E.eq(d.values.extracted_players, 3)
end)

test("a_mod_draws_art_from_a_material_it_ships", function()
	local e, ui = start([[
		OK = obsidian.ui.art("mod_art", { path = "m/fonts/fonts" })
		NO = obsidian.ui.art("other", { path = "not/deployed" })
		KIT = obsidian.ui.canvas("direct")
	]], BAKED)
	e:frames(2)
	E.eq(rawget(_G, "OK"), true)
	E.eq(rawget(_G, "NO"), false, "a material the deploy lacks is refused")
	local kit = rawget(_G, "KIT")
	E.eq(kit:art("mod_art", 10, 20, 30, 40, "#ffffff", { 0.5, 0, 0, 0.5 }), true)
	local b = ui.bitmaps[1]
	E.eq(b.name, "mod_art")
	E.eq(b.gui, "gui fonts/fonts", "the gui that lists the material")
	E.eq(b.uv0.x, 0.5, "u0 above u1 mirrors it")
	E.eq(b.uv1.x, 0)
	E.eq(kit:art("other", 0, 0, 1, 1), false, "and a refused name never reaches the engine")
	E.eq(#ui.bitmaps, 1)
end)

test("a_centred_frame_stays_in_the_middle_at_any_ui_scale", function()
	rawset(_G, "SaveManager", { get = function(_, key) return key == "ui_scale" and 0.5 or nil end })
	local e, ui = start([[
		obsidian.ui.frame(1920, 1080, { centered = true })
		KIT = obsidian.ui.canvas("t")
	]])
	ui.state = "menu"
	ui.height, ui.width = 1080, 1920
	e:core().current = e:core().mods_by_id.m
	e:frames(2)
	rawget(_G, "KIT"):rect(0, 0, 100, 50)
	E.eq(ui.rects[1].pos.x, 480)
	E.eq(ui.rects[1].pos.y, 270)
	E.eq(ui.rects[1].size.x, 50)
	rawset(_G, "SaveManager", nil)
end)

-- A photo mode asks for the screen: only the kits of the mods it names draw, the loader's menu still
-- does, and everyone draws again once it lets go, or once it's switched off.
test("solo_holds_back_every_kit_but_the_named_mods", function()
	local e, ui = start(nil, { mods = { E.mod("a", { scope = "client" }), E.mod("b", { scope = "client" }) },
		sources = { ["obsidian/mods/a/main"] = [[ KIT_A = obsidian.ui.canvas("a") SOLO = obsidian.ui.solo SOLOED = obsidian.ui.soloed ]],
			["obsidian/mods/b/main"] = [[ KIT_B = obsidian.ui.canvas("b") ]] } })
	ui.state = "menu"
	e:frames(2)
	local core = e:core()
	local a, b = rawget(_G, "KIT_A"), rawget(_G, "KIT_B")
	local function drawn()
		ui.rects = {}
		a:rect(0, 0, 5, 5)
		b:rect(10, 0, 5, 5)
		local who = {}
		for _, r in ipairs(ui.rects) do who[#who + 1] = r.pos.x == 0 and "a" or "b" end
		return table.concat(who, ",")
	end
	E.eq(drawn(), "a,b")
	core.current = core.mods_by_id.a
	E.eq(rawget(_G, "SOLO")({ "a" }), true)
	E.eq(drawn(), "a", "only the named mod")
	E.eq(table.concat(rawget(_G, "SOLOED")(), ","), "a")
	E.eq(rawget(_G, "SOLO")({}), true)
	E.eq(drawn(), "", "an empty list holds everyone back")
	E.eq(rawget(_G, "SOLO")(nil), true)
	E.eq(drawn(), "a,b", "and back")
	E.eq(rawget(_G, "SOLOED")(), nil)
	E.eq(select(2, rawget(_G, "SOLO")("a")), "bad_type")
	rawget(_G, "SOLO")({ "a" })
	core.mods_by_id.a.off = true
	e:frames(1)
	E.eq(drawn(), "a,b", "a mod switched off lets the screen go")
end)

return T
