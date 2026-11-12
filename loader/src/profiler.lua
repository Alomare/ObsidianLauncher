-- obsidian.profiler: how much of each frame every mod's own code takes.
--
-- Every call into a mod goes through core.call (hooks, frame functions, keys, on_require, menu rows),
-- so the time is taken there, per mod. A hook calls the game's own function through the loader
-- (hooks.lua `once`), and that time is the game's: it is taken off the mod. A mod's code that calls
-- another mod's code is charged only for its own part. Off until a mod starts it, and then one test of
-- a boolean per call when off again.
--
-- The clock is QueryPerformanceCounter through LuaJIT's ffi where the game has it (Obsidian API's
-- screenshots use the same ffi), else os.clock, which on Windows counts milliseconds only.
local B = require("obsidian/loader/builtins")

local M = { on = false, users = {}, clock_kind = nil }

-- The frame being measured: per mod, seconds and calls. The window: sums over the frames since the
-- last result. The result: what read() answers, made about twice a second.
local frame_time, frame_calls = {}, {}
local window = { frames = 0, seconds = 0, time = {}, calls = {}, peak = {} }
local result = nil
local last_frame_at = nil

-- The stack of calls running now: ids (false for the game's own code), when each started and how much
-- of it its inner calls took.
local ids, starts, inner = {}, {}, {}
local depth = 0

local now = B.clock

local function pick_clock()
	if M.clock_kind then return end
	M.clock_kind = "os.clock"
	local ok, ffi = B.pcall(B.require, "ffi")
	if not ok or B.type(ffi) ~= "table" or ffi.os ~= "Windows" then return end
	-- Declared already by another chunk in the same state: cdef fails, and the calls still work.
	B.pcall(ffi.cdef, [[
		int QueryPerformanceCounter(int64_t *count);
		int QueryPerformanceFrequency(int64_t *frequency);
	]])
	local okf, freq = B.pcall(function()
		local f = ffi.new("int64_t[1]")
		if ffi.C.QueryPerformanceFrequency(f) == 0 then return nil end
		return B.tonumber(f[0])
	end)
	if not okf or not freq or freq <= 0 then return end
	local count = ffi.new("int64_t[1]")
	local qpc, tonumber = ffi.C.QueryPerformanceCounter, B.tonumber
	local base = nil
	local ok2 = B.pcall(function()
		qpc(count)
		base = count[0]
	end)
	if not ok2 then return end
	now = function()
		qpc(count)
		return tonumber(count[0] - base) / freq
	end
	M.clock_kind = "qpc"
end

-- A call into `id` (a mod id, or false for the game's own code) starts. Answers the depth to give
-- leave(), so a call that threw past its leave is closed with the one around it.
function M.enter(id)
	depth = depth + 1
	ids[depth], starts[depth], inner[depth] = id, now(), 0
	return depth
end

-- Closes the calls down to `at` (from enter), charging each its own time.
function M.leave(at)
	local t = now()
	while depth >= at and depth > 0 do
		local spent = t - starts[depth]
		local id = ids[depth]
		if id then
			frame_time[id] = (frame_time[id] or 0) + spent - inner[depth]
			frame_calls[id] = (frame_calls[id] or 0) + 1
		end
		depth = depth - 1
		if depth > 0 then inner[depth] = inner[depth] + spent end
	end
end

local function publish(frame_ms)
	local list = {}
	local n = window.frames
	for id, seconds in B.pairs(window.time) do
		B.insert(list, {
			id = id,
			ms = seconds * 1000 / n,
			peak_ms = (window.peak[id] or 0) * 1000,
			calls = (window.calls[id] or 0) / n,
		})
	end
	B.sort(list, function(a, b) return a.ms > b.ms end)
	local total = 0
	for _, row in B.ipairs(list) do total = total + row.ms end
	result = { frame_ms = frame_ms, frames = n, total_ms = total, mods = list, clock = M.clock_kind }
	window.frames, window.seconds, window.time, window.calls, window.peak = 0, 0, {}, {}, {}
end

-- At the start of each engine frame: the frame before goes into the window.
function M.frame()
	if not M.on then return end
	-- Nothing may stay open across frames; an unclosed call (an engine error past every pcall) ends here.
	if depth > 0 then M.leave(1) end
	local t = now()
	local dt = last_frame_at and (t - last_frame_at) or 0
	last_frame_at = t
	if dt <= 0 or dt > 1 then
		frame_time, frame_calls = {}, {}
		return
	end
	window.frames = window.frames + 1
	window.seconds = window.seconds + dt
	for id, s in B.pairs(frame_time) do
		window.time[id] = (window.time[id] or 0) + s
		if s > (window.peak[id] or 0) then window.peak[id] = s end
	end
	for id, c in B.pairs(frame_calls) do window.calls[id] = (window.calls[id] or 0) + c end
	frame_time, frame_calls = {}, {}
	if window.seconds >= 0.5 then publish(window.seconds * 1000 / window.frames) end
end

local function reset()
	frame_time, frame_calls = {}, {}
	window = { frames = 0, seconds = 0, time = {}, calls = {}, peak = {} }
	result, last_frame_at, depth = nil, nil, 0
end

local function any_user()
	return B.next(M.users) ~= nil
end

-- What mods get as obsidian.profiler. `core` gives the id of the calling mod.
function M.api(core)
	local function caller()
		local state = core.current
		return state and state.id or "?"
	end
	return {
		-- Starts measuring, for the calling mod, until it calls stop(). Measuring costs a little of
		-- every call into every mod, so it runs only while some mod asked for it.
		start = function()
			pick_clock()
			if not M.on then reset() end
			M.users[caller()] = true
			M.on = true
		end,
		stop = function()
			M.users[caller()] = nil
			if not any_user() then
				M.on = false
				reset()
			end
		end,
		running = function() return M.on end,
		-- The last result, about twice a second, or nil before the first one: { frame_ms, frames,
		-- total_ms, clock, mods = { { id, ms, peak_ms, calls }, ... } }, sorted by ms. ms is the
		-- average a frame, peak_ms the most in one frame, calls the average a frame. id
		-- "obsidian.loader" is the loader's own hooks.
		read = function()
			if not result then return nil end
			local copy = { frame_ms = result.frame_ms, frames = result.frames, total_ms = result.total_ms, clock = result.clock, mods = {} }
			for i, row in B.ipairs(result.mods) do
				copy.mods[i] = { id = row.id, ms = row.ms, peak_ms = row.peak_ms, calls = row.calls }
			end
			return copy
		end,
	}
end

return M
