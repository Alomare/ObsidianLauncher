-- Parts of a mod (`parts` in obsidian.json): code tagged with a client part runs in any lobby, code of
-- a session part and untagged code only in a cleared one, and a part that is off runs nothing.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local OWN = string.rep("a", 64)
local TEXTS = {
	en = {
		part_off = "%s: %s is off in this lobby. The rest keeps working.",
		parts_off = "%s: %s are off in this lobby. The rest keeps working.",
	},
}

-- A mod like Integrated Third Person: a client part, a session part, and a session part switched off by the option.
local function cam(over)
	local m = E.mod("cam", {
		name = { en = "Cam" },
		scope = "hybrid",
		keybinds = { { id = "look", default = "f8" }, { id = "boost", default = "f9" } },
		parts = {
			{ id = "view", scope = "client", on = true, name = { en = "View" } },
			{ id = "range", scope = "session", on = true, name = { en = "Range" } },
			{ id = "far", scope = "session", on = false, name = { en = "Far" } },
		},
	})
	for k, v in pairs(over or {}) do m[k] = v end
	return m
end

local CODE = [[
local function mark(x) local l = rawget(_G, "MARKS"); l[#l + 1] = x end
local view, range, far = { part = "view" }, { part = "range" }, { part = "far" }
obsidian.hook("Target", "view", function(o, self) mark("hook view"); return o(self) end, view)
obsidian.hook("Target", "range", function(o, self) mark("hook range"); return o(self) end, range)
obsidian.hook("Target", "far", function(o, self) mark("hook far"); return o(self) end, far)
obsidian.hook("Target", "plain", function(o, self) mark("hook plain"); return o(self) end)
obsidian.on_frame(function() mark("frame view") end, view)
obsidian.on_frame(function() mark("frame range") end, range)
obsidian.on_frame(function() mark("frame far") end, far)
obsidian.on_frame(function() mark("frame always") end, { always = true })
obsidian.on_session(function(on) mark("session view " .. tostring(on)) end, view)
obsidian.on_session(function(on) mark("session range " .. tostring(on)) end, range)
obsidian.on_session(function(on) mark("session far " .. tostring(on)) end, far)
obsidian.keybind("look", function() mark("key look") end, view)
obsidian.keybind("boost", function() mark("key boost") end, range)
rawset(_G, "CAM", {
	scope = obsidian.scope(),
	cleared = function(part) return obsidian.session.cleared(part) end,
})
]]

local function engine(lobby, mods, code)
	rawset(_G, "MARKS", {})
	local calls = {}
	local target = {}
	for _, name in ipairs({ "view", "range", "far", "plain" }) do
		target[name] = function() calls[#calls + 1] = name end
	end
	rawset(_G, "Target", target)
	local e = E.new({
		deploy = E.deploy({ has_session = true, modset = OWN, mods = mods or { cam() }, texts = TEXTS }),
		lobby = lobby,
		mods = code or { ["obsidian/mods/cam/main"] = CODE },
	})
	e:boot()
	e:start_frames()
	return e
end

local function marks()
	local seen = {}
	for _, m in ipairs(rawget(_G, "MARKS")) do seen[m] = (seen[m] or 0) + 1 end
	rawset(_G, "MARKS", {})
	return seen
end

local function call_all()
	local t = rawget(_G, "Target")
	for _, name in ipairs({ "view", "range", "far", "plain" }) do t[name](t) end
end

local function vanilla_joins(lobby, e)
	lobby.members = { "me", "p2" }
	e:frames(31)
	marks()
end

test("a_client_part_runs_in_a_lobby_that_isnt_cleared_and_a_session_part_stops", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	call_all()
	local m = marks()
	E.eq(m["hook view"], 1)
	E.eq(m["hook range"], 1)
	E.eq(m["hook plain"], 1)
	E.eq(m["frame range"] ~= nil, true, "a cleared lobby runs every part that is on")

	vanilla_joins(lobby, e)
	call_all()
	e:frames(1)
	m = marks()
	E.eq(m["hook view"], 1, "the client part's hook")
	E.eq(m["hook range"], nil, "the session part's hook")
	E.eq(m["hook plain"], nil, "untagged code goes with the strictest part")
	E.eq(m["frame view"], 1)
	E.eq(m["frame range"], nil)
	E.eq(m["frame always"], 1, "always runs in any lobby")
end)

test("a_part_that_is_off_runs_nothing", function()
	local e = engine(nil)
	e:frames(31)
	call_all()
	e:frames(1)
	local m = marks()
	E.eq(m["hook far"], nil)
	E.eq(m["frame far"], nil)
	E.has(e:log(), "hook left out: Target.far by cam, part far is off")
	E.lacks(e:log(), "hook installed: Target.far")
	E.has(e:log(), "hook installed: Target.view by cam part view")
	local core = e:core()
	core.current = core.mods_by_id["cam"]
	E.eq(rawget(_G, "CAM").cleared("far"), false, "even in a cleared session")
	E.eq(rawget(_G, "CAM").cleared("range"), true)
	core.current = nil
end)

test("each_on_session_listener_hears_the_switch_of_its_own_part", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	marks()
	lobby.members = { "me", "p2" }
	e:frames(31)
	local m = marks()
	E.eq(m["session range false"], 1)
	E.eq(m["session view false"], nil, "a client part never switches off")
	E.eq(m["session far false"], nil, "a part that is off listens to nothing")
	lobby.members = { "me" }
	e:frames(31)
	E.eq(marks()["session range true"], 1)
end)

test("cleared_answers_for_a_part_of_the_calling_mod", function()
	local lobby = { my_id = "me", members = { "me", "p2" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	local cam = rawget(_G, "CAM")
	local core = e:core()
	core.current = core.mods_by_id["cam"]
	E.eq(cam.cleared(), false)
	E.eq(cam.cleared("view"), true)
	E.eq(cam.cleared("range"), false)
	E.eq(cam.cleared("nothing"), false, "a part the mod doesn't declare counts as untagged")
	core.current = nil
end)

test("a_hybrid_mod_says_hybrid_and_the_loader_has_parts", function()
	local e = engine(nil)
	E.eq(rawget(_G, "CAM").scope, "hybrid")
	local api = rawget(_G, "obsidian")
	E.eq(api.info("cam").scope, "hybrid")
	E.eq(api.capabilities.parts, true)
	E.eq(e:core().mods_by_id["cam"].off, false)
end)

test("keys_of_a_client_part_work_in_any_lobby", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	vanilla_joins(lobby, e)
	e.held.f8, e.held.f9 = true, true
	e:frames(1)
	local m = marks()
	E.eq(m["key look"], 1)
	E.eq(m["key boost"], nil)
end)

test("a_notice_names_the_session_parts_that_stop", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby)
	e:frames(31)
	E.lacks(e:log(), "notice: part_off")
	vanilla_joins(lobby, e)
	e:frames(1)
	E.has(e:log(), "notice: part_off: Cam: Range is off in this lobby. The rest keeps working.")
	E.has(e:log(), "session: cam session parts off, client parts on")
	-- Once per switch: the next checks in the same lobby say nothing more.
	e:frames(61)
	E.eq(E.count(e:log(), "notice: part_off"), 1)
end)

test("two_session_parts_go_in_one_notice", function()
	local two = cam({ parts = {
		{ id = "view", scope = "client", on = true, name = { en = "View" } },
		{ id = "range", scope = "session", on = true, name = { en = "Range" } },
		{ id = "far", scope = "session", on = true, name = { en = "Far" } },
	} })
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local e = engine(lobby, { two })
	e:frames(31)
	vanilla_joins(lobby, e)
	e:frames(1)
	E.has(e:log(), "notice: parts_off: Cam: Range and Far are off in this lobby. The rest keeps working.")
end)

test("a_hybrid_mods_guarded_calls_wait_for_a_cleared_session_even_from_a_client_part", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local lib = E.mod("lib", { scope = "session" })
	local e = engine(lobby, { lib, cam({ requires = { "lib" } }) }, {
		["obsidian/mods/lib/main"] = "return obsidian.session.guard({ poke = function() return 'poked' end }, {})",
		["obsidian/mods/cam/main"] = [[
			local lib = obsidian.import("lib")
			obsidian.on_frame(function()
				local r, why = lib.poke()
				local l = rawget(_G, "MARKS")
				l[#l + 1] = "poke " .. tostring(r) .. " " .. tostring(why)
			end, { part = "view" })
		]],
	})
	e:frames(31)
	E.eq(marks()["poke poked nil"] ~= nil, true)
	vanilla_joins(lobby, e)
	e:frames(1)
	E.eq(marks()["poke false not_cleared"], 1, "the frame runs, the guarded call is refused")
end)

test("a_session_mod_stays_whole_and_says_nothing_of_parts", function()
	local lobby = { my_id = "me", members = { "me" }, data = {} }
	local plain = E.mod("cam", { name = { en = "Cam" }, scope = "session", keybinds = { { id = "look", default = "f8" }, { id = "boost", default = "f9" } } })
	local e = engine(lobby, { plain })
	vanilla_joins(lobby, e)
	call_all()
	e:frames(1)
	local m = marks()
	E.eq(m["hook view"], nil, "with no parts declared, a part name changes nothing")
	E.eq(m["frame view"], nil)
	E.eq(m["frame always"], 1)
	E.eq(rawget(_G, "CAM").scope, "session")
	E.lacks(e:log(), "notice: part")
	E.has(e:log(), "part: cam names view, which is not one of the mod's parts")
end)

return T
