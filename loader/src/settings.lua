-- The settings file the app writes beside the saves (consent, option values and keys), and
-- obsidian.settings.get, obsidian.keybind and obsidian.text. Values are checked again against the
-- options in the deploy table, so a hand-edited file can't hand a mod a bad value.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local war = require("obsidian/loader/war")

local M = { values = {}, keys = {}, modes = {}, pads = {}, binds = {} }

-- The pad buttons a bind can be set to (pad lines), by the names Pad1.button_index takes.
M.PAD_BUTTONS = {
	a = true, b = true, x = true, y = true, left_shoulder = true, right_shoulder = true, left_trigger = true,
	right_trigger = true, left_thumb = true, right_thumb = true, back = true, start = true,
	d_up = true, d_down = true, d_left = true, d_right = true,
}

-- "a+left_shoulder" as a list of one to four known buttons, each once; nil when it is not one.
function M.pad_combo(text)
	local list, seen = {}, {}
	for name in B.gmatch(text .. "+", "([^+]*)%+") do
		if not M.PAD_BUTTONS[name] or seen[name] then return nil end
		seen[name] = true
		list[#list + 1] = name
	end
	if #list == 0 or #list > 4 then return nil end
	return list
end

-- How a bind fires, as in Helldivers 2: press (the frame the input goes down, the default), release
-- (the frame it comes up), tap (comes up within TAP_MAX of going down), double_tap (goes down a second
-- time within DOUBLE_GAP of the first), long_press (once, after HOLD of holding).
M.MODES = { press = true, release = true, tap = true, double_tap = true, long_press = true }
-- The settings format this loader reads (core/src/settings.rs FORMAT). A file of a newer format comes
-- from an Obsidian updated since the last deploy: its new lines are skipped, and the log says to deploy.
M.FORMAT = 1

M.TAP_MAX, M.DOUBLE_GAP, M.HOLD = 0.25, 0.3, 0.5
-- Seconds, for the modes. Tests set it.
M.now = B.clock

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
		elseif x[1] == "format" and #x == 2 and B.match(x[2], "^%d+$") then
			local got = B.tonumber(x[2])
			if got > M.FORMAT then
				core.settings_newer = true
				log.line("settings: written by a newer Obsidian (format " .. got .. ", this loader reads " .. M.FORMAT .. "): deploy again from Obsidian so every setting counts")
			end
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
		elseif x[1] == "war_achievements" and #x == 2 and (x[2] == "0" or x[2] == "1") then
			-- The custom war allows achievements; it counts only with a war line (guardrails.lua).
			core.war_achievements = x[2] == "1"
		elseif x[1] == "war_model" and #x == 2 and B.match(x[2], "^[a-z]+$") and #x[2] <= 16 then
			-- The war model ("enhanced"), for obsidian.war(); it counts only with a war line (war.lua).
			core.war_model = x[2]
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
		elseif x[1] == "pad" and #x == 4 and M.pad_combo(x[4]) then
			M.pads[x[2]] = M.pads[x[2]] or {}
			M.pads[x[2]][x[3]] = M.pad_combo(x[4])
		elseif x[1] == "mode" and #x == 4 and M.MODES[x[4]] then
			M.modes[x[2]] = M.modes[x[2]] or {}
			M.modes[x[2]][x[3]] = x[4]
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

-- Calls fn(device) each time the key bound to `id` fires by the input mode the player chose for it
-- (press when none): the key from the app, else the default. device is "key", or "pad" for opts.pad.
-- opts.always: the session gate leaves it alone, as with obsidian.wrap. opts.secondary_scope: the key
-- belongs to that secondary scope of the mod, and does nothing while it is off. opts.pad: a function
-- that says whether the controller input of the bind is held this frame; the same mode applies to it
-- (obsidian.capabilities.input_modes says this loader reads it). A controller input the player set in
-- the app (a pad line) replaces opts.pad, and gives any bind one (obsidian.capabilities.pad_binds).
-- A key named mouse_<button> is that mouse button: mouse_middle, mouse_right, mouse_extra_1.
function M.keybind(id, fn, opts)
	local state = M.core and M.core.current
	if not state or B.type(fn) ~= "function" then return end
	local declared = find(state.mod.keybinds, id)
	local key = (M.keys[state.id] or {})[id] or (declared and declared.default)
	if not key then
		log.line("keybind " .. state.id .. "." .. B.tostring(id) .. ": not declared")
		return
	end
	local scope, part = M.core.session.part_scope(state, opts)
	if scope == "off" then return end
	local always = B.type(opts) == "table" and opts.always == true
	local pad = B.type(opts) == "table" and B.type(opts.pad) == "function" and opts.pad or nil
	local combo = (M.pads[state.id] or {})[B.tostring(id)]
	if combo then pad = function() return M.pads_hold(combo) end end
	local mode = (M.modes[state.id] or {})[B.tostring(id)] or "press"
	B.insert(M.binds, { state = state, part = part, id = B.tostring(id), key = key, fn = fn, always = always, pad = pad, mode = mode,
		inputs = { key = { down = false }, pad = { down = false } } })
end

-- One input of a bind this frame: whether it fires by the mode. s keeps what the input did before.
function M.step(s, down, mode, now)
	local fire = false
	if down and not s.down then
		s.at, s.held = now, false
		if mode == "press" then
			fire = true
		elseif mode == "double_tap" then
			if s.first and now - s.first <= M.DOUBLE_GAP then
				fire, s.first = true, nil
			else
				s.first = now
			end
		end
	elseif down then
		if mode == "long_press" and not s.held and now - s.at >= M.HOLD then fire, s.held = true, true end
	elseif s.down then
		fire = mode == "release" or (mode == "tap" and now - s.at <= M.TAP_MAX)
	end
	s.down = down
	return fire
end

-- Whether any active pad holds every button of `combo` now. Triggers count past half way.
-- Button indices by pad and name, looked up once: the binds are read every frame.
local pad_index = B.setmetatable({}, { __mode = "k" })
local function index_of(pad, name)
	local known = pad_index[pad]
	if not known then
		known = {}
		pad_index[pad] = known
	end
	local i = known[name]
	if i == nil then
		local ok, got = B.pcall(pad.button_index, name)
		i = (ok and got) or false
		known[name] = i
	end
	return i
end

function M.pads_hold(combo)
	local list = B.rawget(B.G, "Pads")
	if B.type(list) ~= "table" then return false end
	for _, pad in B.ipairs(list) do
		local active = B.type(pad) == "table" and (B.type(pad.active) ~= "function" or B.select(2, B.pcall(pad.active)) == true)
		if active then
			local all = true
			for k = 1, #combo do
				local i = index_of(pad, combo[k])
				local fine, level = false, nil
				if i then fine, level = B.pcall(pad.button, i) end
				if not (fine and B.type(level) == "number" and level > 0.5) then
					all = false
					break
				end
			end
			if all then return true end
		end
	end
	return false
end

-- Where a bind's key is read: the keyboard, or the mouse for mouse_<button>. The index, or false
-- when the device doesn't know the name.
local function key_source(b)
	local name, dev = b.key, B.rawget(B.G, "Keyboard")
	if B.sub(name, 1, 6) == "mouse_" then name, dev = B.sub(name, 7), B.rawget(B.G, "Mouse") end
	if B.type(dev) ~= "table" or B.type(dev.button_index) ~= "function" then return false end
	local ok, i = B.pcall(dev.button_index, name)
	b.device = dev
	return (ok and i) or false
end

-- Whether the key of a bind is down. Keyboard.pressed stays true while a key is held; a mouse button
-- is read by its level.
local function key_down(b)
	if b.device == B.rawget(B.G, "Mouse") then
		local ok, level = B.pcall(b.device.button, b.index)
		return ok and B.type(level) == "number" and level > 0.5
	end
	local ok, down = B.pcall(b.device.pressed, b.index)
	return ok and down and true or false
end

local function fire(b, device)
	local session = M.core.session
	local gated = not b.always and session and session.gated(b.state, b.part)
	if not b.state.off and not (gated and session and not session.allows(b.state, b.part)) then
		M.core.call(b.state, "keybind " .. b.id, b.fn, device)
	end
end

-- From the frame hook. Keyboard.pressed stays true while a key is held, so each input is followed
-- from frame to frame and the mode says when it fires. Session mods react only in a cleared session,
-- as their hooks do.
function M.tick()
	if #M.binds == 0 then return end
	local kb = B.rawget(B.G, "Keyboard")
	if B.type(kb) ~= "table" or B.type(kb.button_index) ~= "function" then return end
	local now = M.now()
	for _, b in B.ipairs(M.binds) do
		if b.index == nil then
			b.index = key_source(b)
			if not b.index then log.line("keybind " .. b.state.id .. "." .. b.id .. ": unknown key " .. b.key) end
		end
		if b.index then
			if M.step(b.inputs.key, key_down(b), b.mode, now) then fire(b, "key") end
		end
		if b.pad then
			local ok, down = B.pcall(b.pad)
			if M.step(b.inputs.pad, ok and down and true or false, b.mode, now) then fire(b, "pad") end
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
