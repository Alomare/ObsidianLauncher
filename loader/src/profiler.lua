-- obsidian.profiler: how much of each frame every mod's own code takes, and in which of its hooks,
-- frame functions and measured sections (`site`: "hook Class.method", "on_frame", a measure label).
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

local M = { on = false, users = {}, clock_kind = nil, SITES = 12 }

-- The frame being measured: per mod, seconds and calls. The window: sums over the frames since the
-- last result. The result: what read() answers, made about twice a second.
local frame_time, frame_calls = {}, {}
local window = { frames = 0, seconds = 0, time = {}, calls = {}, peak = {}, sites = {}, site_calls = {} }
local result = nil
local last_frame_at = nil

-- The stack of calls running now: ids (false for the game's own code), when each started and how much
-- of it its inner calls took.
local ids, starts, inner, sites = {}, {}, {}, {}
local depth = 0

local now = B.clock

local function pick_clock()
	if M.clock_kind then return end
	M.clock_kind = "os.clock"
	-- The game's own require, else what is already loaded, as Obsidian API's screenshots look for it.
	local ok, ffi = B.pcall(B.rawget(B.G, "require") or B.require, "ffi")
	if not ok or B.type(ffi) ~= "table" then
		local package = B.rawget(B.G, "package")
		ffi = B.type(package) == "table" and B.type(package.loaded) == "table" and package.loaded.ffi or nil
		-- LuaJIT keeps ffi in package.preload until something asks for it.
		if B.type(ffi) ~= "table" and B.type(package) == "table" and B.type(package.preload) == "table"
			and B.type(package.preload.ffi) == "function" then
			local got, lib = B.pcall(package.preload.ffi, "ffi")
			if got and B.type(lib) == "table" then
				ffi = lib
				if B.type(package.loaded) == "table" then package.loaded.ffi = lib end
			end
		end
		if B.type(ffi) ~= "table" then
			ok, ffi = B.pcall(B.require, "ffi")
			if not ok then ffi = nil end
		end
	end
	if B.type(ffi) ~= "table" or ffi.os ~= "Windows" then return end
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
function M.enter(id, site)
	depth = depth + 1
	ids[depth], starts[depth], inner[depth], sites[depth] = id, now(), 0, site or "other"
	return depth
end

-- Closes the calls down to `at` (from enter), charging each its own time.
function M.leave(at)
	local t = now()
	while depth >= at and depth > 0 do
		local spent = t - starts[depth]
		local id = ids[depth]
		if id then
			local own = spent - inner[depth]
			frame_time[id] = (frame_time[id] or 0) + own
			frame_calls[id] = (frame_calls[id] or 0) + 1
			local by, n = window.sites[id], window.site_calls[id]
			if not by then
				by, n = {}, {}
				window.sites[id], window.site_calls[id] = by, n
			end
			local site = sites[depth]
			by[site] = (by[site] or 0) + own
			n[site] = (n[site] or 0) + 1
		end
		depth = depth - 1
		if depth > 0 then inner[depth] = inner[depth] + spent end
	end
end

local function publish(frame_ms)
	local list = {}
	local n = window.frames
	for id, seconds in B.pairs(window.time) do
		local where = {}
		local calls = window.site_calls[id] or {}
		for site, s in B.pairs(window.sites[id] or {}) do
			B.insert(where, { site = site, ms = s * 1000 / n, calls = (calls[site] or 0) / n })
		end
		B.sort(where, function(a, b) return a.ms > b.ms end)
		while #where > M.SITES do B.remove(where) end
		B.insert(list, {
			id = id,
			ms = seconds * 1000 / n,
			peak_ms = (window.peak[id] or 0) * 1000,
			calls = (window.calls[id] or 0) / n,
			sites = where,
		})
	end
	B.sort(list, function(a, b) return a.ms > b.ms end)
	local total = 0
	for _, row in B.ipairs(list) do total = total + row.ms end
	result = { frame_ms = frame_ms, frames = n, total_ms = total, mods = list, clock = M.clock_kind }
	window.frames, window.seconds, window.time, window.calls, window.peak, window.sites, window.site_calls = 0, 0, {}, {}, {}, {}, {}
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
	window = { frames = 0, seconds = 0, time = {}, calls = {}, peak = {}, sites = {}, site_calls = {} }
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
		-- Seconds from a fixed point, as precise as the clock the profiler uses.
		clock = function()
			pick_clock()
			return now()
		end,
		-- Runs fn(...) and gives back what it gives. While measuring, the time goes under `label` among
		-- the calling mod's sites, so a big frame function can say which part of it costs.
		measure = function(label, fn, ...)
			if not M.on then return fn(...) end
			local at = M.enter(caller(), B.tostring(label))
			local r = { n = B.select("#", ...), B.pcall(fn, ...) }
			M.leave(at)
			if not r[1] then B.error(r[2], 0) end
			return B.unpack(r, 2, r.n)
		end,
		-- The last result, about twice a second, or nil before the first one: { frame_ms, frames,
		-- total_ms, clock, mods = { { id, ms, peak_ms, calls, sites = { { site, ms, calls } } } } },
		-- sorted by ms. ms is the average a frame, peak_ms the most in one frame, calls the average a
		-- frame; sites are the costliest places of the mod's time, at most 12. id "obsidian.loader" is
		-- the loader's own hooks.
		read = function()
			if not result then return nil end
			local copy = { frame_ms = result.frame_ms, frames = result.frames, total_ms = result.total_ms, clock = result.clock, mods = {} }
			for i, row in B.ipairs(result.mods) do
				local where = {}
				for j, s in B.ipairs(row.sites or {}) do where[j] = { site = s.site, ms = s.ms, calls = s.calls } end
				copy.mods[i] = { id = row.id, ms = row.ms, peak_ms = row.peak_ms, calls = row.calls, sites = where }
			end
			return copy
		end,
	}
end

return M
