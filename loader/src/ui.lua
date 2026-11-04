-- obsidian.ui: the tools to replace the game's 2D screens. It's in the loader, not in Obsidian API, so
-- a pure reskin stays a client mod: requiring a session mod would cost it the Galactic War and
-- matchmaking. Nothing here runs until a mod asks for it.
-- Three things from the game's code shape this file. Its update and draw functions carry logic, so
-- they always run, and only their output is cut, at the primitives of HdGui and GuiDrawer. A mod gets
-- copies of the game's tables, never the tables. And an engine function can kill the process past any
-- pcall, so every call here has its shape proven in a run or shipped in a script, named beside it.
-- What isn't proven yet stays behind CLEARED, and internal/testing/ui-probe2 is how it gets proven.
local B = require("obsidian/loader/builtins")
local glyphs = require("obsidian/loader/glyphs")
local log = require("obsidian/loader/log")

local upper, cos, sin, pi = string.upper, math.cos, math.sin, math.pi

-- Each was held back until a run of internal/testing/ui-probe2 proved it, on 2026-10-31. mission_draw:
-- the canvas in the game state. icons: hud/hud on a gui of the brightness world, which only the cursor's
-- material has shipped on. Muting the whole HUD passed in the same run.
local CLEARED = { mission_draw = true, icons = true }

local MENU_LAYER = 876
local DEFAULT_LAYER = 700
local DESIGN_W, DESIGN_H = 1280, 720
local SAFE_X, SAFE_Y = 96, 86
local WHITE = { 255, 255, 255 }
local NOBODY = {}
local GRADIENT = "hud_icon_color_gradient_01"

-- Sub-materials hash-checked in the resources (ui-input.md 1.1). FontSettings names the same ones.
local FONTS = {
	debug = { path = "foundation/fonts/debug_small", material = "debug_small", size = 16 },
	debug_medium = { path = "foundation/fonts/debug_medium", material = "debug_medium", size = 32 },
	body = { path = "fonts/handel_small", material = "handel_small", size = 18, gui = "fonts" },
	body_large = { path = "fonts/handel_normal", material = "handel_normal", size = 22, gui = "fonts" },
	heading = { path = "fonts/insignia_headliner", material = "insignia_headliner", size = 42, gui = "fonts" },
	heading_large = { path = "fonts/insignia_headliner_large", material = "insignia_headliner_large", size = 64, gui = "fonts" },
	chat = { path = "fonts/chat", material = "chat", size = 30, gui = "fonts" },
	-- Baked by tools/bake_font.py and shipped with the loader. A deploy without them gets the game's.
	ui = { path = "obsidian/fonts/body", material = "body", size = 24, gui = "fonts", instead = "body" },
	ui_heading = { path = "obsidian/fonts/heading", material = "heading", size = 24, gui = "fonts", instead = "heading" },
}

-- Immediate guis on the brightness world: it's the only world that outlives a state change, so a gui on
-- it can't go stale. Each package never unloads once online (abstract_base_state.lua:60); a gui listing
-- a material that isn't loaded is assumed fatal, so it's made only after is_package_online says so.
-- menu/menu is left out: it unloads between states. Shapes and text share a gui once the fonts are
-- there, as layers aren't proven across guis. state_cutscene.lua:36 lists two materials the same way.
local GUIS = {
	shapes = { materials = { "foundation/gui/gui" } },
	fonts = { materials = { "foundation/gui/gui", "fonts/fonts" }, package = "resource_packages/localization" },
	icons = { materials = { "hud/hud" }, package = "resource_packages/common" },
}

-- Names the HUD draws in a mission, where its gui lists only hud/hud and fonts/fonts (hud.lua:83). A
-- name a material doesn't have is assumed fatal, so only these and the game's own stratagem and rank
-- icons reach the engine.
local ICONS = {}
for name in B.gmatch([[
	basicbox_01 distractor_ping galcamp_scanlines hud_icon_color_gradient_01 hud_map_dottedlines_01 icon_arrow_01
	icon_controller_disconnect icon_extra_life icon_kia_skull icon_kia_skull_small icon_map_dropzone_selection_01
	icon_map_enemy_01 icon_map_objective_bunker_01 icon_map_player_01 icon_map_shuttle_01 icon_outofscreen_player_01
	map_carry_objective_dash map_marker_ping menu_infobox_01 patrol_scan_ping voice_volume wing_01
]], "%S+") do ICONS[name] = true end

-- Functions whose drawing a mute cuts. Screens need no entry: "loadout" is MenuScreenLoadout.update.
local SCOPES = {
	["hud.panels"] = { { "Hud", "draw_hud" } },
	["hud.map"] = { { "Hud", "draw_map" } },
	["hud.stratagems"] = { { "Hud", "draw_stratagems" }, { "Hud", "draw_stratagem_cooldown_list" } },
	["hud.names"] = { { "Hud", "draw_player_name_in_world" } },
	["hud.world"] = { { "Hud", "draw_plugins" } },
	["hud.prompts"] = { { "Hud", "draw_interactable" } },
	["hud.dialogue"] = { { "Hud", "draw_dialogue" } },
	chat = { { "TextChat", "update" } },
	hints = { { "HintManager", "draw" } },
	samples = { { "ResearchSampleManager", "draw" } },
	news = { { "NewsScroller", "update" } },
	save_icon = { { "SaveNotification", "update" } },
	mission_end = {},
}
for _, name in B.ipairs({ "MissionResult", "MissionScore", "IndividualStats", "PrePlanetPause", "PlanetCompleted", "PlanetCompletedScore",
	"PlanetCompletedReward", "EventCompletedScore", "EventCompletedReward", "TeamEfforts", "WaitingForData", "GenocideResults",
	"ProvinggroundsCompleted", "ExperienceDistribution", "Done" }) do
	B.insert(SCOPES.mission_end, { "MissionEndState" .. name, "draw" })
end

-- "hud" is every hud.* id at once.
local HUD_IDS = { "hud.panels", "hud.map", "hud.stratagems", "hud.names", "hud.world", "hud.prompts", "hud.dialogue" }

-- Where the color of a retained draw sits among a function's arguments, self left out.
local RETAINED = { create_image = 4, update_image = 5, create_rotated_image = 5, update_rotated_image = 6,
	create_text = 3, update_text = 4, create_rect = 3, update_rect = 4 }
local IMMEDIATE = { "rect", "rotated_rect", "image", "rotated_image", "text" }
local DRAWER = { "draw_rect", "draw_rect_rotated", "draw_triangle", "draw_texture", "draw_texture_uv", "draw_texture_rotated",
	"draw_texture_rotated_uv", "draw_texture_dynamic", "draw_text", "draw_text_shadow" }

local PROMPTS = { select = "#cross", back = "#mod_back" }

local M = {
	wanted = false, cleared = CLEARED, screens = {}, mutes = {}, muted = {}, scope = nil, points = {}, primitives = {},
	refused = {}, frames = {}, remaps = {}, guis = {}, font_files = {}, font_paths = {}, arts = {}, stamp = 0, can_draw = false, fade = 1, win = nil, glyphs = {}, glyph_count = 0,
}

local function global(name)
	return B.rawget(B.G, name)
end

local function want()
	M.wanted = true
end

-- A session mod's output stops with its hooks, and a mod that was switched off stops altogether. An
-- expansion draws in any lobby, so the screens that lead to its content show.
local function allowed(state)
	if state and state.off then return false end
	return state == nil or state.mod.scope == "client" or state.mod.scope == "expansion" or M.session.allows(state)
end

local function state_of()
	local boot = global("Boot")
	local sm = B.type(boot) == "table" and boot.state_machine
	if not sm then return nil, nil end
	local ok, st, name = B.pcall(function() return sm:get_current_state(), sm:get_current_state_name() end)
	if not ok then return nil, nil end
	return st, name
end

-- MenuScreenArmoryProfile <-> armory_profile.
local function screen_id(screen)
	local name = B.type(screen) == "table" and screen.class_name
	if B.type(name) ~= "string" then return nil end
	local short = M.ids[name]
	if not short then
		short = B.sub((B.gsub((B.gsub(name, "^MenuScreen", "")), "%u", function(c) return "_" .. B.lower(c) end)), 2)
		M.ids[name] = short
	end
	return short
end
M.ids = {}

local function class_name(id)
	return "MenuScreen" .. (B.gsub("_" .. id, "_(%l)", function(c) return upper(c) end))
end

local function top_screen(st)
	local list = st.active_screens
	if B.type(list) == "table" and #list > 0 then return list[#list] end
	return st.active_screen
end

local NO_SCREEN = { menu = "ship", main_menu = "title", game = "hud" }

function M.current()
	local st, name = state_of()
	local out = { state = name, screen = NO_SCREEN[name] or name }
	if B.type(st) ~= "table" then return out end
	local top = top_screen(st)
	local id = screen_id(top)
	if id then
		out.screen = id
		local sub = top.current_substate and top.current_substate.state or top.active_state
		out.sub = B.type(sub) == "table" and sub.name or nil
	elseif B.type(st.current_state) == "table" then
		out.sub = st.current_state.name
	end
	return out
end

-- Cutting the game's output. The wrappers are plain closures rather than obsidian.hook: the HUD makes
-- hundreds of these calls a frame, and they only read two fields.

local function cut()
	return M.scope ~= nil and M.muted[M.scope] == true
end

local function leave(outer, ...)
	M.scope = outer
	return ...
end

-- false while the game hasn't defined the table yet. A table without the function never will.
local function wrap_scope(id, target, method)
	local t = target
	if B.type(t) == "string" then t = global(t) end
	if B.type(t) ~= "table" then return false end
	if B.type(t[method]) ~= "function" then return true end
	local original = t[method]
	t[method] = function(...)
		local outer = M.scope
		M.scope = id
		return leave(outer, original(...))
	end
	return true
end

local function skip_when_cut(t, name)
	local original = t[name]
	if B.type(original) ~= "function" then return end
	t[name] = function(...)
		if cut() then return end
		return original(...)
	end
end

-- A retained object is the game's to keep and destroy, so it's still made and updated, in a color
-- with no alpha. The shape is the game's own call with a different color.
local function hide_when_cut(t, name, at)
	local original = t[name]
	if B.type(original) ~= "function" then return end
	t[name] = function(self, a, b, c, d, e, f, g, h)
		if cut() then
			local args = { a, b, c, d, e, f, g, h }
			args[at] = global("Color")(0, 255, 255, 255)
			return original(self, B.unpack(args, 1, 8))
		end
		return original(self, a, b, c, d, e, f, g, h)
	end
end

-- Goes in when a mod first mutes something, and stays: with nothing muted it only reads two fields.
local function install_primitives()
	local hd, drawer = global("HdGui"), global("GuiDrawer")
	if B.type(hd) == "table" and not M.primitives.hd then
		M.primitives.hd = true
		for _, name in B.ipairs(IMMEDIATE) do skip_when_cut(hd, name) end
		for name, at in B.pairs(RETAINED) do hide_when_cut(hd, name, at) end
	end
	if B.type(drawer) == "table" and not M.primitives.drawer then
		M.primitives.drawer = true
		for _, name in B.ipairs(DRAWER) do skip_when_cut(drawer, name) end
	end
end

local function install_points()
	if #M.points == 0 then return end
	local still = {}
	for _, p in B.ipairs(M.points) do
		if not wrap_scope(p.id, p.target, p.method) then B.insert(still, p) end
	end
	M.points = still
end

M.known = {}

-- Asks for the cut points of an id, once. A class the game hasn't loaded yet waits for it. The parts
-- of the HUD go in together: they nest, and a part that isn't muted has to end the cut of the one
-- around it.
local function request(id)
	if M.known[id] then return end
	M.known[id] = true
	want()
	local list = SCOPES[id] or { { class_name(id), "update" } }
	for _, p in B.ipairs(list) do B.insert(M.points, { id = id, target = p[1], method = p[2] }) end
	if B.sub(id, 1, 4) == "hud." then
		for _, part in B.ipairs(HUD_IDS) do request(part) end
	end
end

local function expand(id)
	if id == "hud" then return HUD_IDS end
	return { id }
end

local function refused_key(state, id)
	return (state and state.id or "") .. "\0" .. id
end

-- Asks for one of the game's screens or HUD parts to draw nothing, until unmute. Its logic keeps
-- running. false when this mod's screen failed: the game keeps that one.
function M.mute(id)
	if B.type(id) ~= "string" or id == "" then return false, "bad_type" end
	local state = M.core.current
	for _, one in B.ipairs(expand(id)) do
		if M.refused[refused_key(state, one)] then return false, "failed" end
		request(one)
		local found = false
		for _, m in B.ipairs(M.mutes) do
			if m.id == one and m.state == state then found = true end
		end
		if not found then B.insert(M.mutes, { id = one, state = state }) end
	end
	return true
end

function M.unmute(id)
	if B.type(id) ~= "string" then return false, "bad_type" end
	local state, wanted, kept = M.core.current, {}, {}
	for _, one in B.ipairs(expand(id)) do wanted[one] = true end
	for _, m in B.ipairs(M.mutes) do
		if not (m.state == state and wanted[m.id]) then B.insert(kept, m) end
	end
	M.mutes = kept
	return true
end

-- A function of the game whose drawing a mute of id cuts. For a part no id above reaches. Lua only:
-- it adds no engine call.
function M.scope_of(id, target, method)
	if B.type(id) ~= "string" or B.type(method) ~= "string" then return false, "bad_type" end
	if not (B.type(target) == "string" or B.type(target) == "table") then return false, "bad_type" end
	want()
	M.known[id] = true
	B.insert(M.points, { id = id, target = target, method = method })
	return true
end

local function hud_rebuild()
	local st = state_of()
	local hud = B.type(st) == "table" and st.hud
	local gui = B.type(hud) == "table" and hud.hd_gui
	-- What a resolution change does: the HUD destroys its retained objects and makes them again, so
	-- ones it doesn't update every frame take the change too (hud.lua, Hud.update).
	if B.type(gui) == "table" then gui.is_dirty = true end
end

-- Worked out once a frame: a mod can be switched off, and a lobby can stop being cleared, at any time.
local function refresh_mutes()
	local now = {}
	for _, m in B.ipairs(M.mutes) do
		if allowed(m.state) then now[m.id] = true end
	end
	for _, entry in B.ipairs(M.screens) do
		-- A screen that can't draw yet has nothing to put in place of the game's.
		if entry.active and not entry.failed and allowed(entry.state) and (M.can_draw or not entry.handlers.draw) then
			for _, id in B.ipairs(entry.mute) do now[id] = true end
		end
	end
	local hud = false
	for id in B.pairs(now) do
		if not M.muted[id] and B.sub(id, 1, 4) == "hud." then hud = true end
	end
	for id in B.pairs(M.muted) do
		if not now[id] and B.sub(id, 1, 4) == "hud." then hud = true end
	end
	M.muted = now
	if hud then hud_rebuild() end
end

-- Screens.

local function lift(entry)
	local kept = {}
	local function same(id)
		return entry.id == "*" or id == entry.id or B.sub(id, 1, #entry.id + 1) == entry.id .. "."
	end
	for _, m in B.ipairs(M.mutes) do
		if m.state == entry.state and same(m.id) then
			M.refused[refused_key(entry.state, m.id)] = true
		else
			B.insert(kept, m)
		end
	end
	M.mutes = kept
	for _, id in B.ipairs(entry.mute) do M.refused[refused_key(entry.state, id)] = true end
end

-- A screen that fails stops for the session and gives the game its screen back, so the game stays
-- playable. It counts against the mod like any other failing callback.
local function call(entry, what, ...)
	local fn = entry.handlers[what]
	if B.type(fn) ~= "function" then return true end
	local ok, err = M.core.call(entry.state, "ui " .. what, fn, ...)
	if not ok then
		entry.failed = true
		lift(entry)
		log.limited("ui " .. entry.id, "ui: " .. (entry.state and entry.state.id or "?") .. " " .. entry.id .. " gave the screen back: " .. B.tostring(err))
	end
	return ok
end

local function mute_list(value, id)
	if value == true then return expand(id) end
	local out = {}
	if B.type(value) == "table" then
		for _, one in B.ipairs(value) do
			for _, each in B.ipairs(expand(one)) do B.insert(out, each) end
		end
	end
	return out
end

-- id: a value of current().screen, or "*" for all of them. handlers: enter(screen), draw(kit, data,
-- dt), exit(), and mute: true for this screen, or a list of ids, held only while it's up.
function M.on(id, handlers)
	if B.type(id) ~= "string" or B.type(handlers) ~= "table" then return false, "bad_type" end
	want()
	local entry = { id = id, state = M.core.current, handlers = handlers, mute = mute_list(handlers.mute, id), active = false, failed = false }
	for _, one in B.ipairs(entry.mute) do request(one) end
	B.insert(M.screens, entry)
	return true
end

-- Takes this mod's handlers off a screen.
function M.off(id)
	local state, kept = M.core.current, {}
	for _, entry in B.ipairs(M.screens) do
		if not (entry.id == id and entry.state == state) then B.insert(kept, entry) end
	end
	M.screens = kept
	return true
end

local function run_screens(cur, dt)
	for _, entry in B.ipairs(M.screens) do
		local up = not entry.failed and allowed(entry.state) and (entry.id == "*" or entry.id == cur.screen)
		if up and not entry.active then
			entry.active = true
			call(entry, "enter", cur)
		elseif not up and entry.active then
			entry.active = false
			-- A session mod does nothing while it's gated, and a failed screen is done.
			if allowed(entry.state) and not entry.failed then call(entry, "exit") end
		end
		if entry.active and not entry.failed and M.can_draw then
			entry.kit = entry.kit or M.canvas(entry.id, { owner = entry.state })
			call(entry, "draw", entry.kit, M.data, dt)
		end
	end
end

-- Layout.

local function window()
	if M.win and M.win.stamp == M.stamp then return M.win.w, M.win.h, M.win.x, M.win.y end
	local w, h, x, y
	local st = state_of()
	local proxy = B.type(st) == "table" and st.world_proxy
	if proxy then
		-- The viewport's window, as HdGui uses (hd_gui.lua:17): missions letterbox it.
		local ok, a, b, c, d = B.pcall(function() return proxy:default_viewport():window() end)
		if ok and B.type(a) == "number" then w, h, x, y = a, b, c or 0, d or 0 end
	end
	if not w then
		w, h = global("Application").resolution()
		x, y = 0, 0
	end
	M.win = { stamp = M.stamp, w = w, h = h, x = x, y = y }
	return w, h, x, y
end

local function ui_scale()
	local save = global("SaveManager")
	local ok, value = B.pcall(function() return save:get("ui_scale") end)
	return ok and B.type(value) == "number" and value or 1
end

-- HdGui's sums for any design size (hd_gui.lua:27-34). y keeps the game's centring, which leaves out
-- the player's ui_scale.
local function layout(design_w, design_h, centered)
	local w, h, x, y = window()
	local user = ui_scale()
	local scale = B.min(w / design_w, h / design_h) * user
	if centered then user = 1 end
	return scale, x + (w - design_w * scale) * 0.5, y + (h - design_h * scale / user) * 0.5
end

-- Sets this mod's design space: its kits take coordinates in it from now on, centred and scaled as
-- the game does, whatever the resolution. nil goes back to pixels. Worked out each frame, so a mod
-- can call this while it starts, before the game has a window. opts.centered keeps the space in the
-- middle of the window at any ui_scale, where the game's own sums leave it at the bottom.
function M.frame(design_w, design_h, opts)
	local owner = M.core.current or NOBODY
	if design_w == nil then
		M.frames[owner] = nil
		return true
	end
	if B.type(design_w) ~= "number" or B.type(design_h) ~= "number" or design_w <= 0 or design_h <= 0 then return false, "out_of_range" end
	M.frames[owner] = { w = design_w, h = design_h, centered = B.type(opts) == "table" and opts.centered == true }
	return true
end

-- The area the game keeps its own menus in, with the player's ui_scale, in this mod's coordinates.
function M.safe()
	local f = M.frames[M.core.current or NOBODY]
	local scale, x, y = layout(DESIGN_W, DESIGN_H)
	local left, bottom = x + SAFE_X * scale, y + SAFE_Y * scale
	local w, h = (DESIGN_W - SAFE_X * 2) * scale, (DESIGN_H - SAFE_Y * 2) * scale
	if not f then return { x = left, y = bottom, w = w, h = h, scale = scale } end
	local fs, fx, fy = layout(f.w, f.h)
	return { x = (left - fx) / fs, y = (bottom - fy) / fs, w = w / fs, h = h / fs, scale = scale / fs }
end

-- Mouse.

-- The game's own test (menu_screen_base.lua:968-1009), with the rectangle moved first.
local function hit_test(original, self, gui, pos, size, pad, raw, cursor)
	local remap = M.remaps[screen_id(self)]
	if not remap or not allowed(remap.state) then return original(self, gui, pos, size, pad, raw, cursor) end
	pad = pad or self.user_pad_number
	local at = cursor
	if not at and self.cursor_position then at = global("Vector3Aux").unbox(self.cursor_position) end
	if pad ~= global("KEYBOARD_MOUSE_PAD_NUMBER") or not at then return false end
	local p, s
	if gui:isa(global("HdGui")) or raw then
		p, s = gui:calculate_pos(pos, size)
	else
		p, s = gui:calculate_pos(pos, size, gui:get_current_template().anchor)
	end
	local ok, x, y, w, h = M.core.call(remap.state, "ui remap_hits", remap.fn, p.x, p.y, s.x, s.y)
	if not ok then
		M.remaps[screen_id(self)] = nil
		return original(self, gui, pos, size, pad, raw, cursor)
	end
	if x == false then return false end
	if x == nil then x, y, w, h = p.x, p.y, s.x, s.y end
	return x <= at.x and at.x < x + w and y <= at.y and at.y < y + h
end

local function install_hits()
	if M.primitives.hits then return end
	local base = global("MenuScreenBase")
	if B.type(base) ~= "table" or B.type(base.is_box_selected_by_cursor) ~= "function" then return end
	M.primitives.hits = true
	local original = base.is_box_selected_by_cursor
	base.is_box_selected_by_cursor = function(self, gui, pos, size, pad, raw, cursor)
		return hit_test(original, self, gui, pos, size, pad, raw, cursor)
	end
end

-- The game tests the mouse against where it draws. When a layout moves, fn(x, y, w, h) gets each
-- rectangle in pixels and answers where it is now, nil for no change, or false for nowhere.
function M.remap_hits(id, fn)
	if B.type(id) ~= "string" then return false, "bad_type" end
	if fn == nil then
		M.remaps[id] = nil
		return true
	end
	if B.type(fn) ~= "function" then return false, "bad_type" end
	want()
	M.remaps[id] = { fn = fn, state = M.core.current }
	return true
end

-- Words.

local function main_pad()
	local settings = global("GameSettings")
	return settings and settings.get_main_pad_number and settings.get_main_pad_number()
end

-- The button for a pad, or the game's text with its buttons filled in, in the form that pad shows.
-- name: an action such as "select", "back", "fire" or "reload", a token such as "#mod_map", or a
-- text key. pad: a pad number, nil for the main player's.
function M.prompt(name, pad)
	if B.type(name) ~= "string" or name == "" then return nil end
	local lm = global("LocalizationManager")
	if B.type(lm) ~= "table" then return nil end
	if pad == nil then pad = main_pad() end
	local set = pad == global("KEYBOARD_MOUSE_PAD_NUMBER") and "keyboard_mouse" or ("pad" .. B.tostring(pad))
	local mods = lm.modifier_sets and lm.modifier_sets[set]
	local token = PROMPTS[name] or (B.sub(name, 1, 1) == "#" and name or "#mod_" .. name)
	for _, m in B.ipairs(mods or {}) do
		-- The two marks wrap a key name for HdGui's own overlay, which a canvas doesn't have.
		if m.modifier == token then return (B.gsub(m.value, "\194\172", "")) end
	end
	if B.sub(name, 1, 1) == "#" then return nil end
	return (B.gsub(lm:string(name, nil, mods and set or nil), "\194\172", ""))
end

-- Canvas.

local function online(package)
	local boot = global("Boot")
	local manager = B.type(boot) == "table" and boot.package_manager
	if not manager then return false end
	local ok, on = B.pcall(manager.is_package_online, manager, package)
	return ok and on == true
end

local function guard_shutdown()
	if M.guarded then return end
	M.guarded = true
	-- The guis live on a world the game releases at shutdown, so they go first.
	local shutdown = global("shutdown")
	if B.type(shutdown) ~= "function" then return end
	B.rawset(B.G, "shutdown", function(...)
		for name, gui in B.pairs(M.guis) do
			B.pcall(global("World").destroy_gui, M.world, gui) -- input_component.lua:114
			M.guis[name] = nil
		end
		return shutdown(...)
	end)
end

local function gui_named(name)
	local gui = M.guis[name]
	if gui then return gui end
	local def = GUIS[name]
	local boot = global("Boot")
	local world = B.type(boot) == "table" and boot.brightness_world
	if not world or (def.package and not online(def.package)) then return nil end
	guard_shutdown()
	-- Proven on 2026-10-30 with each first material. hud/hud is held back by CLEARED.icons.
	local args = {}
	for _, material in B.ipairs(def.materials) do
		args[#args + 1] = "material"
		args[#args + 1] = material
	end
	args[#args + 1] = "immediate"
	gui = global("World").create_screen_gui(world, B.unpack(args))
	M.world, M.guis[name] = world, gui
	return gui
end

-- Where shapes and the debug fonts go.
local function canvas_gui()
	return gui_named("fonts") or gui_named("shapes")
end

local HEX = {}

local function parse_hex(s)
	local c = HEX[s]
	if c then return c end
	local r, g, b, a = B.match(s, "^#(%x%x)(%x%x)(%x%x)(%x?%x?)$")
	if not r then B.error("not a color: " .. s, 0) end
	c = { B.tonumber(r, 16), B.tonumber(g, 16), B.tonumber(b, 16), a ~= "" and B.tonumber(a, 16) or 255 }
	HEX[s] = c
	return c
end

-- Colors are { r, g, b }, { r, g, b, a } or "#rrggbb" and "#rrggbbaa", from 0 to 255. Color() is a temp
-- of one frame, so a mod can't keep one, and this makes each fresh. Alpha comes first in Color.
local function color(kit, c)
	if B.type(c) == "string" then c = parse_hex(c) elseif B.type(c) ~= "table" then c = WHITE end
	local alpha = (c[4] or 255) * kit.alpha
	if not kit.over_fades then alpha = alpha * M.fade end
	return global("Color")(alpha, c[1], c[2], c[3])
end

local function sync(kit)
	if kit.stamp == M.stamp then return end
	kit.stamp = M.stamp
	local f = M.frames[kit.owner]
	if f then
		kit.scale, kit.ox, kit.oy = layout(f.w, f.h, f.centered)
	else
		kit.scale, kit.ox, kit.oy = 1, 0, 0
	end
end

-- The menu is the loader's own, and the fonts' gui is what it needs, so a system kit doesn't wait.
-- While a mod has asked for the screen (M.solo), only the kits of the mods it named draw.
local function ready(kit)
	if not (M.can_draw or kit.system) then return false end
	if M.solo_ids and not kit.system and not M.solo_ids[kit.owner.id or ""] then return false end
	sync(kit)
	return canvas_gui() ~= nil
end

-- Only the kits of the mods in `ids` (a list of mod ids) draw, until solo(nil): a photo mode wants the
-- screen to itself. An empty list holds every mod's drawing back. The loader's own menu always draws.
-- Asking again replaces the list. A mod switched off while it holds the screen lets it go.
function M.solo(ids)
	if ids == nil or ids == false then
		M.solo_ids, M.solo_by = nil, nil
		return true
	end
	if B.type(ids) ~= "table" then return false, "bad_type" end
	local set = {}
	for _, id in B.ipairs(ids) do
		if B.type(id) == "string" then set[id] = true end
	end
	M.solo_ids, M.solo_by = set, M.core.current
	return true
end

-- The ids solo() was given, as a fresh list, or nil when every mod draws.
function M.soloed()
	if not M.solo_ids then return nil end
	local out = {}
	for id in B.pairs(M.solo_ids) do B.insert(out, id) end
	B.sort(out)
	return out
end

local Kit = {}
Kit.__index = Kit

-- id: a name for the log. opts: layer, from 0 to 875, and over_fades, to stay at full strength while
-- the game fades to black. The gui draws over the fades, so by default a kit fades with the game.
function M.canvas(id, opts)
	want()
	opts = opts or {}
	return B.setmetatable({ id = id, owner = opts.owner or M.core.current or NOBODY, z = opts.layer or DEFAULT_LAYER, alpha = 1, area = nil,
		over_fades = opts.over_fades == true, scale = 1, ox = 0, oy = 0, stamp = -1 }, Kit)
end

function Kit:layer(n)
	if B.type(n) == "number" then self.z = B.max(0, B.min(B.floor(n), MENU_LAYER - 1)) end
	return self.z
end

-- A point in pixels, such as the mouse's, in this kit's units. The same origin as the gui: the mouse
-- is read raw, and it shares the bottom left corner with what's drawn.
function Kit:point(x, y)
	sync(self)
	return (x - self.ox) / self.scale, (y - self.oy) / self.scale
end

function Kit:opacity(a)
	if B.type(a) == "number" then self.alpha = B.max(0, B.min(a, 1)) end
	return self.alpha
end

local function intersect(area, x, y, w, h)
	if not area then return x, y, w, h end
	local left, bottom = B.max(area.x, x), B.max(area.y, y)
	local right, top = B.min(area.x + area.w, x + w), B.min(area.y + area.h, y + h)
	if right <= left or top <= bottom then return nil end
	return left, bottom, right - left, top - bottom
end

function Kit:rect(x, y, w, h, c)
	if not ready(self) then return end
	local cx, cy, cw, ch = intersect(self.area, x, y, w, h)
	if not cx then return end
	local s = self.scale
	global("Gui").rect(canvas_gui(), global("Vector3")(self.ox + cx * s, self.oy + cy * s, self.z), global("Vector2")(cw * s, ch * s), color(self, c))
end

-- One pass of Sutherland-Hodgman over a flat list of points. There's no engine clip for triangles.
local function clip_pass(points, axis, limit, above)
	local out, n = {}, #points / 2
	for i = 1, n do
		local j = i % n + 1
		local ax, ay, bx, by = points[i * 2 - 1], points[i * 2], points[j * 2 - 1], points[j * 2]
		local a, b = axis == 1 and ax or ay, axis == 1 and bx or by
		local a_in, b_in = (above and a >= limit) or (not above and a <= limit), (above and b >= limit) or (not above and b <= limit)
		if a_in then
			out[#out + 1], out[#out + 2] = ax, ay
		end
		if a_in ~= b_in then
			local t = (limit - a) / (b - a)
			out[#out + 1], out[#out + 2] = ax + (bx - ax) * t, ay + (by - ay) * t
		end
	end
	return out
end

local function clip_points(points, area)
	local out = clip_pass(points, 1, area.x, true)
	if #out > 0 then out = clip_pass(out, 1, area.x + area.w, false) end
	if #out > 0 then out = clip_pass(out, 2, area.y, true) end
	if #out > 0 then out = clip_pass(out, 2, area.y + area.h, false) end
	return out
end

local function signed_area(points)
	local sum, n = 0, #points / 2
	for i = 1, n do
		local j = i % n + 1
		sum = sum + points[i * 2 - 1] * points[j * 2] - points[j * 2 - 1] * points[i * 2]
	end
	return sum
end

-- A convex shape, as a flat list { x1, y1, x2, y2, ... }, drawn as a fan. Points are Vector3(x, 0, y):
-- with (x, y, 0) nothing draws. Only counter-clockwise has been seen to draw, so that's what's sent.
function Kit:poly(points, c)
	if not ready(self) or B.type(points) ~= "table" or #points < 6 then return end
	local pts = points
	if self.area then
		pts = clip_points(points, self.area)
		if #pts < 6 then return end
	end
	local n, s, ox, oy, order = #pts / 2, self.scale, self.ox, self.oy, 1
	if signed_area(pts) < 0 then order = -1 end
	local gui, v3, triangle = canvas_gui(), global("Vector3"), global("Gui").triangle
	local engine_color = color(self, c)
	local function at(i)
		if order < 0 then i = n - i + 1 end
		return v3(ox + pts[i * 2 - 1] * s, 0, oy + pts[i * 2] * s)
	end
	for i = 2, n - 1 do triangle(gui, at(1), at(i), at(i + 1), self.z, engine_color) end
end

-- The edge of a shape, as a band inside it.
function Kit:outline(points, c, width)
	if B.type(points) ~= "table" or #points < 6 then return end
	width = width or 1
	local n, sign = #points / 2, signed_area(points) < 0 and -1 or 1
	local z = self.z
	self.z = z + 1
	for i = 1, n do
		local j = i % n + 1
		local ax, ay, bx, by = points[i * 2 - 1], points[i * 2], points[j * 2 - 1], points[j * 2]
		local dx, dy = bx - ax, by - ay
		local length = (dx * dx + dy * dy) ^ 0.5
		if length > 0 then
			local nx, ny = -dy / length * width * sign, dx / length * width * sign
			self:poly({ ax, ay, bx, by, bx + nx, by + ny, ax + nx, ay + ny }, c)
		end
	end
	self.z = z
end

local function corner_set(text)
	local out = {}
	for word in B.gmatch(B.tostring(text or "tr bl"), "%a+") do out[word] = true end
	return out
end

-- Counter-clockwise from the bottom left.
local function panel_points(x, y, w, h, cut_by, corners)
	local r, t, out = x + w, y + h, {}
	local function add(...)
		for _, n in B.ipairs({ ... }) do out[#out + 1] = n end
	end
	if corners.bl then add(x, y + cut_by, x + cut_by, y) else add(x, y) end
	if corners.br then add(r - cut_by, y, r, y + cut_by) else add(r, y) end
	if corners.tr then add(r, t - cut_by, r - cut_by, t) else add(r, t) end
	if corners.tl then add(x + cut_by, t, x, t - cut_by) else add(x, t) end
	return out
end

-- { x, y, w, h, cut = 16, corners = "tr bl", fill = color, line = color, line_width = 1 }
function Kit:panel(p)
	local x, y, w, h = p.x or p[1], p.y or p[2], p.w or p[3], p.h or p[4]
	local by = B.max(0, B.min(p.cut or 0, B.min(w, h) / 2))
	local points = panel_points(x, y, w, h, by, by > 0 and corner_set(p.corners) or {})
	if p.fill then self:poly(points, p.fill) end
	if p.line then self:outline(points, p.line, p.line_width) end
end

-- One of the icons drawn from shapes (obsidian.ui.glyphs lists them). x and y: its bottom left. false when
-- there's no such name. They need no art, so they work in every state a kit does.
function Kit:glyph(name, x, y, size, c)
	local polys = glyphs[name]
	if not polys then
		log.limited("ui glyph " .. B.tostring(name), "ui: no glyph named " .. B.tostring(name))
		return false
	end
	local s = size / 16
	for _, poly in B.ipairs(polys) do
		local points = {}
		for i = 1, #poly, 2 do
			points[i], points[i + 1] = x + poly[i] * s, y + poly[i + 1] * s
		end
		self:poly(points, c)
	end
	return true
end

-- Points up. With an icon, it's drawn in the middle, whole: cropping one to the shape needs textured
-- triangles, which the probe hasn't settled.
function Kit:hex(x, y, r, c, icon)
	local points = {}
	for i = 0, 5 do
		local a = pi / 2 + i * pi / 3
		points[#points + 1] = x + r * cos(a)
		points[#points + 1] = y + r * sin(a)
	end
	self:poly(points, c)
	if icon then self:icon(icon, x - r * 0.6, y - r * 0.6, r * 1.2, r * 1.2) end
end

local function known_icon(name)
	if ICONS[name] then return true end
	local loadout, progression = global("LoadoutSettings"), global("ProgressionSettings")
	for _, s in B.ipairs(B.type(loadout) == "table" and loadout.stratagems or {}) do
		if s.icon then ICONS[s.icon] = true end
	end
	for _, rank in B.ipairs(B.type(progression) == "table" and progression.ranks or {}) do
		if rank.icon then ICONS[rank.icon] = true end
	end
	return ICONS[name] == true
end

-- Every name kit:glyph draws.
function M.glyph_names()
	local out = {}
	for name in B.pairs(glyphs) do out[#out + 1] = name end
	B.sort(out)
	return out
end

-- The loader's own kit, in the game's design space, for the menu. It draws from the first frame in
-- every state, over fades, and above any mod's kit.
local SYSTEM = {}

function M.system_canvas(id)
	want()
	M.frames[SYSTEM] = { w = DESIGN_W, h = DESIGN_H }
	local kit = M.canvas(id, { owner = SYSTEM, over_fades = true })
	kit.system = true
	return kit
end

-- Every name kit:icon draws.
function M.icons()
	known_icon("")
	local out = {}
	for name in B.pairs(ICONS) do out[#out + 1] = name end
	B.sort(out)
	return out
end

local function bitmap(kit, gui, name, x, y, w, h, c, uv)
	local self = kit
	local u0, v0, u1, v1 = 0, 0, 1, 1
	if uv then u0, v0, u1, v1 = uv[1], uv[2], uv[3], uv[4] end
	local cx, cy, cw, ch = intersect(self.area, x, y, w, h)
	if not cx then return false end
	local du, dv = (u1 - u0) / w, (v1 - v0) / h
	local nu0, nu1 = u0 + (cx - x) * du, u0 + (cx + cw - x) * du
	-- v runs down the image and y up the screen.
	local nv0, nv1 = v0 + (y + h - cy - ch) * dv, v0 + (y + h - cy) * dv
	local s, v2 = self.scale, global("Vector2")
	-- Gui.bitmap_uv(gui, material, uv00, uv11, pos, size, color): hd_gui.lua:220, input_component.lua:267.
	global("Gui").bitmap_uv(gui, name, v2(nu0, nv0), v2(nu1, nv1), global("Vector3")(self.ox + cx * s, self.oy + cy * s, self.z), v2(cw * s, ch * s), color(self, c))
	return true
end

-- uv: { u0, v0, u1, v1 } for part of the image. false when it can't draw that name.
function Kit:icon(name, x, y, w, h, c, uv)
	if not ready(self) or not M.cleared.icons then return false end
	if not known_icon(name) then
		log.limited("ui icon " .. B.tostring(name), "ui: no icon named " .. B.tostring(name))
		return false
	end
	local gui = gui_named("icons")
	if not gui then return false end
	return bitmap(self, gui, name, x, y, w, h, c, uv)
end

-- An image from a material a mod ships in the boot bundle, which the fonts' gui lists. A uv with
-- u0 above u1 mirrors it. false when M.art hasn't accepted that name.
function Kit:art(name, x, y, w, h, c, uv)
	if not ready(self) or not M.arts[name] then return false end
	local gui = gui_named("fonts")
	if not gui then return false end
	return bitmap(self, gui, name, x, y, w, h, c, uv)
end

-- The game's player band: its gradient, tinted. A flat tint until the icons are cleared.
function Kit:band(x, y, w, h, c)
	if self:icon(GRADIENT, x, y, w, h, c) then return end
	local alpha = self.alpha
	self.alpha = alpha * 0.5
	self:rect(x, y, w, h, c)
	self.alpha = alpha
end

-- value from 0 to 1. back: a color behind it.
function Kit:bar(x, y, w, h, value, c, back)
	if back then self:rect(x, y, w, h, back) end
	local filled = w * B.max(0, B.min(value or 0, 1))
	if filled <= 0 then return end
	local z = self.z
	self.z = z + 1
	self:rect(x, y, filled, h, c)
	self.z = z
end

-- gui_drawer.lua:351 is the shipped call. The fallback is the one font that has nearly everything.
local function has_glyphs(gui, s, font)
	local key = font.path .. "\0" .. s
	local known = M.glyphs[key]
	if known == nil then
		known = global("Gui").has_all_glyphs(gui, s, font.path) == true
		if M.glyph_count >= 512 then M.glyphs, M.glyph_count = {}, 0 end
		M.glyphs[key], M.glyph_count = known, M.glyph_count + 1
	end
	return known
end

local function pick_font(name, s)
	local font = FONTS[name or "body"]
	if not font then
		log.limited("ui font " .. B.tostring(name), "ui: no font named " .. B.tostring(name))
		font = FONTS.debug
	end
	if not font.gui then return font, canvas_gui() end
	local gui = gui_named(font.gui)
	if not gui then return FONTS.debug, canvas_gui() end
	if font ~= FONTS.chat and not has_glyphs(gui, s, font) then font = FONTS.chat end
	return font, gui
end

local function text_width(gui, font, s, size, tracking)
	local extents = global("Gui").text_extents
	local low, high
	if tracking then
		low, high = extents(gui, s, font.path, size, "letter_spacing", tracking)
	else
		low, high = extents(gui, s, font.path, size)
	end
	return high.x - low.x
end

local function lines_of(s)
	local out = {}
	for line in B.gmatch(s .. "\n", "(.-)\n") do out[#out + 1] = line end
	return out
end

local function clean(s)
	s = B.tostring(s)
	local utf8 = global("Utf8")
	if B.type(utf8) == "table" and utf8.valid and not utf8.valid(s) then return nil end
	return s
end

-- What's left of a line inside a clip, in whole letters: an immediate gui has one clip per material,
-- so a text can't be cut through a letter without a gui per region.
local function fit_line(line, x, left, right, width_of)
	local chars = {}
	for ch in B.gmatch(line, "[%z\1-\127\192-\255][\128-\191]*") do chars[#chars + 1] = ch end
	local first, last = 1, #chars
	while first <= last and x < left do
		x = x + width_of(chars[first])
		first = first + 1
	end
	while last >= first and x + width_of(B.concat(chars, "", first, last)) > right do last = last - 1 end
	if last < first then return nil end
	return B.concat(chars, "", first, last), x
end

-- opts: font ("body", "body_large", "heading", "heading_large", "chat", "debug", "debug_medium"),
-- size, tracking in units of this kit, align ("left", "center", "right"), color, leading. A line break
-- starts a new line below. y is the baseline.
function Kit:text(s, x, y, opts)
	if not ready(self) then return end
	s = clean(s)
	if not s or s == "" then return end
	opts = opts or {}
	local font, gui = pick_font(opts.font, s)
	local scale = self.scale
	local size = (opts.size or font.size) * scale
	local tracking = opts.tracking and opts.tracking * scale or nil
	local align, area = opts.align, self.area
	local leading = opts.leading or (opts.size or font.size)
	local engine_color = color(self, opts.color)
	local draw, v3 = global("Gui").text, global("Vector3")
	local function width_of(part)
		return text_width(gui, font, part, size, tracking) / scale
	end
	for i, line in B.ipairs(lines_of(s)) do
		local lx, ly = x, y - (i - 1) * leading
		if line ~= "" and (align == "center" or align == "right") then
			local w = width_of(line)
			lx = align == "center" and lx - w / 2 or lx - w
		end
		if area and (ly < area.y or ly + size / scale * 0.7 > area.y + area.h) then line = "" end
		if area and line ~= "" then line, lx = fit_line(line, lx, area.x, area.x + area.w, width_of) end
		if line and line ~= "" then
			local pos = v3(self.ox + lx * scale, self.oy + ly * scale, self.z + 1)
			if tracking then
				draw(gui, line, font.path, size, font.material, pos, engine_color, "letter_spacing", tracking)
			else
				draw(gui, line, font.path, size, font.material, pos, engine_color)
			end
		end
	end
end

-- Width and height of a text in this kit's units, tracking included. The height is lines times size:
-- hd_gui.lua:488 and gui_drawer.lua:348 read it from different axes, and only one can be right.
function Kit:measure(s, opts)
	opts = opts or {}
	if not ready(self) then return 0, 0 end
	s = clean(s)
	if not s or s == "" then return 0, 0 end
	local font, gui = pick_font(opts.font, s)
	local size = (opts.size or font.size) * self.scale
	local tracking = opts.tracking and opts.tracking * self.scale or nil
	local lines, widest = lines_of(s), 0
	for _, line in B.ipairs(lines) do
		if line ~= "" then widest = B.max(widest, text_width(gui, font, line, size, tracking)) end
	end
	return widest / self.scale, #lines * (opts.size or font.size)
end

-- fn(kit) draws inside the rectangle. Shapes and icons are cut exactly, text at whole letters.
function Kit:clip(x, y, w, h, fn)
	local outer = self.area
	local cx, cy, cw, ch = intersect(outer, x, y, w, h)
	if not cx then return end
	self.area = { x = cx, y = cy, w = cw, h = ch }
	local ok, err = B.pcall(fn, self)
	self.area = outer
	if not ok then B.error(err, 0) end
end

-- Data: copies, so a mod can't write to the game by accident, and plain values only.

local function copy(t, depth)
	if B.type(t) ~= "table" then return nil end
	local out = {}
	for k, v in B.pairs(t) do
		local kind = B.type(v)
		if kind == "number" or kind == "string" or kind == "boolean" then
			out[k] = v
		elseif kind == "table" and (depth or 0) > 0 then
			out[k] = copy(v, depth - 1)
		end
	end
	return out
end

local function field(go, key)
	if not go or (go.has and not go:has(key)) then return nil end
	return go:get(key)
end

local function records()
	local st = state_of()
	local list = B.type(st) == "table" and st.players
	return B.type(list) == "table" and list or {}, st
end

-- The player's slot among the game's four, for a nil argument the first one on this machine.
local function slot_of(player)
	if player then return player end
	local list = records()
	local settings = global("GameSettings")
	local main = settings and settings.get_main_pad_number and settings.get_main_pad_number()
	local first
	for i, p in B.ipairs(list) do
		local go = p.player_go
		if go and go:owned() then
			if p.pad_number == main then return i end
			first = first or i
		end
	end
	return first or 1
end

local function player_color(index)
	local settings = global("GameSettings")
	local c = settings and settings.player_colors and settings.player_colors[index]
	-- Boxed as alpha, red, green, blue in x, y, z, w (hud.lua:1564).
	if B.type(c) == "table" then return { c.y, c.z, c.w } end
	return nil
end

local D = {}
M.data = B.setmetatable({}, {
	__index = D,
	__newindex = function() B.error("obsidian.ui.data is read-only", 2) end,
	__metatable = false,
})

-- name, title, rank_icon, xp, color, index, slot, is_local, pad, for each player in the game. title
-- is a text key.
function D.players()
	local out = {}
	local progression = global("ProgressionSettings")
	for slot, p in B.ipairs((records())) do
		local go = p.player_go
		if go and go:exists() then
			local xp, rank = field(go, "xp"), nil
			if xp and progression then rank = progression.get_rank(xp) end
			local index = field(go, "index")
			out[#out + 1] = { slot = slot, name = field(go, "name") or p.name, xp = xp, title = rank and rank.title, rank_icon = rank and rank.icon,
				index = index, color = index and player_color(index), is_local = go:owned(), pad = p.pad_number }
		end
	end
	return out
end

local function loadout_screen()
	local st = state_of()
	local top = B.type(st) == "table" and top_screen(st)
	if B.type(top) == "table" and top.class_name == "MenuScreenLoadout" and top.option_menus then return top end
	return nil
end

local function slot_item(entry)
	local item = entry.item or {}
	return { type = entry.type, id = entry.id, index = entry.index, upgrade = entry.upgrade_value, name = item.name, icon = item.icon }
end

local function available(menu)
	local out = {}
	for kind, list in B.pairs(menu.available_items or {}) do
		out[kind] = {}
		for _, group in B.ipairs(list.groups or {}) do
			local items = {}
			for _, item in B.ipairs(group.items or {}) do items[#items + 1] = { name = item.name, icon = item.icon } end
			out[kind][#out[kind] + 1] = { name = group.name, index = group.item_index, items = items }
		end
	end
	return out
end

-- The Loadout screen's panel of one player. selection: the row. submenu and slot: what's being picked.
function D.loadout(player)
	local screen = loadout_screen()
	local menu = screen and screen.option_menus[slot_of(player)]
	if not menu then return nil end
	local items = {}
	for i, entry in B.ipairs(menu.loadout_menu or {}) do items[i] = slot_item(entry) end
	return { items = items, available = available(menu), selection = menu.current_selection_index, submenu = menu.submenu,
		slot = menu.selected_loadout_slot, ready = menu.enabled == false, random = menu.random_loadout == true,
		loadout = menu.loadout_index, presets = copy(menu.loadouts, 2), new_unlocks = copy(menu.new_unlocks, 1), upgrades = copy(menu.item_upgrades, 1) }
end

local function hud_info(player)
	local st = state_of()
	local hud = B.type(st) == "table" and st.hud
	local list = B.type(hud) == "table" and hud.hud_info
	return B.type(list) == "table" and list[slot_of(player)] or nil, st
end

-- One player's panel, by the game's own field names: state is normal, map, stratagems, countdown,
-- custom, wait_for_spawn or dead.
function D.hud(player)
	local info = hud_info(player)
	if not info then return nil end
	local inventory = info.inventory_state
	return { state = info.state, health = copy(info.health_state), shield = copy(info.shield_state), inventory = copy(inventory),
		character = copy(info.character_state), stratagems = copy(B.type(inventory) == "table" and inventory.stratagems or nil, 1),
		cooldowns = copy(info.stratagem_cooldowns), respawn = info.respawn_countdown }
end

-- The map of one player, in the game's map units, and every player's ping.
function D.map(player)
	local info, st = hud_info(player)
	if not info then return nil end
	local level = st.generated_level or st.custom_level
	local pings, markers = {}, {}
	for slot, p in B.ipairs(st.players or {}) do
		local ping = field(p.player_go, "map_ping")
		if B.type(ping) == "table" and ping[1] ~= -1000 then pings[#pings + 1] = { slot = slot, x = ping[1], y = ping[2] } end
	end
	local manager = st.entity_manager
	local units = global("Unit")
	if manager and units and manager:has_entities("show_on_map") then
		for unit in B.pairs(manager:get_slave_entities("show_on_map")) do
			local at = units.world_position(unit, 0)
			markers[#markers + 1] = { x = at.x, y = at.y, z = at.z }
		end
	end
	return { zoom = info.map_zoom, offset = copy(info.map_offset), size = level and copy(level.map_size), pings = pings, markers = markers }
end

-- objectives, enemy, difficulty, environment, type and the timers the game runs at the end.
function D.mission()
	local st = state_of()
	if B.type(st) ~= "table" or not st.mission_manager then return nil end
	local objectives = {}
	for _, o in B.ipairs(st.mission_manager:get_objectives() or {}) do
		local params = o.params or {}
		objectives[#objectives + 1] = { name = o.name, type = o.type, state = o.state, group = o.group, done = params[1], total = params[2] }
	end
	local out = { objectives = objectives, game_over = st.game_over_timer, exit = st.exit_timer, seed = st.mission_seed }
	local settings = global("DifficultySettings")
	if settings and st.mission_seed then
		local params = settings.get_lookup_mission_params(st.mission_seed)
		out.enemy, out.difficulty, out.environment = params.enemy, params.difficulty, params.environment
		out.type = settings.get_mission_type(st.mission_seed)
	end
	return out
end

-- The war as this game knows it. nil until it's heard from the server. It's a big table: ask once.
function D.campaign()
	local manager = global("CampaignManager")
	if B.type(manager) ~= "table" or not manager:is_valid() then return nil end
	return { status = copy(manager:get_campaign_status(), 3), statistics = copy(manager:get_campaign_statistics(), 3), season = manager:get_current_season() }
end

-- sub: the step of the end screens. values: every plain field of the state, by the game's names.
function D.mission_end()
	local st, name = state_of()
	if name ~= "mission_end" or B.type(st) ~= "table" then return nil end
	return { sub = B.type(st.current_state) == "table" and st.current_state.name or nil, values = copy(st) }
end

-- One frame.

local function fade_level()
	local st = state_of()
	local manager = B.type(st) == "table" and st.fade_manager
	local level = B.type(manager) == "table" and manager.current
	if B.type(level) ~= "number" then return 1 end
	return 1 - B.max(0, B.min(level, 1))
end

function M.tick()
	M.stamp = M.stamp + 1
	M.scope = nil
	if B.next(M.known) ~= nil then install_primitives() end
	install_points()
	if B.next(M.remaps) ~= nil then install_hits() end
	local script = global("Script")
	local vectors, quaternions, matrices = script.temp_count()
	local cur = M.current()
	M.can_draw = cur.state ~= "game" or M.cleared.mission_draw
	M.fade = fade_level()
	if M.solo_by and M.solo_by.off then M.solo_ids, M.solo_by = nil, nil end
	run_screens(cur, global("ENGINE_DT") or 0)
	refresh_mutes()
	script.set_temp_count(vectors, quaternions, matrices)
end

local function protected_tick()
	if not M.wanted then return end
	local ok, err = B.pcall(M.tick)
	if not ok then log.limited("ui frame", "ui: " .. B.tostring(err)) end
end

-- Only what the deploy put in the boot bundle: a font or material that isn't loaded is assumed fatal.
local function load_fonts(deploy)
	for _, file in B.ipairs(deploy.font_files or {}) do
		B.insert(GUIS.fonts.materials, file)
		M.font_files[file] = true
	end
	for _, path in B.ipairs(deploy.font_paths or {}) do M.font_paths[path] = true end
	for name, font in B.pairs(FONTS) do
		if font.instead and not M.font_paths[font.path] then FONTS[name] = FONTS[font.instead] end
	end
end

-- A font a mod ships in the boot bundle, named by its resource. tools/bake_font.py gives the material
-- the font's last name, in fonts.material beside it. false when the deploy doesn't have it.
function M.font(id, opts)
	opts = opts or {}
	local base, name = B.match(B.tostring(opts.path), "^(.*)/([^/]+)$")
	if B.type(id) ~= "string" or FONTS[id] or not name or not M.font_paths[opts.path] or not M.font_files[base .. "/fonts"] then
		log.limited("ui font " .. B.tostring(id), "ui: can't add font " .. B.tostring(id))
		return false
	end
	FONTS[id] = { path = opts.path, material = name, size = opts.size or 24, gui = "fonts" }
	return true
end

-- A sub-material of a material a mod ships in the boot bundle, for kit:art. opts.path: the material's
-- resource name. false when the deploy doesn't have it: a name the gui doesn't list is assumed fatal.
function M.art(id, opts)
	opts = opts or {}
	if B.type(id) ~= "string" or not M.font_files[opts.path] then
		log.limited("ui art " .. B.tostring(id), "ui: can't add art " .. B.tostring(id))
		return false
	end
	M.arts[id] = true
	return true
end

function M.setup(core, hooks, session)
	M.core, M.session = core, session
	load_fonts(core.deploy)
	hooks.on_frame(protected_tick)
end

function M.api()
	return {
		current = M.current,
		on = M.on,
		off = M.off,
		mute = M.mute,
		unmute = M.unmute,
		scope = M.scope_of,
		data = M.data,
		canvas = function(id, opts)
			opts = opts or {}
			opts.owner = nil
			return M.canvas(id, opts)
		end,
		frame = M.frame,
		safe = M.safe,
		remap_hits = M.remap_hits,
		prompt = M.prompt,
		icons = M.icons,
		glyphs = M.glyph_names,
		font = M.font,
		art = M.art,
		solo = M.solo,
		soloed = M.soloed,
	}
end

return M
