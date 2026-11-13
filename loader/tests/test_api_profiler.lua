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

test("a_long_name_rests_scrolls_rests_and_comes_back", function()
	local p = select(3, start())
	E.eq(p.scroll_offset(0, 3, 10, 1), 0, "a name that fits stays")
	E.eq(p.scroll_offset(20, 0.5, 10, 1), 0, "rests at the start")
	assert(math.abs(p.scroll_offset(20, 2, 10, 1) - 10) < 1e-9, "half way after a second of moving")
	E.eq(p.scroll_offset(20, 3.5, 10, 1), 20, "rests at the end")
	assert(math.abs(p.scroll_offset(20, 5, 10, 1) - 10) < 1e-9, "and comes back")
	E.eq(p.DEFAULTS.corner, "top_left")
end)

test("the_log_report_names_each_mod_s_costliest_places", function()
	local p = select(3, start())
	local lines = p.report({ frame_ms = 20, frames = 300, total_ms = 4, clock = "qpc", mods = {
		{ id = "obsidian.api", ms = 3, peak_ms = 6, calls = 40, sites = { { site = "frame: bots", ms = 2, calls = 1 }, { site = "hook Hud.draw_plugins", ms = 0.5, calls = 12 } } },
		{ id = "tiny", ms = 0.001, peak_ms = 0, calls = 1, sites = {} },
	} })
	assert(lines[1]:find("frame 20.00 ms (50 FPS) over 300 frames", 1, true), lines[1])
	E.eq(#lines, 2, "a mod below the minimum is left out")
	assert(lines[2]:find("frame: bots 2.00 ms x1, hook Hud.draw_plugins 0.50 ms x12", 1, true), lines[2])
	assert(lines[2]:find("(obsidian.api): 3.00 ms a frame, slowest 6.00, 40 calls", 1, true), lines[2])
end)

test("measuring_writes_a_report_with_the_parts_of_obsidian_api", function()
	local e, ui, p = start()
	p.values.show = "always"
	p.LOG_EVERY = 0.3
	for _ = 1, 900 do
		ui.frame()
		if e:log("obsidianapi"):find("profiler:   Obsidian API", 1, true) or e:log("obsidianapi"):find("(obsidian.api)", 1, true) then break end
	end
	local log = e:log("obsidianapi")
	E.has(log, "profiler: measuring")
	E.has(log, "profiler: frame ")
	-- The parts of Obsidian API's own frame are measured one by one.
	local r = rawget(_G, "obsidian").profiler.read()
	local api
	for _, m in ipairs(r.mods) do if m.id == "obsidian.api" then api = m end end
	local parts = {}
	for _, s in ipairs(api.sites) do parts[s.site] = true end
	local framed, names = false, {}
	for site in pairs(parts) do
		names[#names + 1] = site
		if site:find("^frame: ") or site:find("^profiler: ") or site:find("^menu: ") then framed = true end
	end
	assert(framed, "the parts by name: " .. table.concat(names, ", "))
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
