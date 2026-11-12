-- obsidian.profiler: each mod's own time a frame, with the game's time inside a hook left out.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

-- A mod that hooks a game function and spends about 2 ms of its own in it, while the game's function
-- takes about 6 ms; and a mod that spends about 1 ms a frame in on_frame.
local HEAVY = [[
local function busy(ms) local t = os.clock() while (os.clock() - t) * 1000 < ms do end end
BUSY = busy
obsidian.hook("Slow", "work", function(original, ...)
	busy(2)
	return original(...)
end)
]]
local LIGHT = [[
obsidian.on_frame(function() BUSY(1) end)
PROFILER = obsidian.profiler
]]

local function booted()
	rawset(_G, "Slow", { work = function() local t = os.clock() while (os.clock() - t) * 1000 < 6 do end return "done" end })
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("heavy", { scope = "client" }), E.mod("light", { scope = "client" }) } }),
		mods = { ["obsidian/mods/heavy/main"] = HEAVY, ["obsidian/mods/light/main"] = LIGHT },
	})
	e:boot()
	e:start_frames()
	return e
end

local function frames(e, n)
	for _ = 1, n do
		E.eq(rawget(_G, "Slow").work(), "done", "the game's answer")
		e:frames(1)
	end
end

test("off_until_started_and_nothing_to_read", function()
	local e = booted()
	frames(e, 3)
	local p = rawget(_G, "PROFILER")
	E.eq(p.running(), false)
	E.eq(p.read(), nil)
	E.eq(rawget(_G, "obsidian").capabilities.profiler, true)
end)

test("each_mod_gets_its_own_time_and_not_the_game_s", function()
	local e = booted()
	local p = rawget(_G, "PROFILER")
	p.start()
	E.eq(p.running(), true)
	frames(e, 80)
	local r = p.read()
	assert(r, "a result after half a second")
	local by = {}
	for _, row in ipairs(r.mods) do by[row.id] = row end
	assert(by.heavy and by.light, "both mods listed")
	assert(by.heavy.ms > 1.5 and by.heavy.ms < 4, "the hook's own 2 ms, not the game's 6: " .. by.heavy.ms)
	assert(by.light.ms > 0.7 and by.light.ms < 3, "the frame function's 1 ms: " .. by.light.ms)
	E.eq(r.mods[1].id, "heavy", "sorted by cost")
	assert(by.heavy.calls >= 1 and by.heavy.calls < 1.5, "one call a frame: " .. by.heavy.calls)
	assert(by.heavy.peak_ms >= by.heavy.ms, "the peak is at least the average")
	assert(r.frame_ms > by.heavy.ms + 6 - 1, "a frame holds the game's time too: " .. r.frame_ms)
	p.stop()
	E.eq(p.running(), false)
	E.eq(p.read(), nil, "stopping clears the result")
end)

test("a_hook_that_throws_still_closes_its_timing", function()
	rawset(_G, "Slow", { work = function() return "done" end })
	local e = E.new({
		deploy = E.deploy({ mods = { E.mod("bad", { scope = "client" }) } }),
		mods = { ["obsidian/mods/bad/main"] = [[
			PROFILER = obsidian.profiler
			obsidian.hook("Slow", "work", function(original, ...) original(...) error("after the original") end)
		]] },
	})
	e:boot()
	e:start_frames()
	local p = rawget(_G, "PROFILER")
	p.start()
	for _ = 1, 40 do
		rawget(_G, "Slow").work()
		e:frames(1)
	end
	local prof = e.loaded["obsidian/loader/profiler"]
	E.eq(prof.on, true)
	-- The stack is empty between frames: a throw after the original left nothing open.
	E.eq(prof.enter("probe"), 1, "nothing left open")
	prof.leave(1)
end)

return T
