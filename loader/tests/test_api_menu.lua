-- Obsidian API's in-game window (api.menu) and its toasts, through mods, as a player would drive them.
local E = require "fake_engine"
local F = require "fake_ui"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local MENU = "obsidian/mods/obsidian.api/lib/menu"

-- Obsidian API, then each mod of `list` ({ id, source, scope, name, texts }), all requiring it. The API
-- runs as a session mod when one of them is. opts: lobby, state.
local function start_mods(list, opts)
	opts = opts or {}
	local mods = {}
	local api = G.bundled("obsidian.api", mods)
	local session, served = false, {}
	for _, m in ipairs(list) do
		session = session or m.scope == "session"
		if m.scope == "expansion" then served[#served + 1] = m.id end
	end
	-- A library only expansions require is an expansion library, listing them (core/src/loader.rs).
	local api_scope = session and "session" or (#served > 0 and "expansion") or "client"
	local deploy = { E.mod("obsidian.api", { scope = api_scope, expansions = #served > 0 and served or nil, texts = api.texts, keybinds = api.keybinds, name = api.name }) }
	for _, m in ipairs(list) do
		mods["obsidian/mods/" .. m.id .. "/main"] = m.source
		deploy[#deploy + 1] = E.mod(m.id, { scope = m.scope or "client", expansions = m.scope == "expansion" and { m.id } or nil,
			requires = { "obsidian.api" }, name = { en = m.name or m.id }, texts = m.texts })
	end
	local own = {}
	for _, id in ipairs(served) do own[id] = "abcdef012345" end
	local e = E.new({ deploy = E.deploy({ mods = deploy, has_session = session, expansions = own }), mods = mods, lobby = opts.lobby })
	e:boot()
	e:start_frames()
	local ui = F.install(e, opts)
	-- The game runs frames before anyone opens the window.
	ui.frame()
	return e, ui
end

local function start(source, opts)
	opts = opts or {}
	return start_mods({ { id = "m", source = source, scope = opts.scope, name = "Test Mod" } }, opts)
end

local function menu()
	return rawget(_G, "obsidian").import("obsidian.api").menu
end

local function menu_of(e)
	return e.loaded[MENU]
end

local PAGE = [[
	local api = obsidian.import("obsidian.api")
	VALUES = { god = false, speed = 5, mode = "a", runs = 0 }
	api.menu.add("Player", function(page)
		page:header("Section")
		page:toggle("God mode", { get = function() return VALUES.god end, set = function(v) VALUES.god = v end })
		page:number("Speed", { get = function() return VALUES.speed end, set = function(v) VALUES.speed = v end, min = 1, max = 8, step = 2 })
		page:choice("Mode", { choices = { { id = "a", label = "First" }, { id = "b", label = "Second" } },
			get = function() return VALUES.mode end, set = function(v) VALUES.mode = v end })
		page:action("Heal", function() VALUES.runs = VALUES.runs + 1 return "Healed" end)
		page:page("More", function(p) p:action("Deep", function() VALUES.deep = true end) end)
	end)
]]

local function values() return rawget(_G, "VALUES") end

-- Opens the window and goes into the first page, which is open beside the list.
local function open_player(e, ui)
	menu().open()
	ui.tap("enter")
end

local function open_root(ui)
	menu().open()
	ui.frame()
end

-- Clicks on a drawn text, a little inside it.
local function click_on(ui, needle, exact)
	local t = ui.find(needle, exact)
	ui.click(t.pos.x + 2, t.pos.y + 4)
	ui.frame()
end

-- The middle of a part of the window, in the pixels the mouse reads.
local function middle(e, area)
	local kit = menu_of(e).kit
	return kit.ox + (area.x + area.w / 2) * kit.scale, kit.oy + (area.y + area.h / 2) * kit.scale
end

local function row_named(e, label)
	for _, row in ipairs(menu_of(e).hits.rows) do
		if row.it.label == label then return row end
	end
	error("no row: " .. label, 2)
end

local function nav_named(e, needle)
	for _, area in ipairs(menu_of(e).hits.nav) do
		if area.folder == needle or (area.root and area.root.title == needle) then return area end
	end
	error("no row in the list: " .. needle, 2)
end

local function near(got, want, what)
	if math.abs(got - want) > 0.001 then error(what .. ": expected " .. want .. ", got " .. got, 2) end
end

local function rows_shown(ui)
	local n = 0
	for _, t in ipairs(ui.texts) do
		if string.find(t.text, "^Row %d+%.") then n = n + 1 end
	end
	return n
end

-- The title bar's words: the text drawn at the left end of the bar, where the margin ends.
local function title_of(e, ui)
	local M = menu_of(e)
	local kit = M.kit
	for _, t in ipairs(ui.texts) do
		if math.abs(t.pos.x - kit.ox - (M.win.x + 14) * kit.scale) < 0.01 then return t.text end
	end
	return nil
end

test("opens_and_closes_taking_every_input_user", function()
	local e, ui = start(PAGE)
	E.eq(menu().open(), true, "open")
	ui.frame()
	E.eq(ui.focus.keyboard_mouse, "obsidian_api_menu", "keyboard")
	E.eq(ui.focus.user1, "obsidian_api_menu", "pad")
	E.eq(ui.gui_args[1], "brightness", "drawn on the brightness world")
	E.eq(ui.gui_args[3], "foundation/gui/gui", "the foundation material, always loaded")
	E.eq(ui.gui_args[4], "immediate")
	E.eq(ui.shown("Player"), true, "the list has the page")
	menu().close()
	E.eq(ui.focus.keyboard_mouse, nil, "released")
	E.eq(menu().is_open(), false)
	ui.frame()
	E.eq(#ui.texts, 0, "nothing drawn once closed")
end)

test("focus_is_taken_back_every_frame", function()
	local e, ui = start(PAGE)
	menu().open()
	ui.focus.keyboard_mouse = "input_component"
	ui.frame()
	E.eq(ui.focus.keyboard_mouse, "obsidian_api_menu")
end)

test("does_not_open_in_logos_cutscene_transition_or_during_chat", function()
	local e, ui = start(PAGE)
	for _, name in ipairs({ "logos", "cutscene", "transition" }) do
		ui.state = name
		E.eq(menu().open(), false, name)
	end
	ui.state = "game"
	ui.state_object = { text_chat = { active = true } }
	E.eq(menu().open(), false, "chat typing")
	ui.state_object = { text_chat = { active = false } }
	E.eq(menu().open(), true, "chat idle")
end)

test("a_state_change_closes_the_menu", function()
	local e, ui = start(PAGE)
	menu().open()
	ui.frame()
	ui.state = "mission_end"
	ui.frame()
	E.eq(menu().is_open(), false)
	E.eq(ui.focus.keyboard_mouse, nil, "input given back")
end)

test("navigation_skips_headers_and_wraps", function()
	local e, ui = start(PAGE)
	open_player(e, ui)
	ui.tap("enter")
	E.eq(values().god, true, "first selectable row is the toggle, under the header")
	ui.tap("up")
	ui.tap("enter")
	E.eq(values().runs, 0, "up from the first row wraps to the last, a page")
end)

test("toggle_number_and_choice", function()
	local e, ui = start(PAGE)
	open_player(e, ui)
	ui.tap("right")
	E.eq(values().god, true, "right flips a toggle")
	ui.tap("down")
	ui.tap("right")
	E.eq(values().speed, 7, "right adds a step")
	ui.tap("right")
	E.eq(values().speed, 8, "clamped to the maximum")
	ui.tap("left")
	E.eq(values().speed, 6, "left takes a step away")
	ui.down["left shift"] = true
	ui.tap("left")
	ui.down["left shift"] = nil
	E.eq(values().speed, 1, "shift is ten steps, clamped to the minimum")
	ui.tap("down")
	ui.tap("right")
	E.eq(values().mode, "b")
	ui.frame()
	E.eq(ui.shown("Second"), true, "the choice shows its label")
end)

test("typed_number_entry_is_clamped_and_can_be_cancelled", function()
	local e, ui = start(PAGE)
	open_player(e, ui)
	ui.tap("down")
	ui.tap("enter")
	ui.typed = { "3", "x", "enter" }
	ui.frame()
	E.eq(values().speed, 3, "letters are dropped")
	ui.tap("enter")
	ui.typed = { "9", "9", "enter" }
	ui.frame()
	E.eq(values().speed, 8, "clamped")
	ui.tap("enter")
	ui.typed = { "2", "esc" }
	ui.frame()
	E.eq(values().speed, 8, "cancelled")
	E.eq(menu().is_open(), true, "Esc in a field leaves the window open")
end)

test("actions_toast_their_result_and_pages_nest", function()
	local e, ui = start(PAGE)
	open_player(e, ui)
	ui.tap("down")
	ui.tap("down")
	ui.tap("down")
	ui.tap("enter")
	E.eq(values().runs, 1)
	ui.frame()
	E.eq(ui.shown("Healed"), true, "toast")
	ui.tap("down")
	ui.tap("enter")
	ui.tap("enter")
	E.eq(values().deep, true, "sub page")
	ui.frame()
	E.eq(ui.shown("Player / More"), true, "breadcrumb")
	ui.tap("esc")
	ui.frame()
	E.eq(ui.shown("Heal"), true, "back to the page above")
	ui.tap("esc")
	E.eq(menu().is_open(), true, "Esc on a page goes to the list of pages")
	ui.tap("esc")
	E.eq(menu().is_open(), false, "and from there it closes")
end)

test("a_failing_action_is_a_toast_and_the_mod_stays_on", function()
	local e, ui = start([[
		local api = obsidian.import("obsidian.api")
		api.menu.add("P", function(page)
			page:action("Bad", function() error("boom") end)
			page:toggle("Broken", { get = function() error("no") end })
		end)
	]])
	open_player(e, ui)
	for _ = 1, 20 do ui.tap("enter") end
	ui.frame()
	E.eq(ui.shown("boom"), true, "the error shows")
	E.eq(e:core().mods_by_id["m"].off, false, "menu errors never count against the mod")
	E.eq(e:core().mods_by_id["obsidian.api"].off, false, "nor against Obsidian API")
	E.eq(menu().is_open(), true)
end)

test("an_item_can_be_disabled_with_a_reason", function()
	local e, ui = start([[
		local api = obsidian.import("obsidian.api")
		RAN = false
		api.menu.add("P", function(page)
			page:action("Host thing", function() RAN = true end, { enabled = function() return false, "Host only" end })
		end)
	]])
	open_player(e, ui)
	ui.tap("enter")
	ui.frame()
	E.eq(rawget(_G, "RAN"), false)
	E.eq(ui.shown("Host only"), true)
end)

test("an_input_row_passes_the_typed_line_on", function()
	local e, ui = start([[
		local api = obsidian.import("obsidian.api")
		api.menu.add("P", function(page)
			page:input("Command", function(text) rawset(_G, "LINE", text) return "Ran " .. text end)
		end)
	]])
	open_player(e, ui)
	ui.tap("enter")
	ui.typed = { "h", "i", "enter" }
	ui.frame()
	ui.frame()
	E.eq(rawget(_G, "LINE"), "hi")
	E.eq(ui.shown("Ran hi"), true, "toast")
end)

test("a_code_row_takes_arrows_and_wasd_and_passes_words_on", function()
	local e, ui = start([[
		local api = obsidian.import("obsidian.api")
		api.menu.add("P", function(page)
			page:input("Code", function(text) rawset(_G, "CODE", text) end, { code = true })
		end)
	]])
	open_player(e, ui)
	ui.tap("enter")
	ui.tap("up")
	ui.tap("d")
	ui.tap("down")
	ui.tap("backspace")
	ui.typed = { "x", "enter" }
	ui.frame()
	ui.frame()
	E.eq(rawget(_G, "CODE"), "up right", "an arrow, D for right, one taken back; letters typed don't count")
end)

test("filter_narrows_the_list_and_esc_clears_it", function()
	local e, ui = start(PAGE)
	open_player(e, ui)
	ui.tap("tab")
	ui.typed = { "h", "e", "enter" }
	ui.frame()
	ui.frame()
	E.eq(ui.shown("Heal"), true)
	E.eq(ui.shown("Speed"), false, "filtered out")
	ui.tap("enter")
	E.eq(values().runs, 1, "the first match is selected")
	ui.tap("esc")
	ui.frame()
	E.eq(ui.shown("Speed"), true, "filter cleared")
	E.eq(menu().is_open(), true, "first Esc only clears the filter")
end)

test("a_session_page_closes_when_the_session_is_no_longer_cleared", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e, ui = start(PAGE, { scope = "session", lobby = lobby })
	e:frames(31)
	open_player(e, ui)
	ui.frame()
	E.eq(ui.shown("God mode"), true)
	lobby.members = { "me", "vanilla" }
	for _ = 1, 31 do e:frames(1) end
	E.eq(menu().is_open(), false, "closed")
	menu().open()
	ui.frame()
	E.eq(ui.shown("Player"), false, "a session page isn't listed while not cleared")
	E.eq(ui.shown("No mod has added a page."), true)
end)

test("a_client_page_stays_open_beside_players_who_havent_agreed", function()
	local e, ui = start(PAGE)
	open_root(ui)
	require("obsidian/loader/session").state = false
	ui.frame()
	E.eq(menu().is_open(), true, "a client mod doesn't need them to")
	E.eq(ui.shown("God mode"), true)
end)

-- An expansion's pages are its way in, so they show before its content starts, beside anyone.
test("an_expansion_page_shows_outside_its_content", function()
	local e, ui = start(PAGE, { scope = "expansion" })
	open_root(ui)
	require("obsidian/loader/session").state = false
	ui.frame()
	E.eq(menu().is_open(), true)
	E.eq(ui.shown("God mode"), true, "listed with the content not started")
end)

test("chat_stays_shut_while_the_menu_is_open", function()
	local e, ui = start(PAGE)
	local typed = 0
	rawset(_G, "TextChat", { start_typing = function() typed = typed + 1 end })
	e.opts.modules["x"] = true
	rawget(_G, "require")("x")
	rawget(_G, "TextChat").start_typing({})
	E.eq(typed, 1, "closed window: chat opens")
	menu().open()
	rawget(_G, "TextChat").start_typing({})
	E.eq(typed, 1, "open window: refused")
end)

test("toasts_show_then_expire", function()
	local e, ui = start("return true")
	menu().toast("Hello", 0.05)
	rawget(_G, "obsidian").toast("Also", 0.05)
	ui.frame()
	E.eq(ui.shown("Hello"), true)
	E.eq(ui.shown("Also"), true, "obsidian.toast comes here too")
	for _ = 1, 5 do ui.frame() end
	E.eq(ui.shown("Hello"), false, "gone")
end)

test("toasts_sit_at_the_top_left_newest_lower_down", function()
	local e, ui = start("return true")
	menu().toast("First")
	menu().toast("Second")
	ui.frame()
	local first, second = ui.find("First"), ui.find("Second")
	E.eq(first.pos.x < 200, true, "on the left")
	E.eq(first.pos.y > 900, true, "at the top of a 1080p screen")
	E.eq(second.pos.y < first.pos.y, true)
end)

test("with_the_window_open_a_toast_sits_beside_it", function()
	local e, ui = start(PAGE)
	local M = menu_of(e)
	menu().open()
	ui.frame()
	rawget(_G, "obsidian").toast("Done")
	ui.frame()
	local x = (ui.find("Done").pos.x - M.kit.ox) / M.kit.scale
	E.eq(x > M.win.x + M.win.w, true, "clear of the window")
end)

test("a_toast_stays_on_screen_beside_a_wide_window", function()
	local e, ui = start(PAGE)
	local M = menu_of(e)
	menu().open()
	ui.frame()
	M.win.x, M.win.w = 0, 1280
	menu().toast("Done")
	ui.frame()
	local t = ui.find("Done")
	E.eq((t.pos.x + #t.text * t.size * 0.5) / M.kit.scale < 1280, true)
end)

test("long_labels_are_cut_to_the_row_and_never_through_a_letter", function()
	local e, ui = start([[
		local api = obsidian.import("obsidian.api")
		api.menu.add("P", function(page)
			page:action(string.rep("é", 200), function() end)
		end)
	]])
	open_player(e, ui)
	ui.frame()
	local M, cut = menu_of(e), false
	for _, t in ipairs(ui.texts) do
		local _, letters = string.gsub(t.text, "é", "é")
		if letters > 0 then
			cut = true
			E.eq(letters < 200, true, "cut")
			E.eq(string.sub(t.text, -3), "...", "marked as cut")
			E.eq(#t.text, letters * 2 + 3, "whole letters only")
			E.eq((t.pos.x + #t.text * t.size * 0.5) / M.kit.scale < M.win.x + M.win.w, true, "inside the window")
		end
	end
	E.eq(cut, true)
end)

-- A long page, for scrolling and resizing.
local LONG = [[
	local api = obsidian.import("obsidian.api")
	VALUES = {}
	api.menu.add("Long", function(page)
		for i = 1, 90 do
			page:action("Row " .. i .. ".", function() VALUES.ran = i end, { help = "Help for " .. i .. "." })
		end
	end)
]]

test("a_click_opens_a_page_flips_a_toggle_and_runs_an_action", function()
	local e, ui = start(PAGE)
	open_root(ui)
	click_on(ui, "Player")
	E.eq(ui.shown("God mode"), true, "the page opened")
	click_on(ui, "God mode")
	E.eq(values().god, true, "the toggle flipped")
	click_on(ui, "Heal")
	E.eq(values().runs, 1, "the action ran")
	click_on(ui, "Section")
	E.eq(values().runs, 1, "a header does nothing")
end)

test("the_arrows_of_a_number_step_and_holding_repeats", function()
	local e, ui = start(PAGE)
	open_root(ui)
	click_on(ui, "Player")
	ui.press(middle(e, row_named(e, "Speed").minus))
	E.eq(values().speed, 3, "the left arrow takes a step away")
	for _ = 1, 24 do ui.frame() end
	E.eq(values().speed, 3, "no repeat before the first delay")
	for _ = 1, 3 do ui.frame() end
	E.eq(values().speed, 1, "held, it repeats")
	ui.release()
	ui.frame()
	ui.click(middle(e, row_named(e, "Speed").plus))
	E.eq(values().speed, 3, "the right arrow adds a step")
end)

test("the_foot_guides_with_keycaps_and_typing_changes_the_guide", function()
	local e, ui = start(PAGE)
	open_root(ui)
	click_on(ui, "Player")
	ui.frame()
	E.eq(ui.shown("Enter"), true, "a keycap")
	E.eq(ui.shown("Select"), true, "and what it does")
	E.eq(ui.shown("Back"), true)
	click_on(ui, "5", true)
	ui.frame()
	E.eq(ui.shown("Confirm"), true, "typing: the keys that end it")
	E.eq(ui.shown("Select"), false)
end)

test("clicking_the_value_of_a_number_types_it_and_a_click_elsewhere_keeps_it", function()
	local e, ui = start(PAGE)
	open_root(ui)
	click_on(ui, "Player")
	click_on(ui, "5", true)
	E.eq(values().speed, 5, "nothing changes yet")
	ui.typed = { "7" }
	ui.frame()
	E.eq(ui.shown("7_"), true, "what's typed shows in the row itself")
	E.eq(ui.shown("Speed: 7_"), false, "not at the foot")
	ui.click(5, 5)
	E.eq(values().speed, 7, "a click ends typing, as Enter does")
end)

test("a_choice_steps_with_a_click_either_way", function()
	local e, ui = start(PAGE)
	open_root(ui)
	click_on(ui, "Player")
	click_on(ui, "First", true)
	E.eq(values().mode, "b", "a click on it steps it")
	ui.click(middle(e, row_named(e, "Mode").minus))
	ui.frame()
	E.eq(values().mode, "a", "and the arrows go either way")
	ui.click(middle(e, row_named(e, "Mode").plus))
	ui.frame()
	E.eq(values().mode, "b")
end)

test("back_right_click_and_close", function()
	local e, ui = start(PAGE)
	open_root(ui)
	E.eq(ui.shown("God mode"), true, "the first page is open beside the list")
	E.eq(menu_of(e).hits.back, nil, "nothing to go back to")
	click_on(ui, "More")
	E.eq(ui.shown("Deep"), true, "a page inside the page")
	ui.click(middle(e, menu_of(e).hits.back))
	ui.frame()
	E.eq(ui.shown("God mode"), true, "the back button leaves it")
	click_on(ui, "More")
	ui.click(700, 500, "right")
	ui.frame()
	E.eq(ui.shown("God mode"), true, "a right click goes back too")
	ui.click(middle(e, menu_of(e).hits.close))
	E.eq(menu().is_open(), false, "the close button closes")
end)

test("the_window_plays_the_games_menu_sounds_quietly", function()
	local e, ui = start(PAGE)
	local played = {}
	rawset(_G, "TimpaniWorld", {
		has_event = function(world, name) return world == "ui sounds" end,
		trigger_event = function(world, name)
			played[#played + 1] = { world = world, name = name }
			return #played
		end,
		set_parameter = function(_, id, name, value) played[id][name] = value end,
	})
	rawget(_G, "World").timpani_world = function(w) return w == "brightness" and "ui sounds" or nil end
	open_player(e, ui)
	ui.frame()
	E.eq(played[1].name, "ui_select", "opening, then into the page")
	E.eq(played[1].volume, 0.35)
	for _ = 1, 6 do ui.frame() end
	ui.tap("down")
	ui.frame()
	E.eq(played[#played].name, "ui_move", "moving")
	ui.tap("up")
	for _ = 1, 6 do ui.frame() end
	ui.tap("enter")
	ui.frame()
	E.eq(values().god, true)
	E.eq(played[#played].name, "ui_move", "a toggle changes")
	ui.tap("esc")
	ui.frame()
	E.eq(played[#played].name, "ui_back", "back to the list")
	ui.tap("esc")
	ui.frame()
	E.eq(menu().is_open(), false)
	E.eq(played[#played].name, "ui_back", "closing")
	local n = #played
	menu().open()
	ui.state = "menu"
	ui.frame()
	ui.frame()
	E.eq(menu().is_open(), false, "a new screen closes it")
	E.eq(#played, n, "without a sound")
	rawset(_G, "TimpaniWorld", nil)
end)

test("the_title_bars_have_no_shield", function()
	local e, ui = start(PAGE)
	open_root(ui)
	local kit = menu_of(e).kit
	local win = menu_of(e).win
	E.eq(title_of(e, ui), "Test Mod", "the title at the margin")
	for _, t in ipairs(ui.triangles) do
		local x, y = (t.a.x - kit.ox) / kit.scale, (t.a.z - kit.oy) / kit.scale
		local in_corner = x > win.x + 10 and x < win.x + 32 and y > win.top - 30 and y < win.top - 4
		E.eq(in_corner and t.color[2] == 240 and t.color[3] == 180, false, "no gold glyph at the left of the title bar")
	end
end)

test("with_no_page_to_list_the_window_says_so_and_closes", function()
	local e, ui = start("return true")
	open_root(ui)
	E.eq(ui.shown("No mod has added a page."), true, "in the words of Obsidian API's locales")
	E.eq(title_of(e, ui), "Obsidian")
	ui.tap("down")
	ui.tap("enter")
	ui.tap("esc")
	E.eq(menu().is_open(), true, "the first Esc only leaves the empty page")
	ui.tap("esc")
	E.eq(menu().is_open(), false)
end)

-- Three pages, each with a row that tells them apart, and one icon that doesn't exist.
local PAGES = [[
	local api = obsidian.import("obsidian.api")
	api.menu.add("One", function(page) page:label("In one") page:toggle("Flag", { get = function() return false end }) end, { icon = "player" })
	api.menu.add(function() return "Two" end, function(page) page:action("In two", function() end) end)
	api.menu.add("Three", function(page) page:action("In three", function() end) end, { icon = "no such icon" })
]]

test("the_keys_walk_the_list_of_pages_and_the_page_follows", function()
	local e, ui = start(PAGES)
	local M = menu_of(e)
	open_root(ui)
	E.eq(M.nav, true, "the keys start on the list")
	E.eq(ui.shown("In one"), true, "with the first page beside it")
	ui.tap("down")
	ui.frame()
	E.eq(ui.shown("In two"), true, "down opens the next page")
	E.eq(ui.shown("In one"), false)
	E.eq(M.nav, true, "the keys stay on the list")
	ui.tap("up")
	ui.tap("up")
	ui.frame()
	E.eq(M.folder, "m", "up from the first page is its folder")
	ui.tap("up")
	ui.frame()
	E.eq(ui.shown("In three"), true, "and up from the top wraps to the last")
	ui.tap("right")
	E.eq(M.nav, false, "right goes into the page")
	ui.tap("left")
	E.eq(M.nav, true, "left comes back, as an action has nothing to adjust")
	ui.tap("enter")
	E.eq(M.nav, false, "Enter goes in too")
	ui.tap("esc")
	E.eq(M.nav, true, "Esc comes back")
	E.eq(menu().is_open(), true)
	ui.tap("esc")
	E.eq(menu().is_open(), false, "and from the list it closes")
	open_root(ui)
	E.eq(ui.shown("In three"), true, "the page left open comes back")
end)

test("left_adjusts_a_row_that_has_a_value_and_doesnt_leave_the_page", function()
	local e, ui = start(PAGE)
	local M = menu_of(e)
	open_player(e, ui)
	ui.tap("down")
	ui.tap("left")
	E.eq(M.nav, false)
	E.eq(values().speed, 3, "it stepped")
end)

test("the_mouse_picks_a_page_and_a_row_takes_the_keys_from_the_list", function()
	local e, ui = start(PAGES)
	local M = menu_of(e)
	open_root(ui)
	click_on(ui, "Two", true)
	E.eq(ui.shown("In two"), true, "a click on the list opens the page")
	E.eq(M.nav, false, "and the keys go with it")
	ui.tap("esc")
	E.eq(M.nav, true)
	local row = ui.find("In two", true)
	ui.move(row.pos.x + 2, row.pos.y + 4)
	E.eq(M.nav, false, "moving over a row does the same")
end)

test("pages_show_an_icon_a_plain_one_without_and_a_wrong_name_is_a_log_line", function()
	local e, ui = start(PAGES)
	open_root(ui)
	E.eq(#ui.triangles > 0, true, "drawn from shapes")
	E.eq(menu().is_open(), true, "a wrong name breaks nothing")
	E.has(e:log(), "no glyph named no such icon")
end)

-- Folders and the title.

local function two_mods()
	return start_mods({
		{ id = "alpha", name = "Alpha Mod", source = [[
			local api = obsidian.import("obsidian.api")
			api.menu.add("First page", function(page) page:action("In first", function() end) end)
			api.menu.add("Second page", function(page) page:action("In second", function() end) end)
		]] },
		{ id = "beta", name = "Beta Mod", source = [[
			local api = obsidian.import("obsidian.api")
			api.menu.add("Beta page", function(page) page:action("In beta", function() end) end)
		]] },
	})
end

test("each_mods_pages_sit_in_a_folder_named_after_the_mod_in_load_order", function()
	local e, ui = two_mods()
	open_root(ui)
	local M = menu_of(e)
	local order = {}
	for _, area in ipairs(M.hits.nav) do order[#order + 1] = area.folder or area.root.title end
	E.eq(table.concat(order, ","), "alpha,First page,Second page,beta,Beta page")
	E.eq(ui.shown("Alpha Mod"), true, "the folder has the name from obsidian.json")
	E.eq(ui.shown("Beta Mod"), true)
	E.eq(nav_named(e, "First page").x, nav_named(e, "alpha").x, "rows span the list")
end)

test("the_title_names_the_mod_whose_page_is_open_and_obsidian_on_a_folder", function()
	local e, ui = two_mods()
	open_root(ui)
	E.eq(title_of(e, ui), "Alpha Mod", "a page of the first mod")
	ui.tap("down")
	ui.tap("down")
	ui.tap("down")
	ui.frame()
	E.eq(ui.shown("In beta"), true)
	E.eq(title_of(e, ui), "Beta Mod", "a page of the second")
	ui.tap("up")
	ui.frame()
	E.eq(menu_of(e).folder, "beta")
	E.eq(title_of(e, ui), "Obsidian", "on a folder: the top level")
	E.eq(ui.shown("Beta page"), true, "with the folder's pages beside the list")
end)

test("a_page_in_a_folders_view_opens_with_a_click", function()
	local e, ui = two_mods()
	local M = menu_of(e)
	open_root(ui)
	ui.tap("up")
	ui.frame()
	E.eq(M.folder, "alpha")
	local area
	for _, row in ipairs(M.hits.rows) do
		if row.it.label == "Second page" then area = row end
	end
	ui.click(middle(e, area))
	ui.frame()
	E.eq(ui.shown("In second"), true, "the page opened")
	E.eq(M.root.title, "Second page", "and the list follows")
	E.eq(M.nav, false, "with the keys in the page")
end)

test("a_folder_opens_and_closes_from_the_keys_and_the_mouse", function()
	local e, ui = two_mods()
	local M = menu_of(e)
	open_root(ui)
	ui.tap("left")
	E.eq(M.folder, "alpha", "left on a page goes up to its folder")
	ui.tap("enter")
	ui.frame()
	E.eq(M.closed.alpha, true, "Enter closes it")
	E.eq(ui.shown("First page"), true, "its pages still show beside the list")
	E.eq(#M.hits.nav, 3, "but not in it")
	ui.tap("right")
	E.eq(M.closed.alpha, nil, "right opens it")
	ui.tap("right")
	E.eq(M.root.title, "First page", "and then goes to its first page")
	ui.tap("up")
	ui.tap("left")
	E.eq(M.closed.alpha, true, "left closes a folder")
	ui.frame()
	ui.click(middle(e, nav_named(e, "beta")))
	ui.frame()
	E.eq(M.closed.beta, true, "a click closes one")
	E.eq(M.folder, "alpha", "and leaves the keys where they were")
	ui.click(middle(e, nav_named(e, "beta")))
	ui.frame()
	E.eq(M.closed.beta, nil, "a second click opens it")
end)

test("closing_the_folder_of_the_open_page_moves_to_the_folder", function()
	local e, ui = two_mods()
	local M = menu_of(e)
	open_root(ui)
	ui.click(middle(e, nav_named(e, "alpha")))
	ui.frame()
	E.eq(M.root, nil)
	E.eq(M.folder, "alpha")
	E.eq(title_of(e, ui), "Obsidian")
end)

test("a_closed_folder_stays_closed_at_the_next_launch", function()
	local e, ui = two_mods()
	local M = menu_of(e)
	open_root(ui)
	ui.tap("left")
	ui.tap("enter")
	ui.frame()
	local core = e:core()
	e.loaded["obsidian/loader/storage"].loaded = {}
	core.current = core.mods_by_id["obsidian.api"]
	E.eq(rawget(_G, "obsidian").storage.get("menu/closed"), "alpha", "in Obsidian API's storage")
	core.current = nil
	-- What a new launch does: the module reads the folders at install.
	M.closed = {}
	core.current = core.mods_by_id["obsidian.api"]
	for owner in tostring(rawget(_G, "obsidian").storage.get("menu/closed")):gmatch("%S+") do M.closed[owner] = true end
	core.current = nil
	menu().close()
	open_root(ui)
	E.eq(#M.hits.nav, 3, "alpha's pages stay folded")
end)

test("a_mods_functions_run_as_that_mod", function()
	local e, ui = start_mods({ { id = "m", name = "Test Mod", texts = { en = { hello = "Hello from m" } }, source = [[
		local api = obsidian.import("obsidian.api")
		api.menu.add(function() return obsidian.text("hello") end, function(page)
			page:label(obsidian.text("hello") .. " too")
			page:action("Who", function() return "caller " .. tostring(obsidian.caller()) end)
		end)
	]] } })
	open_player(e, ui)
	ui.frame()
	E.eq(ui.shown("Hello from m too"), true, "the build reads the mod's own texts")
	E.eq(ui.shown("Hello from m"), true, "and so does the title")
	ui.tap("enter")
	ui.frame()
	E.eq(ui.shown("caller m"), true, "an action runs as the mod")
end)

test("the_words_follow_the_games_language", function()
	rawset(_G, "LocalizationManager", { current_language = "bp" })
	local e, ui = start("return true")
	open_root(ui)
	E.eq(ui.shown("Nenhum mod adicionou uma p"), true, "Portuguese, from Obsidian API's locales")
	E.eq(ui.shown("Filtro"), true)
end)

-- The key.

test("the_key_of_obsidian_api_opens_the_window_once_a_page_is_listed", function()
	local e, ui = start(PAGE)
	ui.tap("f6")
	E.eq(menu().is_open(), true, "F6 opens it")
	ui.frame()
	E.eq(ui.shown("God mode"), true)
	ui.tap("f6")
	E.eq(menu().is_open(), false, "F6 closes it")
end)

test("with_no_page_listed_the_key_does_nothing", function()
	local e, ui = start("return true")
	ui.tap("f6")
	E.eq(menu().is_open(), false)
end)

test("the_key_works_where_the_lobby_isnt_cleared_for_a_client_mods_page", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e, ui = start_mods({
		{ id = "c", name = "Client Mod", source = [[
			local api = obsidian.import("obsidian.api")
			api.menu.add("Client page", function(page) page:label("In client") end)
		]] },
		{ id = "s", name = "Session Mod", scope = "session", source = [[
			local api = obsidian.import("obsidian.api")
			api.menu.add("Session page", function(page) page:label("In session") end)
		]] },
	}, { lobby = lobby })
	lobby.members = { "me", "vanilla" }
	e:frames(31)
	E.eq(e:core().session.cleared(), false)
	ui.tap("f6")
	E.eq(menu().is_open(), true, "the key isn't gated, though Obsidian API runs as a session mod")
	ui.frame()
	E.eq(ui.shown("In client"), true)
	E.eq(ui.shown("Session page"), false, "the session mod's page waits for a cleared lobby")
end)

test("two_toggles_in_one_frame_count_once", function()
	local e, ui = start(PAGE)
	menu().toggle()
	menu().toggle()
	E.eq(menu().is_open(), true, "an older Trainer's key beside Obsidian API's doesn't cancel it out")
	ui.frame()
	menu().toggle()
	E.eq(menu().is_open(), false)
end)

-- The window itself.

test("the_window_follows_the_screen_and_the_mouse_lands_on_what_it_sees", function()
	local e, ui = start(PAGE)
	ui.width, ui.height = 3840, 2160
	ui.frame()
	open_root(ui)
	local M = menu_of(e)
	E.eq(M.kit.scale, 3, "the design space fills the screen")
	E.eq(ui.find("God mode", true).size, 54)
	click_on(ui, "God mode")
	E.eq(values().god, true, "a click finds the row at four times the pixels")
end)

test("the_window_opens_on_the_design_screen_and_fits_a_small_one", function()
	local e, ui = start(PAGE)
	ui.width, ui.height = 1024, 576
	ui.frame()
	open_root(ui)
	local M = menu_of(e)
	E.eq(M.win.x + M.win.w <= 1280, true, "inside the design width")
	E.eq(M.win.top <= 720, true, "inside the design height")
	E.eq(M.win.top - M.win.h >= 0, true, "the bottom too")
end)

test("with_the_games_fonts_loaded_the_window_uses_them_and_one_gui", function()
	local e, ui = start(PAGE)
	rawget(_G, "Boot").package_manager = { is_package_online = function() return true end }
	open_root(ui)
	local title, row = ui.find("Test Mod", true), ui.find("God mode", true)
	E.eq(title.font, "fonts/insignia_headliner", "Insignia for names")
	E.eq(row.font, "fonts/handel_small", "Handel Gothic for reading")
	E.eq(ui.triangles[1].gui, row.gui, "shapes and text share a gui, or layers wouldn't hold")
	E.eq(ui.rects[1].gui, row.gui)
end)

test("a_click_that_closes_the_menu_keeps_the_game_off_the_button_until_it_comes_up", function()
	local e, ui = start(PAGE)
	open_root(ui)
	ui.press(middle(e, menu_of(e).hits.close))
	ui.frame()
	E.eq(menu().is_open(), true, "the left button is still down")
	ui.release()
	E.eq(menu().is_open(), false, "closed once it's up")

	menu().open()
	ui.frame()
	ui.press(700, 500, "right")
	ui.frame()
	E.eq(menu().is_open(), true, "the right button is still down")
	ui.release()
	E.eq(menu().is_open(), false, "closed once it's up")

	menu().open()
	ui.frame()
	ui.press(middle(e, menu_of(e).hits.close))
	menu().toggle()
	E.eq(menu().is_open(), false, "the keyboard still closes at once")
	ui.release()
	menu().open()
	ui.frame()
	E.eq(menu().is_open(), true, "and nothing is left waiting")
end)

test("the_filter_button_starts_the_filter", function()
	local e, ui = start(LONG)
	open_root(ui)
	click_on(ui, "Long")
	click_on(ui, "Filter", true)
	ui.typed = { "Row 7" }
	ui.frame()
	ui.typed = { "enter" }
	ui.frame()
	ui.frame()
	E.eq(ui.shown("Filter: Row 7"), true, "the title bar shows it")
	E.eq(ui.shown("Row 70."), true, "the filter applies")
	E.eq(ui.shown("Row 8."), false)
end)

test("the_title_bar_moves_the_window_and_it_stays_on_screen", function()
	local e, ui = start(PAGE)
	open_root(ui)
	local M = menu_of(e)
	local left, top = M.win.x, M.win.top
	local x, y = middle(e, M.hits.title)
	ui.press(x, y)
	ui.move(x + 150, y - 150)
	near(M.win.x, left + 100, "it follows the cursor, in the kit's units")
	near(M.win.top, top - 100, "up and down too")
	ui.move(-4000, -4000)
	near(M.win.x, 0, "stopped at the left edge")
	near(M.win.top - M.win.h, 0, "stopped at the bottom edge")
	ui.move(9000, 9000)
	near(M.win.x + M.win.w, 1280, "stopped at the right edge")
	near(M.win.top, 720, "stopped at the top edge")
	ui.release()
	local stopped = M.win.x
	ui.move(100, 100)
	E.eq(M.win.x, stopped, "let go, it stays")
end)

test("the_grip_resizes_the_window_and_more_rows_show", function()
	local e, ui = start(LONG)
	open_root(ui)
	click_on(ui, "Long")
	local M = menu_of(e)
	local width, height, top = M.win.w, M.win.h, M.win.top
	local before = rows_shown(ui)
	local x, y = middle(e, M.hits.grip)
	ui.press(x, y)
	ui.move(x + 150, y - 135)
	near(M.win.w, width + 100, "wider")
	near(M.win.h, height + 90, "taller, with the top staying put")
	E.eq(M.win.top, top, "the title bar didn't move")
	E.eq(rows_shown(ui), before + 3, "the extra height holds more rows")
	ui.move(-9000, 9000)
	E.eq(M.win.w, 500, "not narrower than the rows need")
	E.eq(rows_shown(ui) >= 4, true, "not so short that no rows fit")
	ui.release()
end)

test("the_window_is_remembered", function()
	local e, ui = start(PAGE)
	open_root(ui)
	local M = menu_of(e)
	local x, y = middle(e, M.hits.title)
	ui.press(x, y)
	ui.move(x + 150, y - 150)
	ui.release()
	local left, top = M.win.x, M.win.top
	menu().close()
	open_root(ui)
	E.eq(M.win.x, left, "while the game runs")
	menu().close()
	e.loaded["obsidian/loader/storage"].loaded = {}
	M.win = nil
	open_root(ui)
	near(M.win.x, left, "and from Obsidian API's storage at the next launch")
	near(M.win.top, top, "up and down too")
end)

test("the_wheel_and_the_scroll_bar_scroll_and_the_keys_bring_the_selection_back", function()
	local e, ui = start(LONG)
	open_root(ui)
	click_on(ui, "Long")
	E.eq(ui.shown("Row 1."), true)
	ui.scroll(-1)
	E.eq(ui.shown("Row 1."), false, "the wheel scrolled away from the selection")
	E.eq(ui.shown("Row 4."), true, "by three rows")
	ui.scroll(1)
	E.eq(ui.shown("Row 1."), true, "and back")
	ui.scroll(1)
	E.eq(ui.shown("Row 1."), true, "not past the top")
	local track = menu_of(e).hits.track
	ui.press(middle(e, { x = track.x, y = track.y, w = track.w, h = 2 }))
	ui.release()
	E.eq(ui.shown("Row 90."), true, "the bar jumped to the end")
	ui.tap("down")
	ui.frame()
	E.eq(ui.shown("Row 2."), true, "moving the selection brings it into view")
end)

test("hovering_selects_a_row_and_a_resting_cursor_leaves_the_keys_alone", function()
	local e, ui = start(LONG)
	open_root(ui)
	click_on(ui, "Long")
	local row = ui.find("Row 5.")
	ui.move(row.pos.x + 2, row.pos.y + 4)
	ui.frame()
	E.eq(ui.shown("Help for 5."), true, "the row under the cursor is selected")
	ui.tap("down")
	ui.tap("down")
	ui.frame()
	E.eq(ui.shown("Help for 7."), true, "the keys move on from there, the cursor resting")
end)

test("without_a_mouse_the_keys_still_work", function()
	local e, ui = start(PAGE)
	rawset(_G, "Mouse", nil)
	open_player(e, ui)
	ui.tap("enter")
	E.eq(values().god, true)
end)

test("a_broken_mouse_is_logged_once_and_the_keys_still_work", function()
	local e, ui = start(PAGE)
	rawget(_G, "Mouse").axis = function() error("no such axis") end
	open_player(e, ui)
	ui.frame()
	ui.tap("enter")
	E.eq(values().god, true)
	E.eq(menu().is_open(), true, "the window stays open")
	E.eq(E.count(e:log("obsidianapi"), "menu: no mouse"), 1)
end)

test("the_menu_shows_the_system_cursor_and_gives_the_game_its_own_back", function()
	local e, ui = start(PAGE)
	ui.cursor.shown, ui.cursor.clipped = false, true
	open_root(ui)
	E.eq(ui.cursor.system, true, "the arrow of the operating system")
	E.eq(ui.cursor.crashed, false, "the engine has to get a string")
	E.eq(ui.cursor.shown, true)
	E.eq(ui.cursor.clipped, false, "free to leave the aim area")
	menu().close()
	E.eq(ui.cursor.system, false, "not left behind")
	E.eq(ui.cursor.name, "hud/cursors/game")
	E.eq(ui.cursor.shown, false, "hidden again")
	E.eq(ui.cursor.clipped, true, "clipped again")
end)

test("where_the_engine_refuses_the_system_cursor_the_pause_menus_is_left", function()
	local e, ui = start(PAGE)
	ui.cursor.refuse_system = true
	open_root(ui)
	E.eq(ui.cursor.system, false)
	E.eq(ui.cursor.name, "hud/cursors/menu", "still a visible cursor")
	E.eq(ui.cursor.shown, true)
	E.eq(menu().is_open(), true)
	E.has(e:log("obsidianapi"), "no system cursor")
end)

test("a_game_that_hides_the_cursor_behind_the_menu_loses_that_fight", function()
	local e, ui = start(PAGE)
	open_root(ui)
	ui.cursor.shown, ui.cursor.system = false, false
	ui.frame()
	E.eq(ui.cursor.shown, true, "shown again")
	E.eq(ui.cursor.system, true)
	ui.cursor.name, ui.cursor.system = "hud/cursors/game", false
	ui.frame()
	E.eq(ui.cursor.name, "hud/cursors/menu", "swapped back")
	E.eq(ui.cursor.system, true)
end)

test("a_new_state_keeps_the_cursor_it_set_for_itself", function()
	local e, ui = start(PAGE)
	open_root(ui)
	ui.state = "menu"
	ui.cursor.name, ui.cursor.shown = "hud/cursors/menu", false
	ui.frame()
	E.eq(menu().is_open(), false, "the state change closed the window")
	E.eq(ui.cursor.shown, false, "the new state's choice stands")
	E.eq(ui.cursor.system, false)
	E.eq(ui.cursor.clipped, false, "and so does its clipping")
end)

local RESETS = [[
	local api = obsidian.import("obsidian.api")
	VALUES = { god = true, speed = 5, edited = true }
	api.menu.add("Player", function(page)
		page:toggle("God mode", { default = false, get = function() return VALUES.god end, set = function(v) VALUES.god = v end })
		page:number("Speed", { default = 5, min = 1, max = 10, step = 1, get = function() return VALUES.speed end, set = function(v) VALUES.speed = v end })
		page:number("Edited", { changed = function() return VALUES.edited end, reset = function() VALUES.edited = false end,
			get = function() return 1 end, set = function() end })
		page:action("Heal", function() VALUES.healed = true end)
	end)
]]

local function click_area(e, ui, area)
	ui.click(middle(e, area))
	ui.frame()
end

test("a_rows_reset_button_puts_it_back_and_stops_there", function()
	local e, ui = start(RESETS)
	open_root(ui)
	local button = row_named(e, "God mode").reset
	click_area(e, ui, button)
	E.eq(values().god, false, "back at its default")
	click_area(e, ui, button)
	E.eq(values().god, false, "the button isn't a second toggle")
end)

test("the_button_of_a_row_at_its_default_does_nothing", function()
	local e, ui = start(RESETS)
	open_root(ui)
	click_area(e, ui, row_named(e, "Speed").reset)
	E.eq(menu_of(e).field, nil, "a click there doesn't start typing, as a click on the row would")
	E.eq(values().speed, 5)
end)

test("a_row_with_a_way_back_of_its_own_uses_it", function()
	local e, ui = start(RESETS)
	open_root(ui)
	click_area(e, ui, row_named(e, "Edited").reset)
	E.eq(values().edited, false)
end)

test("r_resets_the_selected_row", function()
	local e, ui = start(RESETS)
	open_player(e, ui)
	ui.frame()
	E.eq(menu_of(e).nav, false, "the keys are on the page")
	ui.tap("r")
	E.eq(values().god, false)
	E.eq(values().speed, 5, "and only that row")
end)

test("the_column_sits_right_of_the_controls_on_every_row", function()
	local e, ui = start(RESETS)
	open_root(ui)
	local row = row_named(e, "Speed")
	E.eq(row.plus.x + row.plus.w <= row.reset.x, true, "the stepper stops short of the button")
	E.eq(row_named(e, "Heal").reset, nil, "a row with nothing to reset has no button")
end)

test("a_page_with_nothing_to_reset_has_no_column", function()
	local e, ui = start(PAGE)
	open_root(ui)
	local row = row_named(e, "Speed")
	E.eq(row.reset, nil)
	local win = menu_of(e).win
	E.eq(row.plus.x + row.plus.w > win.x + win.w - 40, true, "the controls keep the edge")
end)

test("the_grip_fills_the_corner_it_drags", function()
	local e, ui = start(PAGE)
	open_root(ui)
	ui.frame()
	local kit, dots = menu_of(e).kit, {}
	for _, r in ipairs(ui.rects) do
		if r.size.x == 2 * kit.scale and r.size.y == 2 * kit.scale then dots[#dots + 1] = r.pos end
	end
	E.eq(#dots, 6, "three rows of dots")
	local top, right = dots[1], dots[1]
	for _, d in ipairs(dots) do
		if d.y > top.y then top = d end
		if d.x > right.x then right = d end
	end
	E.eq(top.x, right.x, "the single dot on top sits over the right edge")
end)

test("the_window_draws_under_the_layer_mods_get_and_above_their_default", function()
	local e, ui = start(PAGE)
	open_root(ui)
	local low, high = math.huge, -math.huge
	for _, r in ipairs(ui.rects) do
		low, high = math.min(low, r.pos.z), math.max(high, r.pos.z)
	end
	E.eq(low > 700, true, "over a mod's canvas at the default layer")
	E.eq(high <= 875, true, "inside what obsidian.ui lets a mod draw at")
end)

return T
