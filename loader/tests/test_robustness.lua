-- Properties that hold over random input: what each input mode promises, a settings file of random
-- lines never breaking the boot, and the garbage a hooked call makes (the per frame cost mods pay).
-- Seeds are fixed, so a failure repeats.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local seed = 12345
local function rand(n)
	seed = (seed * 1103515245 + 12345) % 2147483648
	return seed % n
end

local function engine(settings, source, keybinds)
	local mod = E.mod("a", { keybinds = keybinds or { { id = "toggle", default = "f8" } } })
	local e = E.new({ deploy = E.deploy({ mods = { mod } }), settings = settings, mods = { ["obsidian/mods/a/main"] = source or "" } })
	e:boot()
	e:start_frames()
	return e
end

test("input_modes_keep_their_promises_over_random_input", function()
	local S = engine(nil).loaded["obsidian/loader/settings"]
	for _, mode in ipairs({ "press", "release", "tap", "double_tap", "long_press" }) do
		for run = 1, 200 do
			local s, now, down = {}, 0, false
			local fires, downs, ups, held_at, fired_this_hold = 0, 0, 0, nil, 0
			for _ = 1, 60 do
				now = now + (1 + rand(20)) / 100
				local next_down = rand(3) ~= 0 and not down or (down and rand(4) == 0 and false) or down
				if next_down and not down then downs = downs + 1; held_at = now; fired_this_hold = 0 end
				local was_down = down
				down = next_down
				local fired = S.step(s, down, mode, now)
				if fired then fires = fires + 1; fired_this_hold = fired_this_hold + 1 end
				if was_down and not down then
					ups = ups + 1
					if mode == "tap" and fired then assert(now - held_at <= S.TAP_MAX + 1e-9, "tap fired after a long hold") end
				end
				if mode == "long_press" then assert(fired_this_hold <= 1, "long_press fired twice in one hold") end
				if fired and mode == "press" then assert(down and not was_down, "press fired without a down edge") end
				if fired and mode == "release" then assert(was_down and not down, "release fired without an up edge") end
			end
			if mode == "press" then assert(fires == downs, ("press: %d fires for %d presses"):format(fires, downs)) end
			if mode == "release" then assert(fires == ups, ("release: %d fires for %d releases"):format(fires, ups)) end
			assert(fires <= downs, mode .. " fired more often than the key went down")
		end
	end
end)

test("a_random_settings_file_never_breaks_the_boot", function()
	local words = { "consent", "key", "mode", "pad", "option", "war", "sandbox", "a", "toggle", "f8", "mouse_middle", "left_shoulder+d_up", "1", "0", "tap", "\t", "\t", "\t", "x" }
	for _ = 1, 150 do
		local lines = {}
		for l = 1, rand(12) do
			local parts = {}
			for p = 1, rand(6) do parts[p] = words[1 + rand(#words)] end
			lines[l] = table.concat(parts, "\t")
		end
		local ok, err = pcall(function()
			local e = engine(table.concat(lines, "\n") .. "\n", [[obsidian.keybind("toggle", function() end)]])
			e.held.f8 = true
			e:frames(3)
			assert(not e:log():find("failed", 1, true), e:log())
		end)
		assert(ok, tostring(err) .. "\n---\n" .. table.concat(lines, "\n"))
	end
end)

test("a_hooked_call_makes_little_garbage", function()
	rawset(_G, "Target", { f = function(self, x) return x + 1 end })
	local raw = Target.f
	local e = engine(nil, [[obsidian.hook("Target", "f", function(original, self, x) return original(self, x) * 2 end)]])
	e:frames(1)
	local N = 200000
	local t = os.clock()
	for i = 1, N do raw(Target, i) end
	local plain = os.clock() - t
	collectgarbage("collect")
	local before = collectgarbage("count")
	collectgarbage("stop")
	t = os.clock()
	for i = 1, N do assert(Target.f(Target, i) == (i + 1) * 2) end
	local hooked = os.clock() - t
	local kb = collectgarbage("count") - before
	collectgarbage("restart")
	local per_call = kb * 1024 / N
	rawset(_G, "Target", nil)
	-- 224 bytes after the 2.0 review (580 before): the arguments and results of the call, nothing per hook.
	assert(per_call < 320, ("a hooked call makes %.0f bytes of garbage (%.0f ns each, plain %.0f ns)"):format(per_call, hooked / N * 1e9, plain / N * 1e9))
end)

return T
