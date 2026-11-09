-- obsidian.hook and obsidian.on_require, the wrapped require, and the per-frame driver.
-- Hooks of session mods call the original whenever the session isn't cleared (spec 6.4).
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { pending = {}, chains = {}, chain_of = {}, requires = {}, fired = {}, seen = {}, values = {}, frame_fns = {}, frame_installed = false }

function M.setup(core, session)
	M.core, M.session = core, session
end

local function pack(...)
	return { n = B.select("#", ...), ... }
end

-- Puts the hook around target[method] and returns the new function.
local function wrap(h, target, original)
	local core, session, state, fn, site = M.core, M.session, h.state, h.fn, "hook " .. B.tostring(h.method)
	-- A missing or unknown scope counts as session. The loader's own hooks are never gated.
	local gated = not h.loader and not h.always and not (state and state.mod.scope == "client")
	-- A hook that fails falls back to the game's function, unless it already ran it: that would
	-- spawn, spend or save twice. One closure per hook, as a new one per call stops the JIT here.
	local ran, results = false, nil
	local function once(...)
		ran = true
		results = pack(original(...))
		return B.unpack(results, 1, results.n)
	end
	target[h.method] = function(...)
		if (state and state.off) or (gated and not session.allows(state)) then return original(...) end
		local outer_ran, outer_results = ran, results
		ran, results = false, nil
		local r = pack(core.call(state, site, fn, once, ...))
		local did, got = ran, results
		ran, results = outer_ran, outer_results
		if r[1] then return B.unpack(r, 2, r.n) end
		-- With no results the game's function threw, and running it again would only do that twice.
		if did then
			if got then return B.unpack(got, 1, got.n) end
			return
		end
		return original(...)
	end
	return target[h.method]
end

local function describe(h)
	local where = B.type(h.target) == "string" and h.target or "table"
	return where .. "." .. B.tostring(h.method) .. (h.state and (" by " .. h.state.id) or "")
end

local function install(h)
	local target = h.target
	if B.type(target) == "string" then target = B.rawget(B.G, target) end
	if B.type(target) ~= "table" then return false end
	local original = target[h.method]
	if B.type(original) ~= "function" then return false end
	local top = wrap(h, target, original)
	-- Every hook on one method, in order, and the outermost wrapper, so they can all go back in
	-- if a mod declares the method again (see recheck).
	local by_method = M.chain_of[target]
	if not by_method then
		by_method = {}
		M.chain_of[target] = by_method
	end
	local chain = by_method[h.method]
	if not chain then
		chain = { target = target, method = h.method, hooks = {} }
		by_method[h.method] = chain
		B.insert(M.chains, chain)
	end
	B.insert(chain.hooks, h)
	chain.top = top
	log.line("hook installed: " .. describe(h))
	return true
end

-- True when f calls the wrapper: holds it as an upvalue, inside a function it holds, or in a table it holds.
local function reaches(f, wrapper, depth, seen)
	if seen[f] then return false end
	seen[f] = true
	for i = 1, 255 do
		local name, v = B.getupvalue(f, i)
		if name == nil then return false end
		if v == wrapper then return true end
		local kind = B.type(v)
		if kind == "function" and depth > 0 and reaches(v, wrapper, depth - 1, seen) then return true end
		if kind == "table" and not seen[v] then
			seen[v] = true
			for _, x in B.pairs(v) do
				if x == wrapper then return true end
				if depth > 0 and B.type(x) == "function" and reaches(x, wrapper, depth - 1, seen) then return true end
			end
		end
	end
	return false
end

-- A mod that loads a separate copy of a game class declares the methods again on the same table, which
-- throws away every hook on them (the Third Person Camera mod does this from the wrapper it puts around require).
-- After each require, a method that changed and no longer calls the hooks gets them all back, in order.
-- A mod that wrapped the hooked method keeps the hooks inside, so that one is left alone.
local function recheck()
	if not B.getupvalue then return end
	for _, chain in B.ipairs(M.chains) do
		local now = chain.target[chain.method]
		if now ~= chain.top and B.type(now) == "function" then
			if not reaches(now, chain.top, 3, {}) then
				for _, h in B.ipairs(chain.hooks) do
					now = wrap(h, chain.target, now)
					log.line("hook installed again: " .. describe(h))
				end
			end
			chain.top = now
		end
	end
end

function M.hook(target, method, fn)
	local h = { state = M.core.current, target = target, method = method, fn = fn }
	if not install(h) then B.insert(M.pending, h) end
end

-- A hook the session gate leaves alone, for a mod whose replaced files can't be switched off either.
function M.wrap(target, method, fn)
	local h = { state = M.core.current, always = true, target = target, method = method, fn = fn }
	if not install(h) then B.insert(M.pending, h) end
end

-- A hook owned by the loader itself (guardrails, game texts).
function M.loader_hook(target, method, fn)
	local h = { state = nil, loader = true, target = target, method = method, fn = fn }
	if not install(h) then B.insert(M.pending, h) end
end

function M.on_require(name, fn)
	local list = M.requires[name]
	if not list then
		list = {}
		M.requires[name] = list
	end
	local cb = { state = M.core.current, fn = fn }
	B.insert(list, cb)
	-- Too late for the load: called now, with what the load gave.
	local value, loaded = M.values[name], M.seen[name]
	if not loaded then
		local package = B.rawget(B.G, "package")
		if B.type(package) == "table" and B.type(package.loaded) == "table" and package.loaded[name] ~= nil then
			value, loaded = package.loaded[name], true
		end
	end
	if loaded then M.core.call(cb.state, "on_require " .. B.tostring(name), cb.fn, value) end
end

local function resolve_pending()
	if #M.pending == 0 then return end
	local still = {}
	for _, h in B.ipairs(M.pending) do
		if not install(h) then B.insert(still, h) end
	end
	M.pending = still
end

-- The engine calls the global update every frame. The boot script defines update after the loader
-- runs, so the wrapper goes in at the first require that happens once update exists.
local function install_frame()
	if M.frame_installed then return end
	local update = B.rawget(B.G, "update")
	if B.type(update) ~= "function" then return end
	M.frame_installed = true
	log.line("frame hook installed")
	local fns = M.frame_fns
	B.rawset(B.G, "update", function(...)
		local r = pack(update(...))
		for i = 1, #fns do B.pcall(fns[i]) end
		return B.unpack(r, 1, r.n)
	end)
	-- The boot script defines shutdown before update. A log that says it was watching and never says
	-- the game closed is how the app can tell a crash from a normal exit.
	local shutdown = B.rawget(B.G, "shutdown")
	if B.type(shutdown) == "function" then
		B.rawset(B.G, "shutdown", function(...)
			log.line("game closed")
			return shutdown(...)
		end)
		log.line("watching for the game to close")
	end
end

function M.on_frame(fn)
	B.insert(M.frame_fns, fn)
end

function M.wrap_require()
	local original = B.require
	B.rawset(B.G, "require", function(name, ...)
		local r = pack(original(name, ...))
		M.seen[name], M.values[name] = true, r[1]
		local list = M.requires[name]
		if list and not M.fired[name] then
			M.fired[name] = true
			for _, cb in B.ipairs(list) do M.core.call(cb.state, "on_require " .. B.tostring(name), cb.fn, r[1]) end
		end
		recheck()
		resolve_pending()
		install_frame()
		return B.unpack(r, 1, r.n)
	end)
end

return M
