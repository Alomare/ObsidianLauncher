-- What Obsidian API gives a photo mode: a camera a mod takes, lights, time, the look, the ship's music in
-- a mission, screenshots, panels with tabs and sliders, and rows of the pause menu.
local E = require "fake_engine"
local F = require "fake_ui"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local function start(opts)
	local e, game, ui = G.start({ "obsidian.api" }, opts)
	e:core().current = e:core().mods_by_id["obsidian.api"]
	return rawget(_G, "obsidian").import("obsidian.api"), game, e, ui
end

local function near(got, want, what)
	if type(got) ~= "number" or math.abs(got - want) > 0.01 then error((what or "value") .. ": expected " .. want .. ", got " .. tostring(got), 2) end
end

-- The camera of the mission: the viewport's Camera, the manager the state machine updates, and the
-- engine's Camera calls, which record what they were given.
local function camera_world(game)
	local cam = { calls = {}, fov = math.rad(60), far = 300, projection = "perspective" }
	game.cam = cam
	local v3 = rawget(_G, "Vector3")
	rawset(_G, "Camera", {
		PERSPECTIVE = "perspective", ORTHOGRAPHIC = "orthographic",
		world_position = function() return v3(10, 20, 30) end,
		world_rotation = function() return "looking down" end,
		vertical_fov = function() return cam.fov end,
		far_range = function() return cam.far end,
		set_far_range = function(_, f) cam.far = f end,
		set_local_position = function(_, unit, p) cam.unit, cam.pos = unit, { x = p.x, y = p.y, z = p.z } end,
		set_local_rotation = function(_, _, q) cam.rot = q end,
		set_vertical_fov = function(_, f) cam.fov = f end,
		set_projection_type = function(_, kind) cam.projection = kind end,
		set_orthographic_view = function(_, ...) cam.ortho = { ... } end,
	})
	local Q = setmetatable({
		forward = function(q)
			if q == "looking down" then return v3(0, math.cos(math.rad(45)), -math.sin(math.rad(45))) end
			return v3(0, 1, 0)
		end,
		multiply = function(a, b) return { "times", a, b } end,
		identity = function() return "identity" end,
	}, { __call = function(_, axis, angle) return { axis = axis, angle = angle } end })
	rawset(_G, "Quaternion", Q)
	rawset(_G, "World", setmetatable({ update_unit = function(_, unit) cam.updated = unit end }, { __index = rawget(_G, "World") }))
	rawset(_G, "ScatterSystem", { move_observer = function(_, _, p) cam.observer = { x = p.x, y = p.y, z = p.z } end })
	local viewport = game.state.viewport
	viewport.camera_unit = "camera unit"
	viewport.get_camera = function() return "the camera" end
	viewport.get_world_proxy = function() return { is_active = function() return true end, get_world = function() return "world" end } end
	viewport.is_active = function() return true end
	local cm = rawget(_G, "CameraManager")
	cm.viewport, cm.scatter_system, cm.observer_id = viewport, "scatter", 7
	cm.updates = 0
	cm.update = function(self) self.updates = self.updates + 1 end
	-- The game's camera manager is defined after boot, so the wrap goes in at the next require.
	rawget(_G, "require")("lua/network/network_global")
	return cam, cm
end

test("a_taken_camera_goes_where_the_mod_sets_it_and_the_game_gets_it_back", function()
	local api, game = start()
	local cam, cm = camera_world(game)
	local pose = api.world.camera.get()
	near(pose.position.z, 30, "where the game's camera is")
	near(pose.yaw, 0, "yaw")
	near(pose.pitch, -45, "pitch")
	near(pose.fov, 60, "fov")
	E.eq(api.world.camera.take(), true)
	E.eq(api.world.camera.taken(), true)
	E.eq(api.world.camera.set({ position = { x = 1, y = 2, z = 3 }, yaw = 90, pitch = -10, roll = 5, fov = 40 }), true)
	cm:update(0)
	E.eq(cm.updates, 0, "the game's own update doesn't run")
	E.eq(cam.unit, "camera unit")
	near(cam.pos.x, 1)
	near(cam.pos.z, 3)
	near(cam.fov, math.rad(40), "fov")
	E.eq(cam.updated, "camera unit", "the camera's node moves at once")
	E.eq(type(cam.observer), "table", "the grass follows")
	E.eq(api.world.camera.set({ ortho = 30, far = 900 }), true)
	cm:update(0)
	E.eq(cam.projection, "orthographic")
	near(cam.ortho[2], 15, "half the width")
	near(cam.far, 900)
	E.eq(api.world.camera.get().ortho, 30)
	E.eq(api.world.camera.release(), true)
	E.eq(cam.projection, "perspective", "back to perspective")
	near(cam.far, 300, "and the game's own reach")
	near(cam.fov, math.rad(60), "and field of view")
	cm:update(0)
	E.eq(cm.updates, 1, "the game moves it again")
end)

test("a_taken_camera_needs_a_mission_and_a_change_of_screen_gives_it_back", function()
	local api, game, e, ui = start()
	local _, cm = camera_world(game)
	E.eq(select(2, api.world.camera.set({ yaw = 1 })), "not_available", "nothing to set before take")
	api.world.camera.take()
	E.eq(select(2, api.world.camera.set("far")), "bad_type")
	E.eq(api.world.camera.set({ pitch = 120, fov = 1 }), true)
	near(api.world.camera.get().pitch, 89.9, "pitch is held short of straight")
	near(api.world.camera.get().fov, 5, "fov is held in range")
	ui.state_object = { viewport = game.state.viewport }
	e:frames(1)
	E.eq(api.world.camera.taken(), false, "the screen changed")
	cm:update(0)
	E.eq(cm.updates, 1)
	ui.state = "main_menu"
	E.eq(select(2, api.world.camera.take()), "not_in_mission")
end)

-- Lights: a hidden beacon flare with its light taken over.
local function light_world(game, lights)
	game.lights = {}
	camera_world(game)
	local units = rawget(_G, "Unit")
	units.set_unit_visibility = function(u, on) u.visible = on end
	units.disable_physics = function(u) u.still = true end
	units.has_light = function(u, name) return lights and name == "red_light" end
	units.num_lights = function() return lights and 1 or 0 end
	units.light = function(u, name) u.light = u.light or { name = name } return u.light end
	units.teleport_local_position = function(u, _, p) u.pos = p end
	units.teleport_local_rotation = function(u, _, q) u.rot = q end
	units.alive = function(u) return u ~= nil and u.alive ~= false end
	local W = rawget(_G, "World")
	W.spawn_unit = function(_, name, pos)
		local u = { name = name, pos = pos, alive = true }
		game.lights[#game.lights + 1] = u
		return u
	end
	W.destroy_unit = function(_, u) u.alive = false end
	W.update_unit = function(_, u) u.settled = (u.settled or 0) + 1 end
	rawset(_G, "Light", setmetatable({ OMNI = "omni", SPOT = "spot" }, { __index = function(t, k)
		return function(l, v) l[k] = v end
	end }))
end

test("a_light_is_a_hidden_flare_with_its_light_set", function()
	local api, game = start()
	light_world(game, true)
	local light = api.world.light({ x = 1, y = 2, z = 3 }, { color = { 1, 0.5, 0 }, intensity = 4, range = 12, shadows = true })
	E.eq(type(light), "table")
	local u = game.lights[1]
	E.eq(u.name, "gameobjects/flare_01")
	E.eq(u.visible, false, "the flare isn't seen")
	E.eq(u.still, true, "and doesn't fall")
	E.eq(u.light.set_enabled, true)
	near(u.light.set_intensity, 4)
	near(u.light.set_falloff_end, 12)
	E.eq(u.light.set_casts_shadows, true)
	near(u.light.set_color.y, 0.5, "green")
	E.eq(light:set({ spot = 60, yaw = 90, pitch = -30 }), true)
	E.eq(u.light.set_type, "spot")
	near(u.light.set_spot_angle_end, math.rad(30), "half the cone")
	near(u.light.set_intensity, 4, "the rest stays")
	E.eq(light:move({ x = 5, y = 5, z = 5 }), true)
	E.eq(u.pos.x, 5)
	E.eq(u.settled > 0, true, "moved at once in a stopped world")
	light:remove()
	E.eq(u.alive, false)
	E.eq(light:alive(), false)
	E.eq(light:set({ intensity = 1 }), false)
end)

test("a_flare_without_a_light_is_taken_away", function()
	local api, game = start()
	light_world(game, false)
	local light, why = api.world.light({ x = 1, y = 2, z = 3 })
	E.eq(light, nil)
	E.eq(why, "no_light")
	E.eq(game.lights[1].alive, false)
	E.eq(select(2, api.world.light("here")), "bad_type")
end)

test("time_stops_and_slows_only_alone_and_comes_back_with_a_new_screen", function()
	local api, game, e, ui = start()
	E.eq(api.world.time_scale(0), true)
	E.eq(rawget(_G, "PAUSED"), true)
	E.eq(api.world.time_now(), 0)
	E.eq(api.world.time_scale(0.25), true)
	E.eq(rawget(_G, "PAUSED"), false)
	E.eq(rawget(_G, "TIME_SCALE"), 0.25)
	E.eq(api.world.time_scale(1), true)
	E.eq(rawget(_G, "TIME_SCALE"), 1)
	E.eq(select(2, api.world.time_scale(2)), "out_of_range")
	api.world.time_scale(0)
	ui.state_object = {}
	e:frames(1)
	E.eq(rawget(_G, "PAUSED"), false, "a new screen runs")
	E.eq(api.world.time_now(), 1)
	local lobby = rawget(_G, "NetworkGlobal").lobby
	lobby.members = function() return { "me", "friend" } end
	E.eq(select(2, api.world.time_scale(0)), "solo_only")
end)

test("the_time_of_day_blends_and_goes_back_to_the_missions", function()
	local api, game = start()
	local got = nil
	game.state.viewport.set_shading_environment_settings = function(_, ...) got = { ... } end
	E.eq(api.world.time_of_day("dusk", "night", 0.25), true)
	E.eq(table.concat({ got[1], got[2], got[3], got[4] }, " "), "dusk 1 night 0.25")
	E.eq(api.world.time_of_day("day"), true)
	E.eq(#got, 2)
	E.eq(select(2, api.world.time_of_day("swamp")), "wrong_environment", "a desert has no swamp")
	game.state.generated_level.environment_name = "forest"
	E.eq(api.world.time_of_day("swamp_dusk"), true)
	E.eq(api.world.time_of_day(nil), true)
	E.eq(got[1], "dusk", "the mission's own")
end)

test("units_hide_and_flashlights_light_on_this_machine", function()
	local api, game = start()
	local units = rawget(_G, "Unit")
	units.set_unit_visibility = function(u, on) u.visible = on end
	local events = {}
	units.flow_event = function(_, name) events[#events + 1] = name end
	E.eq(api.world.visible(game.me, false), true)
	E.eq(game.me.visible, false)
	E.eq(api.world.visible(game.me, true), true)
	E.eq(game.me.visible, true)
	E.eq(api.world.flashlight(game.me, true), true)
	E.eq(api.world.flashlight(game.me, false), true)
	E.eq(table.concat(events, " "), "flashlight_on flashlight_off")
	E.eq(select(2, api.world.visible(nil, false)), "no_target")
end)

test("the_shaders_named_values_are_taken", function()
	local api, game = start()
	E.eq(api.world.shading("vignette", "vector3", { 1, 0.6, 10 }), true)
	E.eq(game.shadings.vignette.kind, "vector3")
	E.eq(api.world.shading("emissive_material_intensity", "scalar", 2), true)
	E.eq(api.world.shading("dust_color", "vector3", { 1, 0, 0 }), true)
	E.eq(select(2, api.world.shading("saturation", "scalar", 1)), "unknown_shading")
end)

-- The ship's music in a mission: its package loads first, the game's music stays down meanwhile, and the
-- track starts at its own level once the package is there.
test("a_menu_track_loads_its_package_in_a_mission_and_plays_at_its_level", function()
	local api, game, e = start()
	local T = rawget(_G, "TimpaniWorld")
	local machine = game.state.audio_state_machine
	machine.timpani_world = "sound world"
	machine.music_threat = T.trigger_event("sound world", "music_threat")
	local ok, why = api.music.play("music_bridge", { db = -6 })
	E.eq(ok, true)
	E.eq(why, "loading")
	E.eq(game.packages["resource_packages/menu"], true, "asked for")
	E.eq(game.flushed, false, "in the background")
	game.packages["resource_packages/menu"] = false
	T.set_parameter("sound world", machine.music_threat, "volume", 1)
	e:frames(1)
	E.eq(game.params[machine.music_threat].volume, 0, "the game's music stays down while it loads")
	E.eq(api.music.now().loading, true)
	game.packages["resource_packages/menu"] = true
	game.events["sound world"].music_bridge = true
	e:frames(1)
	local id = nil
	for i, p in ipairs(game.played) do
		if p.name == "music_bridge" then id = i end
	end
	E.eq(id ~= nil, true, "it plays")
	near(game.params[id].volume, 10 ^ (-6 / 20), "at -6 dB")
	E.eq(api.music.now().loading, false)
	E.eq(select(2, api.music.play("music_bridge", { db = 3 })), "out_of_range")
	E.eq(select(2, api.music.play("music_nothing")), "unknown_event")
	api.music.release()
	E.eq(game.playing[id], nil)
end)

test("a_shot_without_the_ffi_is_refused_and_says_why", function()
	local api = start()
	rawset(_G, "HAS_FFI", false)
	local capture = api.capture
	capture.reset()
	local ok, why = capture.shot({}, function() end)
	E.eq(ok, false)
	E.eq(why, "no_ffi")
	E.eq(capture.busy(), false)
	E.eq(select(2, capture.available()), "no_ffi")
	E.eq(select(2, capture.folder()), "no_ffi")
	E.eq(select(2, capture.open_folder("C:\\")), "no_ffi")
end)

test("a_shot_cuts_the_middle_for_an_aspect_and_a_crop_from_shares", function()
	local api = start()
	local capture = api.capture
	local x, y, w, h = capture.area(1920, 1080, { aspect = 1 })
	E.eq(table.concat({ x, y, w, h }, " "), "420 0 1080 1080")
	x, y, w, h = capture.area(1000, 1000, { aspect = 2 })
	E.eq(table.concat({ x, y, w, h }, " "), "0 250 1000 500")
	x, y, w, h = capture.area(100, 100, { crop = { x = 0.1, y = 0.2, w = 0.5, h = 0.5 } })
	E.eq(table.concat({ x, y, w, h }, " "), "10 20 50 50")
	x, y, w, h = capture.area(100, 100, {})
	E.eq(table.concat({ x, y, w, h }, " "), "0 0 100 100")
end)

-- Panels, through a mod, as a player drives them.
local function start_mod(source)
	local mods = {}
	local api = G.bundled("obsidian.api", mods)
	mods["obsidian/mods/m/main"] = source
	local deploy = { E.mod("obsidian.api", { scope = "client", texts = api.texts, keybinds = api.keybinds, name = api.name }),
		E.mod("m", { scope = "client", requires = { "obsidian.api" }, name = { en = "Test Mod" } }) }
	local e = E.new({ deploy = E.deploy({ mods = deploy, has_session = false }), mods = mods })
	e:boot()
	e:start_frames()
	local ui = F.install(e, {})
	ui.frame()
	return e, ui
end

local PANEL = [[
	local api = obsidian.import("obsidian.api")
	VALUES = { fov = 60, mode = "free", closed = 0, dof = false }
	PANEL = api.menu.panel({ title = "Photo Mode", status = function() return "PAUSED" end,
		guide = { { keys = { "H" }, label = "Hide" } },
		on_close = function() VALUES.closed = VALUES.closed + 1 VALUES.closer = obsidian.caller() end,
		tabs = {
			{ title = "Camera", build = function(page)
				page:choice("Mode", { choices = { { id = "free", label = "Free" }, { id = "orbit", label = "Orbit", swatch = { 255, 0, 0 } } },
					get = function() return VALUES.mode end, set = function(v) VALUES.mode = v page:refresh() end })
				page:slider("Field of view", { min = 20, max = 120, step = 5, format = "%d deg", default = 60,
					get = function() return VALUES.fov end, set = function(v) VALUES.fov = v end })
				if VALUES.mode == "orbit" then page:slider("Distance", { min = 1, max = 30, get = function() return 5 end, set = function() end }) end
			end },
			{ title = "Lens", build = function(page)
				page:toggle("Depth of field", { get = function() return VALUES.dof end, set = function(v) VALUES.dof = v end })
			end },
		} })
]]

local function values() return rawget(_G, "VALUES") end
local function panel() return rawget(_G, "PANEL") end
local function menu_of(e) return e.loaded["obsidian/mods/obsidian.api/lib/menu"] end

local function row_named(e, label)
	for _, row in ipairs(menu_of(e).hits.rows) do
		if row.it.label == label then return row end
	end
	error("no row: " .. label, 2)
end

test("a_panel_opens_on_its_tabs_and_esc_closes_it_as_its_mod", function()
	local e, ui = start_mod(PANEL)
	E.eq(panel():open(), true)
	ui.frame()
	E.eq(ui.shown("Photo Mode"), true, "the title")
	E.eq(ui.shown("PAUSED"), true, "the status")
	E.eq(ui.shown("Field of view"), true, "the first tab's rows")
	E.eq(ui.shown("60 deg"), true, "a slider's value")
	E.eq(ui.shown("Hide"), true, "the mod's keys in the guide")
	local api = rawget(_G, "obsidian").import("obsidian.api")
	E.eq(api.menu.open(), false, "the F6 window waits")
	ui.tap("tab")
	ui.frame()
	E.eq(panel():tab_number(), 2)
	E.eq(ui.shown("Depth of field"), true, "the second tab")
	ui.tap("enter")
	E.eq(values().dof, true)
	ui.tap("esc")
	E.eq(panel():is_open(), false)
	E.eq(values().closed, 1)
	E.eq(values().closer, "m", "on_close runs as the mod")
	E.eq(api.menu.open(), true, "the window opens again")
end)

test("a_panel_leaves_wasd_and_the_right_button_to_its_mod", function()
	local e, ui = start_mod(PANEL)
	panel():open()
	ui.frame()
	ui.tap("s")
	ui.tap("d")
	E.eq(values().mode, "free", "D changes nothing")
	ui.tap("down")
	ui.tap("right")
	E.eq(values().fov, 65, "the arrows step the slider")
	local row = row_named(e, "Mode")
	local kit = menu_of(e).kit
	ui.click(kit.ox + (row.x + 30) * kit.scale, kit.oy + (row.y + 5) * kit.scale, "right")
	E.eq(panel():is_open(), true, "the right button doesn't close it")
end)

test("a_slider_follows_the_mouse_along_its_bar_on_its_steps", function()
	local e, ui = start_mod(PANEL)
	panel():open()
	ui.frame()
	local kit = menu_of(e).kit
	local track = row_named(e, "Field of view").track
	local y = kit.oy + (track.y + track.h / 2) * kit.scale
	ui.press(kit.ox + track.left * kit.scale, y)
	E.eq(values().fov, 20, "the bar's start")
	ui.move(kit.ox + (track.left + track.width) * kit.scale, y)
	E.eq(values().fov, 120, "dragged to the end")
	ui.move(kit.ox + (track.left + track.width * 0.52) * kit.scale, y)
	E.eq(values().fov, 70, "on a step")
	ui.release()
	ui.move(kit.ox + track.left * kit.scale, y)
	E.eq(values().fov, 70, "let go")
end)

test("a_panel_lists_again_when_a_row_asks_and_shows_a_choices_swatch", function()
	local e, ui = start_mod(PANEL)
	panel():open()
	ui.frame()
	E.eq(ui.shown("Distance"), false)
	ui.tap("right")
	ui.frame()
	E.eq(values().mode, "orbit")
	E.eq(ui.shown("Distance"), true, "the tab built again")
	local red = false
	for _, r in ipairs(ui.rects) do
		if r.color[2] == 255 and r.color[3] == 0 and r.color[4] == 0 then red = true end
	end
	E.eq(red, true, "the swatch")
end)

test("a_hidden_panel_draws_nothing_and_a_shot_holds_every_panel_back", function()
	local e, ui = start_mod(PANEL)
	panel():open()
	panel():hide(true)
	ui.frame()
	E.eq(ui.shown("Photo Mode"), false)
	ui.tap("esc")
	E.eq(panel():is_open(), true, "a hidden panel reads no keys")
	panel():hide(false)
	menu_of(e).held_back = true
	ui.frame()
	E.eq(ui.shown("Photo Mode"), false, "held back for a shot")
	menu_of(e).held_back = false
	ui.frame()
	E.eq(ui.shown("Photo Mode"), true)
	E.eq(panel():hidden(), false)
end)

-- Where a point of the open panel is on screen, in pixels: the panel scales around the corner it's kept at.
local function screen_at(e, x, y)
	local M = menu_of(e)
	local p, kit = M.panel_on, M.kit
	local s, ax, ay = p.scale_now, p.place.x, p.place.y
	return kit.ox + (ax + (x - ax) * s) * kit.scale, kit.oy + (ay + (y - ay) * s) * kit.scale
end

local function middle_of(e, area) return screen_at(e, area.x + area.w / 2, area.y + area.h / 2) end

-- The screen in the kit's units.
local function visible_of(e)
	local kit = menu_of(e).kit
	local left, bottom = kit:point(0, 0)
	local right, top = kit:point(1920, 1080)
	return left, bottom, right, top
end

local function count_texts(ui, pattern)
	local n = 0
	for _, t in ipairs(ui.texts) do
		if string.find(t.text, pattern) then n = n + 1 end
	end
	return n
end

local HELP_WORDS = {}
for i = 1, 120 do HELP_WORDS[i] = "w" .. i end

local LONG_PANEL = [[
	local api = obsidian.import("obsidian.api")
	VALUES = { reset = 0 }
	PANEL = api.menu.panel({ title = "Long", id = "long",
		buttons = { { glyph = "reset", help = "Puts every value back.", run = function()
			VALUES.reset = VALUES.reset + 1
			VALUES.by = obsidian.caller()
			return "All back"
		end } },
		tabs = {
			{ title = "Camera", icon = "weapons", build = function(page)
				for i = 1, 30 do
					page:toggle("Row " .. i .. ".", { get = function() return false end, set = function() end, help = "]] .. table.concat(HELP_WORDS, " ") .. [[" })
				end
			end },
			{ title = "Lens", icon = "search", build = function(page) page:label("Glass") end },
			{ title = "Scene", icon = "no_such_icon", build = function(page) page:label("Here") end },
		} })
]]

local function relaunch(e, p)
	e.loaded["obsidian/loader/storage"].loaded = {}
	p.loaded, p.place, p.alpha_now, p.scale_now, p.foot = false, nil, 0.5, 1, 0
end

test("a_new_panel_sits_at_the_bottom_right_three_wide_by_two_high", function()
	local e, ui = start_mod(PANEL)
	panel():open()
	ui.frame()
	local p = panel()
	local _, bottom, right = visible_of(e)
	near(p.screen.x + p.screen.w, right - 18, "at the right edge, inside the margin")
	near(p.screen.top - p.screen.h, bottom + 18, "at the bottom")
	near(p.box.w / p.box.h, 1.5, "three wide by two high")
	E.eq(p:opacity(), 0.5)
	E.eq(p:scale(), 1)
	local grip = menu_of(e).hits.grip
	near(grip.x, p.box.x, "the grip in the corner across from the anchor's")
	near(grip.y + grip.h, p.box.top)
end)

test("a_panel_drags_by_the_title_resizes_from_the_grip_and_stays_on_screen", function()
	local e, ui = start_mod(PANEL)
	panel():open()
	ui.frame()
	local M, p = menu_of(e), panel()
	local left, bottom, right, top = visible_of(e)
	local x, y = middle_of(e, M.hits.title)
	ui.press(x, y)
	ui.move(x - 150, y + 60)
	near(p.screen.x + p.screen.w, right - 18 - 100, "moved left")
	near(p.screen.top - p.screen.h, bottom + 18 + 40, "and up")
	ui.move(9000, -9000)
	near(p.screen.x + p.screen.w, right, "stopped at the right edge")
	near(p.screen.top - p.screen.h, bottom, "stopped at the bottom edge")
	ui.release()
	E.eq(M.drag, nil)
	local w, h, corner_x, corner_y = p.box.w, p.box.h, p.place.x, p.place.y
	x, y = middle_of(e, M.hits.grip)
	ui.press(x, y)
	ui.move(x - 60, y + 45)
	near(p.box.w, w + 40, "wider")
	near(p.box.h, h + 30, "taller")
	near(p.place.x, corner_x, "the kept corner stays put")
	near(p.place.y, corner_y)
	ui.move(-9000, 9000)
	near(p.screen.x, left, "not wider than the screen")
	near(p.screen.top, top, "nor taller")
	ui.move(9000, -9000)
	E.eq(p.box.w, 300, "not narrower than the rows need")
	E.eq(p.box.h, 200)
	ui.release()
end)

test("a_panel_keeps_its_place_size_help_room_opacity_and_scale_for_the_next_launch", function()
	local e, ui = start_mod(LONG_PANEL)
	local p = panel()
	p:open()
	ui.frame()
	local M = menu_of(e)
	local x, y = middle_of(e, M.hits.title)
	ui.press(x, y)
	ui.move(x - 150, y + 60)
	ui.release()
	x, y = middle_of(e, M.hits.divider)
	ui.press(x, y)
	ui.move(x, y + 30)
	ui.release()
	E.eq(p:set_opacity(0.3), true)
	E.eq(p:set_scale(1.2), true)
	ui.frame()
	local place, foot = { x = p.place.x, y = p.place.y, w = p.place.w, h = p.place.h }, p.foot
	p:close()
	relaunch(e, p)
	p:open()
	ui.frame()
	near(p.place.x, place.x, "the place")
	near(p.place.y, place.y)
	near(p.place.w, place.w, "the size")
	near(p.place.h, place.h)
	near(p.foot, foot, "the help's room")
	near(p:opacity(), 0.3, "the opacity")
	near(p:scale(), 1.2, "the scale")
	local core = e:core()
	local was = core.current
	core.current = core.mods_by_id["obsidian.api"]
	E.eq(type(rawget(_G, "obsidian").storage.get("menu/panel/m/long")), "string", "in Obsidian API's storage, by mod and id")
	core.current = was
	E.eq(p:reset_place(), true)
	ui.frame()
	local _, bottom, right = visible_of(e)
	near(p.screen.x + p.screen.w, right - 18, "back in the corner")
	near(p.screen.top - p.screen.h, bottom + 18)
	near(p:opacity(), 0.3, "the opacity stays")
end)

test("the_wheel_scrolls_a_panel_only_with_the_mouse_over_it", function()
	local e, ui = start_mod(LONG_PANEL)
	panel():open()
	ui.frame()
	local M = menu_of(e)
	ui.move(middle_of(e, M.hits.rows[2]))
	E.eq(ui.shown("Row 1."), true)
	ui.scroll(-1)
	E.eq(ui.shown("Row 1."), false, "scrolled")
	E.eq(ui.shown("Row 4."), true, "by three rows")
	local top = M.stack[#M.stack].top
	ui.move(5, 5)
	ui.scroll(-1)
	E.eq(M.stack[#M.stack].top, top, "the wheel away from the panel is the mod's")
end)

test("a_panels_background_takes_the_opacity_and_the_words_stay_whole", function()
	local e, ui = start_mod(PANEL)
	local p = panel()
	p:open()
	ui.frame()
	local function body_alpha()
		for _, t in ipairs(ui.triangles) do
			if t.color[2] == 41 and t.color[3] == 44 and t.color[4] == 48 then return t.color[1] end
		end
	end
	near(body_alpha(), 125, "half of the background at first")
	E.eq(ui.find("Field of view").color[1], 255, "the rows at full strength")
	p:set_opacity(0.2)
	ui.frame()
	near(body_alpha(), 50)
	p:set_opacity(5)
	ui.frame()
	near(body_alpha(), 250, "held at 1")
	E.eq(select(2, p:set_opacity("half")), "bad_type")
end)

test("a_scaled_panel_grows_from_its_corner_and_the_mouse_follows", function()
	local e, ui = start_mod(PANEL)
	local p = panel()
	p:open()
	ui.frame()
	local kit = menu_of(e).kit
	local right, bottom = p.screen.x + p.screen.w, p.screen.top - p.screen.h
	local size = ui.find("Photo Mode").size
	E.eq(p:set_scale(1.5), true)
	ui.frame()
	near(p.screen.x + p.screen.w, right, "the corner stays")
	near(p.screen.top - p.screen.h, bottom)
	near(p.screen.w, p.box.w * 1.5, "half again as wide")
	near(ui.find("Photo Mode").size, size * 1.5, "and the words as big")
	near(size, 21 * kit.scale)
	local row = row_named(e, "Mode")
	ui.click(middle_of(e, row.plus))
	E.eq(values().mode, "orbit", "a click lands on the scaled row")
	p:set_scale(3)
	E.eq(p:scale(), 1.6)
	p:set_scale(0.1)
	E.eq(p:scale(), 0.6)
end)

test("icon_tabs_show_the_icon_and_the_title_over_the_help", function()
	local e, ui = start_mod(LONG_PANEL)
	panel():open()
	ui.frame()
	local M = menu_of(e)
	E.eq(M.hits.tabs[1].icon, true, "a glyph draws")
	E.eq(M.hits.tabs[3].icon, false, "an unknown icon doesn't")
	E.eq(ui.shown("Scene"), true, "so that tab's title shows instead")
	E.eq(ui.shown("Lens"), false, "an icon tab shows no title")
	E.eq(ui.shown("Camera"), true, "the open tab's title, over the help")
	ui.move(middle_of(e, M.hits.tabs[2]))
	E.eq(ui.shown("Lens"), true, "the title of the tab under the mouse")
	E.eq(ui.shown("Camera"), false)
	ui.click(middle_of(e, M.hits.tabs[2]))
	ui.move(5, 5)
	E.eq(panel():tab_number(), 2)
	E.eq(ui.shown("Lens"), true, "the open tab")
end)

test("a_header_button_runs_as_the_mod_and_shows_its_help", function()
	local e, ui = start_mod(LONG_PANEL)
	panel():open()
	ui.frame()
	local M = menu_of(e)
	local button = M.hits.buttons[1]
	E.eq(button.x + button.w <= M.hits.close.x, true, "left of the close button")
	ui.move(middle_of(e, button))
	E.eq(ui.shown("Puts every value back."), true, "the help, in the foot")
	ui.click(middle_of(e, button))
	ui.frame()
	E.eq(values().reset, 1)
	E.eq(values().by, "m", "as the mod")
	E.eq(ui.shown("All back"), true, "an answer becomes a toast")
	E.eq(panel():is_open(), true)
end)

test("the_help_fits_four_lines_and_the_line_over_it_drags", function()
	local e, ui = start_mod(LONG_PANEL)
	local p = panel()
	p:open()
	ui.frame()
	local M = menu_of(e)
	E.eq(count_texts(ui, "^w%d+"), 3, "four lines: the tab's title, then three of help")
	local rows = #M.hits.rows
	local x, y = middle_of(e, M.hits.divider)
	ui.press(x, y)
	ui.move(x, y + 48)
	ui.release()
	E.eq(count_texts(ui, "^w%d+"), 5, "two lines more")
	E.eq(#M.hits.rows < rows, true, "and fewer rows")
	ui.press(middle_of(e, M.hits.divider))
	ui.move(x, -9000)
	ui.release()
	E.eq(count_texts(ui, "^w%d+"), 0, "one line at the least, the tab's title")
	E.eq(ui.shown("Camera"), true)
	ui.press(middle_of(e, M.hits.divider))
	ui.move(x, 9000)
	ui.release()
	E.eq(#M.hits.rows, 2, "two rows at the least")
end)

test("a_hidden_cursor_stays_hidden_and_the_panel_leaves_the_mouse", function()
	local e, ui = start_mod(PANEL)
	local p = panel()
	p:open()
	ui.frame()
	E.eq(ui.cursor.shown, true)
	E.eq(p:set_cursor_hidden(true), true)
	ui.frame()
	ui.frame()
	E.eq(ui.cursor.shown, false, "not shown again each frame")
	E.eq(p:cursor_hidden(), true)
	local row = row_named(e, "Mode")
	ui.click(middle_of(e, row.plus))
	E.eq(values().mode, "free", "a click goes to the mod")
	E.eq(p:under_mouse(), false)
	p:set_cursor_hidden(false)
	ui.frame()
	E.eq(ui.cursor.shown, true, "back")
	E.eq(ui.cursor.name, "hud/cursors/menu")
	p:set_cursor_hidden(true)
	p:close()
	E.eq(p:cursor_hidden(), false, "closing brings it back")
end)

test("a_panel_down_the_left_still_docks_there_full_height", function()
	local e, ui = start_mod([[
		local api = obsidian.import("obsidian.api")
		PANEL = api.menu.panel({ title = "Side", side = "left", tabs = { { title = "One", build = function(page) page:label("Text") end } } })
	]])
	local p = panel()
	p:open()
	ui.frame()
	local left, bottom, _, top = visible_of(e)
	near(p.screen.x, left + 18, "at the left")
	near(p.screen.top, top - 18, "from the top")
	near(p.screen.h, top - bottom - 36, "the full height")
	near(p.box.w, 390)
	local grip = menu_of(e).hits.grip
	near(grip.x + grip.w, p.box.x + p.box.w, "the grip at the bottom right")
	near(grip.y, p.box.top - p.box.h)
end)

-- The sound world of the world the engine keeps for drawing the screen, which records what plays.
local function ui_sounds(has)
	local played = {}
	rawset(_G, "TimpaniWorld", {
		has_event = function(world, name) return world == "ui sounds" and has ~= false and name:sub(1, 3) == "ui_" end,
		trigger_event = function(world, name)
			played[#played + 1] = { world = world, name = name }
			return #played
		end,
		set_parameter = function(_, id, name, value) played[id][name] = value end,
	})
	rawget(_G, "World").timpani_world = function(w) return w == "brightness" and "ui sounds" or nil end
	return played
end

local function wait(ui, n)
	for _ = 1, n or 6 do ui.frame() end
end

test("a_panel_plays_the_games_menu_sounds_quietly", function()
	local e, ui = start_mod(PANEL)
	local played = ui_sounds()
	panel():open()
	ui.frame()
	E.eq(played[1].name, "ui_select", "opening")
	E.eq(played[1].world, "ui sounds", "in the world that runs while a mission is paused")
	near(played[1].volume, 0.35, "quietly")
	wait(ui)
	ui.tap("down")
	ui.frame()
	E.eq(played[#played].name, "ui_move", "moving")
	wait(ui)
	local n = #played
	ui.tap("right")
	ui.tap("right")
	ui.frame()
	E.eq(values().fov, 70, "two steps")
	E.eq(#played, n + 1, "one sound for steps closer together than 0.06 s")
	E.eq(played[#played].name, "ui_move")
	wait(ui)
	ui.tap("tab")
	ui.frame()
	E.eq(played[#played].name, "ui_move", "a new tab")
	ui.tap("esc")
	ui.frame()
	E.eq(played[#played].name, "ui_back", "closing")
	rawset(_G, "TimpaniWorld", nil)
end)

test("a_panel_with_sounds_off_plays_none_and_a_missing_event_plays_nothing", function()
	local e, ui = start_mod([[
		local api = obsidian.import("obsidian.api")
		PANEL = api.menu.panel({ title = "Quiet", sounds = false, tabs = {
			{ title = "One", build = function(page) page:toggle("A", { get = function() return false end, set = function() end }) page:toggle("B", {}) end } } })
		LOUD = api.menu.panel({ title = "Loud", tabs = { { title = "One", build = function(page) page:label("Text") end } } })
	]])
	local played = ui_sounds()
	panel():open()
	ui.frame()
	ui.tap("down")
	ui.frame()
	ui.tap("esc")
	ui.frame()
	E.eq(#played, 0, "sounds = false")
	local none = ui_sounds(false)
	rawget(_G, "LOUD"):open()
	ui.frame()
	E.eq(rawget(_G, "LOUD"):is_open(), true)
	E.eq(#none, 0, "a sound world without the event")
	rawset(_G, "TimpaniWorld", nil)
end)

-- The pause menu, as test_api.lua fakes it, with the state that owns it.
local function fake_pause_menu()
	local lm = rawget(_G, "LocalizationManager") or { string = function(_, k) return "LOC:" .. k end }
	rawset(_G, "LocalizationManager", lm)
	local Base = { was_option_selected = function(_, input) return input.select == true end }
	local Main = {}
	function Main.on_enter(self)
		self.options = { { name = "menu_keybindings" }, { name = "menu_settings" }, { name = "menu_exit_to_menu" } }
		self.selected_option, self.states, self.state_stack = 1, self.states or {}, {}
	end
	function Main.update(self)
		self.drawn = {}
		for i, o in ipairs(self.options) do self.drawn[i] = lm:string(o.name) end
	end
	function Main.handle_input(self, input)
		if Base.was_option_selected(self, input) then self.chose = self.options[self.selected_option].name end
	end
	function Main.play_sound_accept() end
	rawset(_G, "MenuScreenBase", Base)
	rawset(_G, "MenuScreenOptions", setmetatable({}, { __call = function() return {} end }))
	rawset(_G, "MenuScreenMain", Main)
	rawget(_G, "require")("lua/network/network_global")
	return Main
end

test("a_pause_row_closes_the_pause_menu_then_runs_as_its_mod", function()
	local e, ui = start_mod([[
		local api = obsidian.import("obsidian.api")
		RAN = {}
		api.mod_settings.pause_row("photo", { en = "Photo Mode", fr = "Mode photo" }, function() RAN[#RAN + 1] = obsidian.caller() end,
			{ when = function(screen) return screen == "game" end })
	]])
	local Main = fake_pause_menu()
	local lm = rawget(_G, "LocalizationManager")
	local real_string = lm.string
	local parent = {}
	local main = setmetatable({ parent = parent, state_name = "game" }, { __index = Main })
	parent.active_screen = main
	parent.pop_menu = function(self) self.active_screen, self.popped = nil, true end
	main:on_enter()
	E.eq(main.options[3].name, "obsidian_pause_row_1", "under Settings, with no Mods row")
	main:update()
	E.eq(main.drawn[3], "PHOTO MODE", "in capitals, as the game's rows")
	main.selected_option = 3
	main:handle_input({ select = true })
	E.eq(main.chose, nil, "the pause menu didn't act on it")
	E.eq(#rawget(_G, "RAN"), 0, "not inside the menu's own input")
	ui.frame()
	E.eq(parent.popped, true, "the pause menu closed")
	E.eq(rawget(_G, "RAN")[1], "m", "then the row ran as its mod")
	local ship = setmetatable({ parent = {}, state_name = "menu" }, { __index = Main })
	ship:on_enter()
	E.eq(#ship.options, 3, "not on the ship")
	lm.string = real_string
end)

-- The ship sets its own camera (52 degrees, 10 m): a change and a reset give back exactly that, not a
-- mission's values.
test("camera_effects_give_back_the_cameras_own_values_on_each_screen", function()
	local api, game = start()
	local cm = rawget(_G, "CameraManager")
	cm.fov, cm.follow_distance, cm.max_follow_distance, cm.min_follow_distance, cm.follow_angle = math.rad(52), 10, 10, 10, math.rad(40)
	cm.change_settings = function(self, fov, distance, angle)
		if distance then self.follow_distance, self.max_follow_distance, self.min_follow_distance = distance, distance, distance end
		if angle then self.follow_angle = math.rad(angle) end
	end
	near(api.world.camera.defaults().fov, 52, "the game's own")
	api.world.camera.fov(90)
	near(math.deg(cm.fov), 90)
	near(api.world.camera.defaults().fov, 52, "still the game's own while changed")
	api.effects.set("camera_distance", 30)
	api.effects.set("camera_angle", 70)
	E.eq(cm.follow_distance, 30)
	api.world.camera.fov(nil)
	api.effects.set("camera_distance", false)
	api.effects.set("camera_angle", false)
	near(math.deg(cm.fov), 52, "the ship's field of view back")
	E.eq(cm.follow_distance, 10, "and distance")
	E.eq(cm.min_follow_distance, 10)
	near(math.deg(cm.follow_angle), 40, "and angle")
end)

test("the_camera_can_be_taken_on_the_ship_and_goes_back_to_the_ships_own_state", function()
	local api, game, e, ui = start()
	local cam, cm = camera_world(game)
	ui.state = "menu"
	cm.state = "follow_menu"
	E.eq(api.world.camera.take(), true, "on the ship")
	E.eq(api.world.camera.photo(true), true)
	E.eq(cm.state, "fully_static")
	E.eq(api.world.camera.photo(false), true)
	E.eq(cm.state, "follow_menu", "the ship's own state back")
	api.world.camera.release()
	ui.state = "main_menu"
	E.eq(select(2, api.world.camera.take()), "not_in_mission", "not on the main menu")
end)

return T
