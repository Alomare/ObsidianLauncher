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

return T
