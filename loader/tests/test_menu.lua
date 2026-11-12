-- obsidian.menu, obsidian.toast and overlays: what the loader still does now that Obsidian API draws the
-- in-game window. The window itself is tested in test_api_menu.lua.
local E = require "fake_engine"
local F = require "fake_ui"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

-- Runs mods ({ id, source, scope, requires }) in that order, with Obsidian API where `api` says: "first",
-- "last" or nil for none.
local function start(list, api)
	local mods, deploy = {}, {}
	local function add_api()
		local b = G.bundled("obsidian.api", mods)
		deploy[#deploy + 1] = E.mod("obsidian.api", { scope = "client", texts = b.texts, keybinds = b.keybinds, name = b.name })
	end
	if api == "first" then add_api() end
	for _, m in ipairs(list) do
		mods["obsidian/mods/" .. m.id .. "/main"] = m.source
		deploy[#deploy + 1] = E.mod(m.id, { scope = m.scope or "client", requires = m.requires, name = { en = m.name or m.id }, texts = m.texts })
	end
	if api == "last" then add_api() end
	local e = E.new({ deploy = E.deploy({ mods = deploy }), mods = mods })
	e:boot()
	e:start_frames()
	local ui = F.install(e)
	ui.frame()
	return e, ui
end

local function one(source)
	return start({ { id = "m", source = source } })
end

local function api_menu(e)
	return e.loaded["obsidian/mods/obsidian.api/lib/menu"]
end

local OLD_STYLE = [[
	obsidian.menu.add(function() return obsidian.text("title") end, function(page)
		page:action("Who", function() return "caller " .. tostring(obsidian.caller()) end)
	end, { icon = "player" })
]]

test("obsidian_menu_pages_go_to_the_api_window_as_the_mod_that_added_them", function()
	local e, ui = start({ { id = "old", name = "Old Mod", requires = { "obsidian.api" }, texts = { en = { title = "Old page" } }, source = OLD_STYLE } }, "first")
	local obsidian = rawget(_G, "obsidian")
	E.eq(obsidian.menu.open(), true, "opened through the loader")
	E.eq(api_menu(e).shown, true, "in Obsidian API's window")
	E.eq(obsidian.menu.is_open(), true)
	ui.frame()
	E.eq(ui.shown("Old page"), true, "the title in the mod's own words")
	E.eq(ui.shown("Old Mod"), true, "in the mod's folder")
	E.eq(api_menu(e).roots[1].owner, "old")
	E.eq(api_menu(e).roots[1].icon, "player")
	ui.tap("enter")
	ui.tap("enter")
	ui.frame()
	E.eq(ui.shown("caller old"), true, "its actions run as that mod")
	obsidian.menu.toggle()
	E.eq(obsidian.menu.is_open(), false)
end)

test("a_page_added_before_obsidian_api_started_waits_for_it", function()
	local e, ui = start({ { id = "early", name = "Early Mod", texts = { en = { title = "Early page" } }, source = OLD_STYLE } }, "last")
	local M = api_menu(e)
	local own = 0
	for _, r in ipairs(M.roots) do if r.owner == "early" then own = own + 1 end end
	E.eq(own, 1, "handed over once every mod had started")
	E.eq(M.roots[1].owner, "early", "as the mod that added it")
	rawget(_G, "obsidian").menu.open()
	ui.frame()
	E.eq(ui.shown("Early page"), true)
end)

test("without_obsidian_api_there_is_no_window_and_the_log_says_why", function()
	local e, ui = one([[obsidian.menu.add("P", function(page) page:label("x") end)]])
	local menu = rawget(_G, "obsidian").menu
	E.eq(menu.open(), false)
	E.eq(menu.is_open(), false)
	menu.toggle()
	menu.close()
	E.eq(#e.loaded["obsidian/loader/menu"].pending, 1, "the page waits")
	E.has(e:log(), "menu: no window to open: Obsidian API 1.15.0 or newer draws it")
end)

test("obsidian_info_names_a_mod_in_the_games_language", function()
	rawset(_G, "LocalizationManager", { current_language = "fr" })
	local e = start({ { id = "m", source = [[
		rawset(_G, "SELF_INFO", obsidian.info())
		rawset(_G, "API_INFO", obsidian.info("obsidian.api"))
		rawset(_G, "NONE", obsidian.info("nobody") == nil)
	]] } }, "first")
	local mine = rawget(_G, "SELF_INFO")
	E.eq(mine.id, "m")
	E.eq(mine.name, "m", "English when the language has none")
	E.eq(mine.scope, "client")
	E.eq(mine.version, "1.0.0")
	E.eq(rawget(_G, "API_INFO").name, "Obsidian API")
	E.eq(rawget(_G, "NONE"), true, "a mod that isn't running")
	e:core().mods_by_id.m.mod.name = { en = "English", fr = "Français" }
	E.eq(rawget(_G, "obsidian").info("m").name, "Français")
end)

test("toasts_show_then_expire", function()
	local e, ui = one("return true")
	rawget(_G, "obsidian").toast("Hello", 0.05)
	ui.frame()
	E.eq(ui.shown("Hello"), true)
	for _ = 1, 5 do ui.frame() end
	E.eq(ui.shown("Hello"), false, "gone")
end)

test("toasts_sit_at_the_top_left_newest_lower_down", function()
	local e, ui = one("return true")
	rawget(_G, "obsidian").toast("First")
	rawget(_G, "obsidian").toast("Second")
	ui.frame()
	local first, second = ui.find("First"), ui.find("Second")
	E.eq(first.pos.x < 200, true, "on the left")
	E.eq(first.pos.y > 900, true, "at the top of a 1080p screen")
	E.eq(second.pos.y < first.pos.y, true)
end)

test("with_obsidian_api_running_a_toast_goes_to_its_window", function()
	local e, ui = start({ { id = "m", source = "return true" } }, "first")
	rawget(_G, "obsidian").toast("Through the API")
	E.eq(#e.loaded["obsidian/loader/menu"].toasts, 0, "not drawn by the loader")
	E.eq(api_menu(e).toasts[1].text, "Through the API")
end)

test("the_gui_is_destroyed_before_the_game_shuts_down", function()
	local e, ui = one("return true")
	local down = 0
	rawset(_G, "shutdown", function() down = down + 1 end)
	rawget(_G, "obsidian").toast("x")
	ui.frame()
	rawget(_G, "shutdown")()
	E.eq(down, 1, "the game's shutdown still runs")
	E.eq(ui.destroyed, 1)
end)

test("nothing_touches_the_game_until_a_toast_or_overlay_is_used", function()
	local e, ui = one("return true")
	ui.frame()
	E.eq(ui.gui_args, nil, "no gui made for a mod that never uses them")
end)

test("an_overlay_gets_the_larger_font_on_a_large_screen", function()
	local e, ui = one([[
		obsidian.menu.overlay(function(draw, w, h) draw.text("Big", 10, h - 20) end)
	]])
	ui.height = 2160
	ui.frame()
	local t = ui.find("Big", true)
	E.eq(t.font, "foundation/fonts/debug_medium")
	E.eq(t.material, "debug_medium")
	E.eq(t.size, 32)
end)

test("an_overlay_draws_every_frame", function()
	local e, ui = one([[
		obsidian.menu.overlay(function(draw, w, h) draw.text("Speed " .. w, 10, h - 20) end)
	]])
	ui.frame()
	E.eq(ui.shown("Speed 1920"), true)
end)

return T
