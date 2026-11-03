local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local OPTIONS = {
	{ id = "d", type = "number", default = 5, min = 1, max = 12, step = 0.5 },
	{ id = "sky", type = "choice", default = "day", choices = { "day", "night" } },
	{ id = "cam", type = "toggle", default = false },
}

-- One client mod "a" that stores what it reads in the global GOT.
local function engine(settings, over, source)
	local mod = E.mod("a", { options = OPTIONS, keybinds = { { id = "toggle", default = "f8" } } })
	for k, v in pairs(over or {}) do mod[k] = v end
	local e = E.new({
		deploy = E.deploy({ mods = { mod } }),
		settings = settings,
		mods = { ["obsidian/mods/a/main"] = source or [[
			local s = obsidian.settings
			rawset(_G, "GOT", { d = s.get("d"), sky = s.get("sky"), cam = s.get("cam"), nope = s.get("nope") })
		]] },
	})
	e:boot()
	e:start_frames()
	return e
end

test("missing_settings_file_gives_defaults", function()
	local e = engine(nil)
	local got = rawget(_G, "GOT")
	E.eq(got.d, 5, "d")
	E.eq(got.sky, "day", "sky")
	E.eq(got.cam, false, "cam")
	E.eq(got.nope, nil, "unknown option")
	E.eq(e:core().consent(), false, "consent")
end)

test("settings_file_sets_values_and_consent", function()
	local e = engine("# comment\nconsent\t1\noption\ta\td\tnumber\t7.5\noption\ta\tsky\tchoice\tnight\noption\ta\tcam\ttoggle\t1\n")
	local got = rawget(_G, "GOT")
	E.eq(got.d, 7.5, "d")
	E.eq(got.sky, "night", "sky")
	E.eq(got.cam, true, "cam")
	E.eq(e:core().consent(), true, "consent")
	E.lacks(e:log(), "settings: skipped")
end)

test("bad_line_skipped_and_logged_once", function()
	local e = engine("garbage\noption\ta\nconsent\t1\nconsent\tmaybe\n\n")
	E.eq(e:core().consent(), true, "consent from the good line")
	E.eq(E.count(e:log(), "settings: skipped"), 1, "log lines")
	E.has(e:log(), "settings: skipped 3 bad lines (first: line 1)")
end)

test("out_of_range_number_gives_default", function()
	local e = engine("option\ta\td\tnumber\t99\noption\ta\tsky\tchoice\tdusk\noption\ta\tcam\tnumber\t1\n")
	local got = rawget(_G, "GOT")
	E.eq(got.d, 5, "number above max")
	E.eq(got.sky, "day", "unknown choice")
	E.eq(got.cam, false, "type changed")
	E.has(e:log(), "settings: a.d 99 is not allowed, default used")
end)

test("number_off_step_gives_default", function()
	engine("option\ta\td\tnumber\t2.25\n")
	E.eq(rawget(_G, "GOT").d, 5)
end)

test("text_falls_back_to_english", function()
	rawset(_G, "LocalizationManager", { current_language = "fr" })
	local e = engine(nil, { texts = { en = { hi = "Hello %s", bye = "Bye" }, fr = { hi = "Salut %s" } } }, [[
		rawset(_G, "GOT", { hi = obsidian.text("hi", "x"), bye = obsidian.text("bye"), shared = obsidian.text("mod_failed", "m"), none = obsidian.text("none") })
	]])
	local got = rawget(_G, "GOT")
	E.eq(got.hi, "Salut x", "mod text in French")
	E.eq(got.bye, "Bye", "mod text in English")
	E.eq(got.shared, "Obsidian: m failed to start", "shared catalog in English")
	E.eq(got.none, "none", "missing key")
	E.eq(e:core().text("mod_failed", "m"), "Obsidian: m failed to start")
end)

local KEY_MOD = [[
	rawset(_G, "PRESSES", 0)
	obsidian.keybind("toggle", function() rawset(_G, "PRESSES", rawget(_G, "PRESSES") + 1) end)
]]

test("keybind_fires_once_per_press", function()
	local e = engine("key\ta\ttoggle\tf9\n", nil, KEY_MOD)
	e.held.f8 = true
	e:frames(2)
	E.eq(rawget(_G, "PRESSES"), 0, "default key after a remap")
	e.held.f9 = true
	e:frames(3)
	E.eq(rawget(_G, "PRESSES"), 1, "held for three frames")
	e.held.f9 = nil
	e:frames(1)
	e.held.f9 = true
	e:frames(1)
	E.eq(rawget(_G, "PRESSES"), 2, "second press")
end)

test("keybind_default_key_and_unknown_key", function()
	local e = engine("key\ta\ttoggle\tnumpad_9\n", nil, KEY_MOD)
	e.held.f8 = true
	e:frames(2)
	E.eq(rawget(_G, "PRESSES"), 0, "unknown key binds nothing")
	E.eq(E.count(e:log(), "keybind a.toggle: unknown key numpad_9"), 1, "logged once")
	local e2 = engine(nil, nil, KEY_MOD)
	e2.held.f8 = true
	e2:frames(1)
	E.eq(rawget(_G, "PRESSES"), 1, "default key")
end)

test("session_mod_keybind_waits_for_a_cleared_session", function()
	local e = engine(nil, { scope = "session" }, KEY_MOD)
	e:core().deploy.has_session = true
	rawset(_G, "NetworkGlobal", { lobby = {
		my_id = "me",
		lobby = {},
		state = function() return "joined" end,
		members = function() return { "me", "p2" } end,
		member_data = function() return nil end,
		set_member_data = function() end,
	} })
	e:frames(31)
	e.held.f8 = true
	e:frames(1)
	E.eq(rawget(_G, "PRESSES"), 0, "not cleared")
end)

test("a_keybind_with_always_fires_in_a_lobby_that_isnt_cleared", function()
	local e = engine(nil, { scope = "session" }, [[
		rawset(_G, "PRESSES", 0)
		obsidian.keybind("toggle", function() rawset(_G, "PRESSES", rawget(_G, "PRESSES") + 1) end, { always = true })
	]])
	e:core().deploy.has_session = true
	rawset(_G, "NetworkGlobal", { lobby = {
		my_id = "me",
		lobby = {},
		state = function() return "joined" end,
		members = function() return { "me", "p2" } end,
		member_data = function() return nil end,
		set_member_data = function() end,
	} })
	e:frames(31)
	E.eq(e:core().session.cleared(), false, "the lobby isn't cleared")
	e.held.f8 = true
	e:frames(1)
	E.eq(rawget(_G, "PRESSES"), 1, "the session gate leaves it alone, as with obsidian.wrap")
end)

return T
