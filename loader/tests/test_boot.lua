local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

test("sets_vanilla_build_date", function()
	local e = E.new()
	e:boot()
	E.eq(rawget(_G, "BUILD_DATE_INFORMATION"), "2020-12-22 09:33:14")
end)

test("a_normal_exit_leaves_a_line_a_crash_does_not", function()
	local e = E.new()
	e:boot()
	e:start_frames()
	e:frames(1)
	E.has(e:log(), "watching for the game to close")
	E.eq(string.find(e:log(), "game closed", 1, true), nil, "still running, as after a crash")
	rawget(_G, "shutdown")()
	E.has(e:log(), "game closed")
	E.eq(rawget(_G, "SHUTDOWNS"), 1, "the game's own shutdown still runs")
end)

test("missing_core_still_sets_build_date", function()
	local e = E.new({ deploy = false })
	rawset(_G, "require", function() error("no such resource") end)
	e:boot()
	E.eq(rawget(_G, "BUILD_DATE_INFORMATION"), "2020-12-22 09:33:14")
end)

test("wrong_revision_starts_no_mod_and_logs_once", function()
	local e = E.new({
		revision = "parent: 40000:0123456789ab tip",
		deploy = E.deploy({ mods = { E.mod("a.b") } }),
		mods = { ["obsidian/mods/a.b/main"] = "STARTED = true" },
	})
	e:boot()
	E.eq(rawget(_G, "STARTED"), nil, "mod ran")
	E.eq(E.count(e:log(), "build unknown"), 1)
	E.eq(rawget(_G, "obsidian"), nil, "api installed")
end)

test("mod_error_disables_only_that_mod", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("bad"), E.mod("good") } }),
		mods = { ["obsidian/mods/bad/main"] = "error('broken on purpose')", ["obsidian/mods/good/main"] = "GOOD = true" },
	})
	e:boot()
	E.eq(rawget(_G, "GOOD"), true, "good mod ran")
	E.has(e:log(), "failed bad")
	E.has(e:log(), "broken on purpose")
	E.has(e:log(), "started good")
	local n = e:core().notices
	E.eq(#n, 1, "notices")
	E.eq(n[1].key, "mod_failed")
	E.eq(n[1].args[1], "bad")
end)

test("mod_needing_newer_api_is_refused", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("new", { loader_api = 2 }) } }),
		mods = { ["obsidian/mods/new/main"] = "STARTED = true" },
	})
	e:boot()
	E.eq(rawget(_G, "STARTED"), nil)
	E.has(e:log(), "refused new: needs loader API 2")
end)

test("after_startup_runs_after_every_entry", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("a"), E.mod("b") } }),
		mods = {
			["obsidian/mods/a/main"] = "ORDER = 'a'; obsidian.after_startup(function() rawset(_G, 'ORDER', ORDER .. ',after') end)",
			["obsidian/mods/b/main"] = "ORDER = ORDER .. ',b'",
		},
	})
	e:boot()
	E.eq(rawget(_G, "ORDER"), "a,b,after")
end)

test("mod_replacing_pcall_does_not_break_the_loader", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("vandal"), E.mod("bad"), E.mod("good") } }),
		mods = {
			["obsidian/mods/vandal/main"] = "pcall = function() error('x') end; xpcall = pcall; string.format = nil; tostring = nil",
			["obsidian/mods/bad/main"] = "error('second')",
			["obsidian/mods/good/main"] = "GOOD = true",
		},
	})
	e:boot()
	E.eq(rawget(_G, "GOOD"), true)
	E.has(e:log(), "failed bad")
	E.has(e:log(), "boot done")
end)

test("log_marks_mod_loading_before_start", function()
	local e = E.new({ deploy = E.deploy({ mods = { E.mod("a.b") } }) })
	e.opts.mods = { ["obsidian/mods/a.b/main"] = "SEEN = io.open(os.getenv('APPDATA') .. '/Arrowhead/Helldivers/ObsidianLauncher/logs/loader.log'):read('*a')" }
	e:boot()
	local seen = rawget(_G, "SEEN")
	E.has(seen, "loading a.b", "log during start")
	E.lacks(seen, "started a.b", "log during start")
	E.has(e:log(), "started a.b")
end)

test("changes_line_lists_new_globals", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("a.b") } }),
		mods = { ["obsidian/mods/a.b/main"] = "FOO = 1; string.shout = string.upper" },
	})
	e:boot()
	E.has(e:log(), "changes: a.b globals+=FOO stdlib+=string.shout")
end)

test("log_name_is_sanitized", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("../x") } }),
		mods = { ["obsidian/mods/../x/main"] = "obsidian.log('hello', 42)" },
	})
	e:boot()
	E.has(e:log("x"), "hello 42")
end)

test("capabilities_is_read_only", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("a") } }),
		mods = { ["obsidian/mods/a/main"] = "CAP = obsidian.capabilities.after_startup; obsidian.capabilities.after_startup = false" },
	})
	e:boot()
	E.eq(rawget(_G, "CAP"), true)
	E.has(e:log(), "read-only")
	E.eq(e:core().api.capabilities.after_startup, true)
end)

test("repeated_error_logged_three_times_then_mod_off", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("a") } }),
		mods = { ["obsidian/mods/a/main"] = "" },
	})
	e:boot()
	local core = e:core()
	local state = core.mods_by_id["a"]
	local runs = 0
	for _ = 1, 12 do
		core.call(state, "tick", function() runs = runs + 1; error("again") end)
	end
	E.eq(runs, 10, "runs before the mod is switched off")
	E.eq(E.count(e:log(), "error a tick"), 3)
	E.has(e:log(), "switched off a after 10 errors")
end)

test("module_loads_a_script_of_the_calling_mod", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("a.b") } }),
		mods = {
			["obsidian/mods/a.b/main"] = "PARTS = obsidian.module('lib/parts')",
			["obsidian/mods/a.b/lib/parts"] = "return { name = 'parts' }",
		},
	})
	e:boot()
	E.eq(rawget(_G, "PARTS").name, "parts")
end)

test("import_gives_what_the_entry_of_another_mod_returned", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("base"), E.mod("user", { requires = { "base" } }) } }),
		mods = {
			["obsidian/mods/base/main"] = "return { answer = 42 }",
			["obsidian/mods/user/main"] = "ANSWER = obsidian.import('base').answer; MISSING = obsidian.import('nobody') == nil",
		},
	})
	e:boot()
	E.eq(rawget(_G, "ANSWER"), 42)
	E.eq(rawget(_G, "MISSING"), true)
end)

test("a_mod_is_refused_when_a_required_mod_failed", function()
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("base"), E.mod("user", { requires = { "base" } }) } }),
		mods = {
			["obsidian/mods/base/main"] = "error('boom')",
			["obsidian/mods/user/main"] = "STARTED = true",
		},
	})
	e:boot()
	E.eq(rawget(_G, "STARTED"), nil, "user ran")
	E.has(e:log(), "refused user: needs base")
	E.eq(rawget(_G, "obsidian").import("base"), nil, "a failed mod exports nothing")
end)

local function one_mod(source, opts)
	local mods = { E.mod("a.b", opts) }
	local e = E.new({ deploy = E.deploy({ mods = mods }), mods = { ["obsidian/mods/a.b/main"] = source } })
	return e
end

test("language_follows_the_games_code_for_every_language", function()
	local e = one_mod("LANG = obsidian.language()")
	rawset(_G, "LocalizationManager", { current_language = "ru" })
	e:boot()
	E.eq(rawget(_G, "LANG"), "ru")
	local lm = rawget(_G, "LocalizationManager")
	local cases = { [""] = "en", en = "en", fr = "fr", las = "es", bp = "pt-BR", de = "de", pl = "pl", chs = "zh-CN", ch = "zh-TW", se = "sv", xx = "en" }
	for code, want in pairs(cases) do
		lm.current_language = code
		E.eq(rawget(_G, "obsidian").language(), want, code)
	end
end)

test("language_reads_boot_before_the_game_sets_up_localization", function()
	local e = one_mod("LANG = obsidian.language()")
	rawset(_G, "LocalizationManager", {})
	rawset(_G, "Boot", { language = "de" })
	e:boot()
	E.eq(rawget(_G, "LANG"), "de")
end)

test("a_mods_name_and_text_come_in_a_language_beyond_the_first_four", function()
	local e = one_mod("NAME = obsidian.info().name; TEXT = obsidian.text('hello')",
		{ name = { en = "Rank Cap", ru = "Potolok" }, texts = { en = { hello = "hi" }, ru = { hello = "privet" } } })
	rawset(_G, "LocalizationManager", { current_language = "ru" })
	e:boot()
	E.eq(rawget(_G, "NAME"), "Potolok")
	E.eq(rawget(_G, "TEXT"), "privet")
end)

test("game_text_shows_a_mods_words_for_a_key_in_the_games_language", function()
	local e = one_mod([[
		OK = obsidian.game_text("rank_title_60", { en = "Warden", ru = "Strazh" })
		BAD = obsidian.game_text("rank_title_61", 7)
		PLAIN = obsidian.game_text("mod_only", "Words")
	]])
	rawset(_G, "LocalizationManager", { current_language = "en", string = function(_, key) return "vanilla " .. key end })
	e:boot()
	E.eq(rawget(_G, "OK"), true)
	E.eq(rawget(_G, "BAD"), false)
	local lm = rawget(_G, "LocalizationManager")
	E.eq(lm:string("rank_title_60"), "Warden")
	E.eq(lm:string("mod_only"), "Words")
	E.eq(lm:string("title_rank_01"), "vanilla title_rank_01", "other keys stay the game's")
	lm.current_language = "ru"
	E.eq(lm:string("rank_title_60"), "Strazh")
	lm.current_language = "de"
	E.eq(lm:string("rank_title_60"), "Warden", "English when the language has none")
	E.eq(rawget(_G, "obsidian").game_text("rank_title_60", nil), true)
	E.eq(lm:string("rank_title_60"), "vanilla rank_title_60", "nil takes the words away")
end)

test("settings_get_names_a_mod_for_code_outside_it", function()
	local e = one_mod("OWN = obsidian.settings.get('camera')",
		{ options = { { id = "camera", type = "choice", choices = { "free", "shared" }, default = "free" } } })
	rawset(_G, "Boot", {})
	e:boot()
	E.eq(rawget(_G, "OWN"), "free")
	local get = rawget(_G, "obsidian").settings.get
	E.eq(get("camera"), nil, "no mod is running here")
	E.eq(get("camera", "a.b"), "free", "a game script the mod replaced names the mod")
	E.eq(get("camera", "nobody"), nil)
	E.eq(get("missing", "a.b"), nil)
end)

return T
