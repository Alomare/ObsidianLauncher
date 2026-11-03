-- obsidian.menu and obsidian.toast. The in-game window belongs to Obsidian API (api.menu, from API 1.15.0),
-- so it changes with the API's releases. obsidian.menu hands each call to it as the mod that made the
-- call: one obsidian table serves every mod, and the loader sets whose code is running before it calls
-- any of it, so the API sees the right mod. A page added before the API has started waits here until it
-- has. With no API running, there's no window: the pages wait, and open() answers false.
--
-- Toasts go to the API's window while it runs, so they sit beside it, and are drawn here otherwise, so
-- the loader's own notices always show. Overlays stay here too: debug-font text and boxes on a gui of
-- their own. Gui, World and Color are read through _G each frame: the engine defines them, and the
-- game's strict mode forbids touching undeclared globals by name from here.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local ui = require("obsidian/loader/ui")

-- The mod whose exports.menu draws the window.
local PROVIDER = "obsidian.api"
local MATERIAL = "foundation/gui/gui"
local LAYER_BACK, LAYER_TOAST = 880, 896
local CUT_MARK = "..."
local TOAST_H, TOAST_GAP, WIDTHS_KEPT = 28, 16, 512

local WHITE, GOLD, DARK, PANEL = { 236, 235, 231 }, { 240, 180, 41 }, { 15, 17, 19 }, { 41, 44, 48, 250 }
local BODY = { font = "ui", size = 18 }

local M = { pending = {}, toasts = {}, overlays = {}, gui = nil, kit = nil, widths = {}, width_count = 0 }

local function global(name)
	return B.rawget(B.G, name)
end

local function color(c, alpha)
	return global("Color")(alpha or 255, c[1], c[2], c[3])
end

-- The API's menu, while that mod runs and has one.
local function provider()
	local state = M.core and M.core.mods_by_id[PROVIDER]
	local exports = state and not state.off and state.exports
	local menu = B.type(exports) == "table" and exports.menu
	if B.type(menu) == "table" and B.type(menu.add) == "function" then return menu end
	return nil
end

-- Hands the waiting pages over, each as the mod that added it.
function M.flush()
	local menu = provider()
	if not menu or #M.pending == 0 then return end
	local list = M.pending
	M.pending = {}
	for _, p in B.ipairs(list) do
		local core, previous = M.core, M.core.current
		core.current = p.state
		local ok, err = B.pcall(menu.add, p.title, p.build, p.opts)
		core.current = previous
		if not ok then log.limited("menu add", "menu: a page of " .. (p.state and p.state.id or "?") .. " failed to go in: " .. B.tostring(err)) end
	end
end

local function no_window()
	log.limited("menu none", "menu: no window to open: Obsidian API 1.15.0 or newer draws it")
end

-- The in-game window's calls, handed to the API.
local function forward(name, fallback)
	return function(...)
		local menu = provider()
		if not menu or B.type(menu[name]) ~= "function" then return fallback(...) end
		M.flush()
		return menu[name](...)
	end
end

function M.add(title, build, opts)
	if B.type(build) ~= "function" then return end
	local menu = provider()
	if menu then
		M.flush()
		return menu.add(title, build, opts)
	end
	B.insert(M.pending, { state = M.core.current, title = title, build = build, opts = opts })
end

M.open = forward("open", function()
	no_window()
	return false
end)
M.close = forward("close", function() end)
M.toggle = forward("toggle", no_window)
M.is_open = forward("is_open", function() return false end)

-- The loader's own toasts: where there's no API window, at the top left, in the window's look.

local function width_of(s)
	local w = M.widths[s]
	if not w then
		w = M.kit:measure(s, BODY)
		if M.width_count >= WIDTHS_KEPT then M.widths, M.width_count = {}, 0 end
		if w > 0 then M.widths[s], M.width_count = w, M.width_count + 1 end
	end
	return w
end

-- Cut to a width, in whole letters. The game's fonts lack ~, so dots mark the cut.
local function fit(s, width)
	if width_of(s) <= width then return s end
	local chars = {}
	for ch in B.gmatch(s, "[%z\1-\127\192-\255][\128-\191]*") do chars[#chars + 1] = ch end
	local low, high = 0, #chars
	while low < high do
		local at = B.floor((low + high + 1) / 2)
		if width_of(B.concat(chars, "", 1, at) .. CUT_MARK) <= width then low = at else high = at - 1 end
	end
	return B.concat(chars, "", 1, low) .. CUT_MARK
end

function M.toast(text, seconds)
	local menu = provider()
	if menu and B.type(menu.toast) == "function" then return menu.toast(text, seconds) end
	B.insert(M.toasts, { text = B.tostring(text), left = seconds or 4 })
end

local function draw_toasts(dt)
	for i = #M.toasts, 1, -1 do
		local t = M.toasts[i]
		t.left = t.left - dt
		if t.left <= 0 then B.remove(M.toasts, i) end
	end
	local kit = M.kit
	local w, h = global("Application").resolution()
	local left = kit:point(0, 0)
	local right, top = kit:point(w, h)
	local x, y = left + TOAST_GAP, top - TOAST_GAP - TOAST_H
	for _, t in B.ipairs(M.toasts) do
		local s = fit(t.text, right - left - 80)
		local tw = width_of(s) + 26
		kit:opacity(B.min(t.left * 2, 1))
		kit.z = LAYER_TOAST
		kit:panel({ x, y, tw, TOAST_H, cut = 6, corners = "br", fill = PANEL })
		kit.z = LAYER_TOAST + 1
		kit:rect(x, y, 3, TOAST_H, GOLD)
		BODY.color = WHITE
		kit:text(s, x + 13, y + (TOAST_H - BODY.size * 0.7) / 2, BODY)
		y = y - TOAST_H - 4
	end
	kit:opacity(1)
end

-- Overlays take pixels and the debug fonts, on a gui of their own.
local function overlay_font()
	local _, h = global("Application").resolution()
	if h >= 1400 then return "foundation/fonts/debug_medium", 32, "debug_medium", 15, 36 end
	return "foundation/fonts/debug_small", 16, "debug_small", 8, 20
end

local function rect(x, y, w, h, layer, c, alpha)
	global("Gui").rect(M.gui, global("Vector3")(x, y, layer), global("Vector2")(w, h), color(c, alpha))
end

local function text(s, x, y, layer, c, alpha)
	local path, size, material = overlay_font()
	global("Gui").text(M.gui, s, path, size, material, global("Vector3")(x, y, layer), color(c, alpha))
end

local function ensure_gui()
	if M.gui then return true end
	local boot = global("Boot")
	local world = B.type(boot) == "table" and boot.brightness_world
	if not world then return false end
	M.world = world
	M.gui = global("World").create_screen_gui(world, "material", MATERIAL, "immediate")
	-- The gui lives on a world the game releases at shutdown, so it goes first.
	local shutdown = global("shutdown")
	if B.type(shutdown) == "function" then
		B.rawset(B.G, "shutdown", function(...)
			if M.gui then B.pcall(global("World").destroy_gui, M.world, M.gui) end
			M.gui = nil
			return shutdown(...)
		end)
	end
	return true
end

local function allowed(state)
	return state == nil or state.mod.scope == "client" or M.session.allows(state)
end

local function run(state, fn, ...)
	local core, previous = M.core, M.core.current
	core.current = state
	local ok, err = B.pcall(fn, ...)
	core.current = previous
	if not ok then log.limited("menu overlay", "menu error (" .. (state and state.id or "loader") .. ", overlay): " .. B.tostring(err)) end
end

function M.frame()
	if #M.toasts == 0 and #M.overlays == 0 then return end
	local script = global("Script")
	local vectors, quaternions, matrices = script.temp_count()
	if #M.toasts > 0 then
		M.kit = M.kit or ui.system_canvas("toasts")
		draw_toasts(global("ENGINE_DT") or 0)
	end
	if #M.overlays > 0 and ensure_gui() then
		local w, h = global("Application").resolution()
		for _, o in B.ipairs(M.overlays) do
			if allowed(o.state) then run(o.state, o.fn, M.draw_api, w, h) end
		end
	end
	script.set_temp_count(vectors, quaternions, matrices)
end

-- A failing overlay or a bad engine call must never reach the frame hook again and again.
local function protected_frame()
	local ok, err = B.pcall(M.frame)
	if not ok then log.limited("menu frame", "menu: " .. B.tostring(err)) end
end

M.draw_api = {
	text = function(s, x, y, c) text(B.tostring(s), x, y, LAYER_BACK - 2, c or WHITE) end,
	rect = function(x, y, w, h, c, alpha) rect(x, y, w, h, LAYER_BACK - 4, c or DARK, alpha or 180) end,
	char_width = function()
		local _, _, _, advance = overlay_font()
		return advance
	end,
	line_height = function()
		local _, _, _, _, line = overlay_font()
		return line
	end,
}

function M.setup(core, hooks, session)
	M.core, M.session = core, session
	hooks.on_frame(protected_frame)
end

function M.api()
	return {
		-- A page in the in-game window, which Obsidian API draws. title: text, or a function that
		-- answers it. build(page) runs each time the page opens. opts.icon: one of obsidian.ui.glyphs().
		add = M.add,
		open = M.open,
		close = M.close,
		toggle = M.toggle,
		is_open = M.is_open,
		-- fn(draw, width, height) runs every frame, under the same rules as on_frame. Origin is the
		-- bottom left of the screen, in pixels.
		overlay = function(fn)
			if B.type(fn) == "function" then B.insert(M.overlays, { state = M.core.current, fn = fn }) end
		end,
	}
end

return M
