-- The settings file the app writes beside the saves (consent, option values and keys), and
-- obsidian.settings.get, obsidian.keybind and obsidian.text. Values are checked again against the
-- options in the deploy table, so a hand-edited file can't hand a mod a bad value.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local war = require("obsidian/loader/war")

local M = { values = {}, keys = {}, binds = {} }

local function fields(line)
	local out = {}
	for f in B.gmatch(line .. "\t", "([^\t]*)\t") do B.insert(out, f) end
	return out
end

-- The settings text of one value, read as the option's type; nil when the value is not allowed.
local function check(opt, text)
	local t = opt.type
	if t == "toggle" then
		if text == "1" then return true elseif text == "0" then return false end
		return nil
	elseif t == "number" then
		local n = B.tonumber(text)
		if not n or n ~= n or n == 1 / 0 or n == -1 / 0 then return nil end
		if (opt.min and n < opt.min) or (opt.max and n > opt.max) then return nil end
		if opt.step and opt.step > 0 then
			local k = (n - (opt.min or 0)) / opt.step
			if B.abs(k - B.floor(k + 0.5)) > 1e-6 then return nil end
		end
		return n
	elseif t == "choice" then
		for _, c in B.ipairs(opt.choices or {}) do
			if c == text then return text end
		end
		return nil
	elseif t == "key" then
		return B.match(text, "^[a-z0-9_]+$") and text or nil
	elseif t == "color" then
		return B.match(text, "^#%x%x%x%x%x%x$") and B.lower(text) or nil
	end
	return nil
end

local function find(list, id)
	for _, x in B.ipairs(list or {}) do
		if x.id == id then return x end
	end
	return nil
end

-- Reads the file and fills core.consent_on, the guardrails, the Sandbox choice and the option values.
-- A missing file leaves the defaults: no consent, no guardrails, the Official save and every option at
-- the default.
function M.load(core, path)
	M.core = core
	local mods = {}
	for _, mod in B.ipairs(core.deploy and core.deploy.mods or {}) do mods[B.tostring(mod.id)] = mod end
	local f = path and B.open(path, "rb")
	if not f then
		log.line("settings: no file, defaults used")
		return
	end
	local text = f:read("*a") or ""
	f:close()
	local bad, first, n = 0, nil, 0
	for line in B.gmatch(text, "([^\n]*)\n?") do
		n = n + 1
		line = B.gsub(line, "\r$", "")
		local x = fields(line)
		local ok = true
		if line == "" or B.sub(line, 1, 1) == "#" then
			-- blank or comment
		elseif x[1] == "consent" and #x == 2 and (x[2] == "0" or x[2] == "1") then
			core.consent_on = x[2] == "1"
		elseif (x[1] == "block_achievements" or x[1] == "freeze_progress") and #x == 2 and (x[2] == "0" or x[2] == "1") then
			core[x[1]] = x[2] == "1"
		elseif x[1] == "hide_client_mods" and #x == 2 and (x[2] == "0" or x[2] == "1") then
			core.hide_client_mods = x[2] == "1"
		elseif x[1] == "sandbox" then
			-- A Sandbox line the loader can't use is still a Sandbox: the launch must never open the
			-- Official save while the player believes it is a Sandbox.
			if #x == 2 and B.match(x[2], "^[0-9a-f]+$") and #x[2] == 8 then
				core.sandbox = x[2]
			else
				core.sandbox_bad = true
				log.line("settings: the sandbox line is not valid, saving stays off for this launch")
			end
		elseif x[1] == "war" then
			-- Only with a Sandbox line before it, which the core always writes first.
			local w = war.parse(x)
			if w and core.sandbox then
				core.war = w
			else
				log.line("settings: the war line is not valid or has no Sandbox, the official war stays")
			end
		elseif x[1] == "option" and #x == 5 then
			local opt = mods[x[2]] and find(mods[x[2]].options, x[3])
			local v = nil
			if opt and opt.type == x[4] then v = check(opt, x[5]) end
			if v == nil then
				log.line("settings: " .. x[2] .. "." .. x[3] .. " " .. x[5] .. " is not allowed, default used")
			else
				M.values[x[2]] = M.values[x[2]] or {}
				M.values[x[2]][x[3]] = v
			end
		elseif x[1] == "key" and #x == 4 and B.match(x[4], "^[a-z0-9_]+$") then
			M.keys[x[2]] = M.keys[x[2]] or {}
			M.keys[x[2]][x[3]] = x[4]
		else
			ok = false
		end
		if not ok then
			bad = bad + 1
			first = first or n
		end
	end
	if bad > 0 then log.line("settings: skipped " .. bad .. " bad lines (first: line " .. first .. ")") end
end

-- The value of one option of the calling mod: the checked value from the file, else the default.
-- mod_id: read that mod's option instead, for code that runs outside the mod, such as a game script
-- the mod replaces.
function M.get(id, mod_id)
	local core = M.core
	local state = core and core.current
	if mod_id ~= nil then state = core and core.mods_by_id[B.tostring(mod_id)] or nil end
	if not state then return nil end
	local opt = find(state.mod.options, id)
	if not opt then return nil end
	local v = (M.values[state.id] or {})[id]
	if v == nil then return opt.default end
	return v
end

-- Calls fn once each time the key bound to `id` goes down: the key from the app, else the default.
-- opts.always: the session gate leaves it alone, as with obsidian.wrap.
function M.keybind(id, fn, opts)
	local state = M.core and M.core.current
	if not state or B.type(fn) ~= "function" then return end
	local declared = find(state.mod.keybinds, id)
	local key = (M.keys[state.id] or {})[id] or (declared and declared.default)
	if not key then
		log.line("keybind " .. state.id .. "." .. B.tostring(id) .. ": not declared")
		return
	end
	local always = B.type(opts) == "table" and opts.always == true
	B.insert(M.binds, { state = state, id = B.tostring(id), key = key, fn = fn, down = false, always = always })
end

-- From the frame hook. Keyboard.pressed may stay true while a key is held, so a bind fires on the
-- frame the key goes down only. Session mods react only in a cleared session, as their hooks do.
function M.tick()
	if #M.binds == 0 then return end
	local kb = B.rawget(B.G, "Keyboard")
	if B.type(kb) ~= "table" or B.type(kb.button_index) ~= "function" then return end
	local session = M.core.session
	for _, b in B.ipairs(M.binds) do
		if b.index == nil then
			local ok, i = B.pcall(kb.button_index, b.key)
			b.index = (ok and i) or false
			if not b.index then log.line("keybind " .. b.state.id .. "." .. b.id .. ": unknown key " .. b.key) end
		end
		if b.index then
			local ok, down = B.pcall(kb.pressed, b.index)
			down = ok and down and true or false
			if down and not b.down then
				local gated = b.state.mod.scope ~= "client" and not b.always
				if not b.state.off and not (gated and session and not session.allows(b.state)) then
					M.core.call(b.state, "keybind " .. b.id, b.fn)
				end
			end
			b.down = down
		end
	end
end

-- A text of the calling mod in the game language, then in English, then the shared catalog.
function M.text(key, ...)
	local core = M.core
	local state = core and core.current
	local texts = state and state.mod.texts
	if B.type(texts) == "table" then
		local t = (texts[core.language()] or {})[key] or (texts.en or {})[key]
		if t then
			local ok, s = B.pcall(B.format, t, ...)
			return ok and s or t
		end
	end
	return core.text(key, ...)
end

return M
