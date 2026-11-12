-- Obsidian API's Script Profiler: the panel over the game, its page in the F6 window, and the sums.
local E = require "fake_engine"
local F = require "fake_ui"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local PROFILER = "obsidian/mods/obsidian.api/lib/profiler"

-- A client mod that spends about 2 ms a frame of its own.
local SLOW = [[
	obsidian.on_frame(function() local t = os.clock() while (os.clock() - t) * 1000 < 2 do end end)
]]

local function start(opts)
	local mods = {}
	local api = G.bundled("obsidian.api", mods)
	mods["obsidian/mods/slow/main"] = SLOW
	local deploy = {
		E.mod("obsidian.api", { scope = "client", texts = api.texts, keybinds = api.keybinds, name = api.name }),
		E.mod("slow", { scope = "client", requires = { "obsidian.api" }, name = { en = "Slow Mod" } }),
	}
	local e = E.new({ deploy = E.deploy({ mods = deploy }), mods = mods })
	e:boot()
	e:start_frames()
	local ui = F.install(e, opts or {})
	ui.frame()
	return e, ui, e.loaded[PROFILER]
end

test("hidden_by_default_and_not_measuring", function()
	local e, ui, p = start()
	for _ = 1, 40 do ui.frame() end
	E.eq(p.get("show"), "hidden")
	E.eq(rawget(_G, "obsidian").profiler.running(), false, "no cost while hidden")
	E.eq(ui.shown("FPS cost"), false)
end)

test("always_lists_each_mod_with_its_time_and_fps_cost", function()
	local e, ui, p = start()
	p.values.show = "always"
	-- The first result comes after half a second of frames.
	for _ = 1, 600 do
		ui.frame()
		if ui.shown("Frame ") then break end
	end
	E.eq(rawget(_G, "obsidian").profiler.running(), true)
	assert(ui.shown("FPS cost"), "the column titles")
	assert(ui.shown("Slow Mod"), "the mod by name")
	assert(ui.shown("Frame "), "the frame line")
	E.eq(ui.shown("Peak ms"), false, "basic detail")
	p.values.detail = "detailed"
	ui.frame()
	assert(ui.shown("Peak ms") and ui.shown("All scripts"), "detailed adds columns and the total")
	-- The background is black at 20%.
	local back = false
	for _, r in ipairs(ui.rects) do
		local c = r.color
		if type(c) == "table" and c[1] == 51 and c[2] == 0 and c[3] == 0 and c[4] == 0 then back = true end
	end
	assert(back, "a 20% black background")
	p.values.show = "hidden"
	ui.frame()
	E.eq(rawget(_G, "obsidian").profiler.running(), false, "hidden stops measuring")
end)

test("pause_menu_only_shows_in_the_pause_menu_or_the_f6_window", function()
	local p = select(3, start())
	E.eq(p.wanted("pause", "hud", false), false)
	E.eq(p.wanted("pause", "main", false), true)
	E.eq(p.wanted("pause", "hud", true), true, "with the F6 window open")
	E.eq(p.wanted("always", "hud", false), true)
	E.eq(p.wanted("hidden", "main", true), false)
end)

test("fps_cost_and_rows", function()
	local p = select(3, start())
	-- 20 ms a frame is 50 FPS; without 4 ms of it, 62.5.
	assert(math.abs(p.fps_cost(20, 4) - 12.5) < 1e-9)
	E.eq(p.fps_cost(20, 0), 0)
	local result = { frame_ms = 10, mods = { { id = "a", ms = 2, peak_ms = 3, calls = 4 }, { id = "b", ms = 1, peak_ms = 1, calls = 1 } } }
	local rows = p.rows(result, 1)
	E.eq(#rows, 1, "cut to the rows asked for")
	E.eq(rows[1].id, "a")
	E.eq(#p.rows(result, 0), 2, "0 lists all")
end)

test("the_page_is_in_the_f6_window", function()
	local e = start()
	local menu = e.loaded["obsidian/mods/obsidian.api/lib/menu"]
	local found = false
	for _, root in ipairs(menu.roots) do
		if root.title == "Script Profiler" then found = true end
	end
	assert(found, "a Script Profiler page")
end)

return T
