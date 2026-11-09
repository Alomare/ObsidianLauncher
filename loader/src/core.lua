-- Obsidian Loader core: the boot sequence, starting mods, error isolation and the `obsidian` API.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local hooks = require("obsidian/loader/hooks")
local session = require("obsidian/loader/session")
local expansion = require("obsidian/loader/expansion")
local backend_block = require("obsidian/loader/backend_block")
local guardrails = require("obsidian/loader/guardrails")
local pool = require("obsidian/loader/pool")
local status = require("obsidian/loader/status")
local settings = require("obsidian/loader/settings")
local game_text = require("obsidian/loader/game_text")
local menu = require("obsidian/loader/menu")
local ui = require("obsidian/loader/ui")
local storage = require("obsidian/loader/storage")
local save_data = require("obsidian/loader/save_data")
local sandbox = require("obsidian/loader/sandbox")
local lobby = require("obsidian/loader/lobby")

local API_VERSION = 1
local MAX_ERRORS = 10
local STDLIB = { "string", "table", "math", "io", "os", "coroutine", "debug", "package", "bit", "jit" }

local M = {
	mods = {}, mods_by_id = {}, notices = {}, notices_shown = {}, after = {}, frame_callbacks = {},
	current = nil, deploy = nil, consent_on = false, block_achievements = false, freeze_progress = false, frame_count = 0,
}

local capabilities = {
	after_startup = true, log = true, hook = true, wrap = true, on_require = true, on_session = true, session = true,
	settings = true, keybind = true, text = true, on_frame = true, module = true, import = true, menu = true, toast = true, notify = true, storage = true, caller = true,
	ui = true, scope = true, guardrails = true, info = true, expansion = true, save_data = true,
}
local api = {
	version = API_VERSION,
	hook = hooks.hook,
	wrap = hooks.wrap,
	on_require = hooks.on_require,
	on_session = session.on_session,
	session = session.api(),
	expansion = expansion.api(),
	settings = { get = settings.get },
	keybind = settings.keybind,
	text = settings.text,
	menu = menu.api(),
	ui = ui.api(),
	toast = menu.toast,
	storage = storage.api(),
	save_data = save_data.api(),
	guardrails = guardrails.api(M),
	capabilities = B.setmetatable({}, {
		__index = capabilities,
		__newindex = function() B.error("obsidian.capabilities is read-only", 2) end,
		__metatable = false,
	}),
}
M.api = api
M.capabilities = capabilities

-- "Allow session mods from other Obsidian users", from the settings file.
function M.consent()
	return M.consent_on == true
end

-- Whether this launch was started on a Sandbox save, usable or not.
function M.sandboxed()
	return sandbox.wanted(M)
end

function M.notice(key, ...)
	B.insert(M.notices, { key = key, args = { ... } })
end

local LANGUAGES = { en = "en", fr = "fr", es = "es", bp = "pt-BR" }

function M.language()
	local lm = B.rawget(B.G, "LocalizationManager")
	local code = B.type(lm) == "table" and lm.current_language or nil
	return LANGUAGES[code] or "en"
end

-- A loader text in the game language, English when the language or the key is missing.
function M.text(key, ...)
	local texts = (M.deploy and M.deploy.texts) or {}
	local t = (texts[M.language()] or {})[key] or (texts.en or {})[key] or key
	local ok, s = B.pcall(B.format, t, ...)
	return ok and s or t
end

-- Notices are queued anywhere and shown from the frame hook, where the game UI exists.
local NOTICE_SECONDS = 10
local NOTIFY_SECONDS = 7
local MOD_WAITING = 3
local NOTICE_KEY, MOD_KEY = "obsidian_notice/", "obsidian_mod/"

-- The game's own hint box, on the ship and in a mission. One key per text, because it leaves out a
-- text it already has: a notice raised again doesn't queue up. The override shows it with hints off.
local function show_hint(n)
	local boot, settings = B.rawget(B.G, "Boot"), B.rawget(B.G, "HintSettings")
	local machine = B.type(boot) == "table" and boot.state_machine
	if not machine or B.type(settings) ~= "table" then return false end
	local box = machine:get_current_state().hint_manager
	if not box then return false end
	local key = NOTICE_KEY .. n.key .. "/" .. B.concat(n.args, "/")
	settings[key] = settings[key] or { audio = "ui_select", duration = NOTICE_SECONDS, icon = "icon_hint", text = key }
	game_text.replace(M, hooks, key, n.key, n.args)
	box:add_hint(key, true, nil, nil, NOTICE_SECONDS)
	-- What the loader says goes ahead of what mods queued, behind what the loader queued before.
	local queue = box.queued_hints
	if B.type(queue) == "table" and queue[#queue] and queue[#queue].text == key then
		local to = 1
		while to < #queue and B.sub(queue[to].text, 1, #NOTICE_KEY) == NOTICE_KEY do to = to + 1 end
		B.insert(queue, to, B.remove(queue))
	end
	return true
end

-- obsidian.notify: a mod's words in the same box, for what's rare and matters. The box shows one at
-- a time, so a mod can only have so many waiting. Where there's no box, it's a toast.
local function notify(text, seconds)
	if B.type(text) ~= "string" or text == "" then return false, "bad_type" end
	if seconds ~= nil and not (B.type(seconds) == "number" and seconds > 0) then return false, "out_of_range" end
	seconds = seconds or NOTIFY_SECONDS
	local boot, settings = B.rawget(B.G, "Boot"), B.rawget(B.G, "HintSettings")
	local machine = B.type(boot) == "table" and boot.state_machine
	local ok, state = B.pcall(function() return machine:get_current_state() end)
	local box = ok and B.type(state) == "table" and state.hint_manager
	if not box or B.type(settings) ~= "table" then
		menu.toast(text, seconds)
		return true
	end
	local id = M.current and M.current.id or "unknown"
	local mine = MOD_KEY .. id .. "/"
	local waiting = 0
	for _, q in B.ipairs(box.queued_hints or {}) do
		if B.sub(q.text, 1, #mine) == mine then waiting = waiting + 1 end
	end
	if waiting >= MOD_WAITING then
		log.limited(id .. "\0notify", "notify: " .. id .. " has " .. MOD_WAITING .. " waiting, so this one is dropped: " .. text)
		return false, "queue_full"
	end
	local key = mine .. text
	settings[key] = settings[key] or { audio = "ui_select", duration = seconds, icon = "icon_hint", text = key }
	game_text.replace(M, hooks, key, "%s", { text })
	box:add_hint(key, true, nil, nil, seconds)
	return true
end
api.notify = notify

local function show_notices()
	while #M.notices > 0 do
		local n = B.remove(M.notices, 1)
		B.insert(M.notices_shown, n)
		local text = M.text(n.key, B.unpack(n.args))
		log.line("notice: " .. n.key .. ": " .. text)
		local ok, shown = B.pcall(show_hint, n)
		if not ok then log.limited("hint box", "hint box failed: " .. B.tostring(shown)) end
		if not (ok and shown) then menu.toast(text, NOTICE_SECONDS) end
	end
end

local function traceback(err)
	if B.traceback then return B.traceback(B.tostring(err), 2) end
	return B.tostring(err)
end

-- Runs fn for a mod, protected. Errors are logged at most 3 times per site, and a mod that keeps
-- erroring is switched off for the rest of the session.
function M.call(state, site, fn, ...)
	if state and state.off then return false end
	local previous = M.current
	M.current = state
	local args, n = { ... }, B.select("#", ...)
	local result = { B.xpcall(function() return fn(B.unpack(args, 1, n)) end, traceback) }
	M.current = previous
	if not result[1] and state then
		state.errors = state.errors + 1
		log.limited(state.id .. "\0" .. site, "error " .. state.id .. " " .. site .. ": " .. B.tostring(result[2]))
		if state.errors >= MAX_ERRORS then
			state.off = true
			log.line("switched off " .. state.id .. " after " .. MAX_ERRORS .. " errors")
			M.notice("mod_failed", state.id)
		end
	end
	return B.unpack(result)
end

local function snapshot()
	local s = { globals = {}, stdlib = {}, loaded = {} }
	for k, v in B.pairs(B.G) do s.globals[k] = v end
	for _, lib in B.ipairs(STDLIB) do
		local t = B.rawget(B.G, lib)
		if B.type(t) == "table" then
			for k, v in B.pairs(t) do s.stdlib[lib .. "." .. B.tostring(k)] = v end
		end
	end
	local package = B.rawget(B.G, "package")
	if B.type(package) == "table" and B.type(package.loaded) == "table" then
		for k in B.pairs(package.loaded) do s.loaded[k] = true end
	end
	return s
end

local function diff(label, before, after, out)
	local added, replaced = {}, {}
	for k, v in B.pairs(after) do
		if before[k] == nil then
			B.insert(added, B.tostring(k))
		elseif not B.rawequal(before[k], v) then
			B.insert(replaced, B.tostring(k))
		end
	end
	for k in B.pairs(before) do
		if after[k] == nil then B.insert(replaced, B.tostring(k)) end
	end
	B.sort(added)
	B.sort(replaced)
	if #added > 0 then B.insert(out, label .. "+=" .. B.concat(added, ",")) end
	if #replaced > 0 then B.insert(out, label .. "~=" .. B.concat(replaced, ",")) end
end

-- One line per mod naming the globals, standard library members and modules the mod added or replaced.
local function changes(before, after)
	local out = {}
	diff("globals", before.globals, after.globals, out)
	diff("stdlib", before.stdlib, after.stdlib, out)
	diff("loaded", before.loaded, after.loaded, out)
	if #out == 0 then return "none" end
	return B.concat(out, " ")
end

function api.log(...)
	local state = M.current
	if state then log.mod(state.id, ...) else log.line("mod (unknown):", ...) end
end

-- fn runs once per frame, protected; returning true stops the calls. A session mod's callbacks
-- run only in a cleared session, like the mod's hooks.
function api.on_frame(fn)
	if B.type(fn) == "function" then B.insert(M.frame_callbacks, { state = M.current, fn = fn }) end
end

local function run_frame_callbacks()
	local list, keep = M.frame_callbacks, {}
	for _, cb in B.ipairs(list) do
		local state = cb.state
		local gated = not (state and state.mod.scope == "client")
		local stop = false
		if state and state.off then
			stop = true
		elseif not (gated and not session.allows(state)) then
			local ok, result = M.call(state, "on_frame", cb.fn)
			stop = ok and result == true
		end
		if not stop then B.insert(keep, cb) end
	end
	M.frame_callbacks = keep
end

-- How a mod runs in this deploy: "client", "session" or "expansion". A library has become client or
-- session: session when a session mod or an expansion requires it, so the library sets up what the
-- expansion may use.
local function scope_of(state)
	local scope = state.mod.scope
	if scope == "client" then return "client" end
	if scope == "expansion" and expansion.is_own(state.id) then return "expansion" end
	return "session"
end

function api.scope()
	local state = M.current
	if not state then return nil end
	return scope_of(state)
end

-- A mod's name from its obsidian.json, in the game's language, else English, else the id.
function M.name_of(state)
	local names = B.type(state.mod.name) == "table" and state.mod.name or {}
	local name = names[M.language()] or names.en
	return B.type(name) == "string" and name or state.id
end

function api.after_startup(fn)
	B.insert(M.after, { state = M.current, fn = fn })
end

-- The id of the mod whose code is running, so a shared mod can say who asked for something.
function api.caller()
	return M.current and M.current.id or nil
end

-- What a running mod is: id, name (from its obsidian.json, in the game's language, else English),
-- version and scope, as obsidian.scope() gives it. id: nil for the calling mod. nil for a mod that
-- isn't running.
function api.info(id)
	local state
	if id == nil then state = M.current else state = M.mods_by_id[B.tostring(id)] end
	if not state or state.off then return nil end
	return { id = state.id, name = M.name_of(state), version = B.tostring(state.mod.version), scope = scope_of(state) }
end

-- Loads a script of the calling mod, named like the file in `modules` without `.lua`.
function api.module(name)
	local state = M.current
	if not state then B.error("obsidian.module needs a running mod", 2) end
	return B.require("obsidian/mods/" .. state.id .. "/" .. B.tostring(name))
end

-- What the entry script of another mod returned; nil when that mod isn't running. A mod that
-- needs one lists it in `requires`, which starts that mod first.
function api.import(id)
	local state = M.mods_by_id[id]
	if state and not state.off then return state.exports end
	return nil
end

local function refuse(state, why)
	state.off = true
	log.line("refused " .. state.id .. ": needs " .. why)
	M.notice("mod_failed", state.id)
end

local function start(mod)
	local state = { id = B.tostring(mod.id), mod = mod, errors = 0, off = false }
	B.insert(M.mods, state)
	M.mods_by_id[state.id] = state
	local needs = B.tonumber(mod.loader_api) or 1
	if needs > API_VERSION then return refuse(state, "loader API " .. needs) end
	for _, id in B.ipairs(mod.requires or {}) do
		local required = M.mods_by_id[id]
		if not required or required.off then return refuse(state, id) end
	end
	log.line("loading " .. state.id)
	local before = snapshot()
	local ok, err = M.call(state, "start", B.require, mod.entry)
	if ok then
		state.exports = err
		log.line("started " .. state.id)
	else
		state.off = true
		log.line("failed " .. state.id .. ": " .. B.tostring(err))
		M.notice("mod_failed", state.id)
	end
	log.line("changes: " .. state.id .. " " .. changes(before, snapshot()))
end

function M.boot()
	local jit = B.rawget(B.G, "jit")
	local jit_was_on = false
	if B.type(jit) == "table" then
		-- The game and every mod share one machine-code cache; LuaJIT 2.0 defaults are 512 KB and 1000 traces.
		if B.type(jit.opt) == "table" then B.pcall(jit.opt.start, "maxmcode=4096", "maxtrace=4000") end
		if jit.status then jit_was_on = (jit.status()) == true end
		-- One-time loader code leaves no traces behind.
		if jit_was_on then B.pcall(jit.off) end
	end

	local appdata = B.getenv("APPDATA")
	local game_side = appdata and (appdata .. "/Arrowhead/Helldivers/ObsidianLauncher")
	if game_side then log.setup(game_side .. "/logs") end
	log.line("Obsidian Loader, API " .. API_VERSION .. ", jit " .. (B.type(jit) == "table" and "available" or "missing"))

	local ok, deploy = B.pcall(B.require, "obsidian/deploy")
	local deployed = ok and B.type(deploy) == "table"
	M.deploy = deployed and deploy or nil
	-- Before the checks below: a launch on a Sandbox that stops at one of them must still keep the
	-- game off the Official save.
	settings.load(M, game_side and (game_side .. "/settings.txt"))
	hooks.setup(M, session)
	local function stop_saving()
		if not sandbox.wanted(M) then return end
		sandbox.setup(M, hooks, true)
		hooks.on_frame(show_notices)
		hooks.wrap_require()
	end
	if not deployed then
		log.line("no deploy table, no mod started: " .. B.tostring(deploy))
		stop_saving()
		if jit_was_on then B.pcall(jit.on) end
		return
	end

	local revision = B.rawget(B.G, "BUILD_REVISION_INFORMATION")
	local expected = B.tostring(deploy.revision)
	if B.type(revision) ~= "string" or B.sub(revision, 1, #expected) ~= expected then
		log.line("build unknown, no mod started (expected " .. expected .. ")")
		M.notice("build_unknown")
		stop_saving()
		if jit_was_on then B.pcall(jit.on) end
		return
	end

	sandbox.setup(M, hooks)
	local tag = pool.apply(deploy, M.sandboxed())

	M.session = session
	M.expansion = expansion
	session.setup(M)
	hooks.on_frame(function() M.frame_count = M.frame_count + 1 end)
	hooks.on_frame(session.tick)
	expansion.setup(M, hooks, session)
	hooks.on_frame(settings.tick)
	hooks.on_frame(show_notices)
	hooks.on_frame(run_frame_callbacks)
	menu.setup(M, hooks, session)
	ui.setup(M, hooks, session)
	storage.setup(M, game_side and (game_side .. "/logs"))
	save_data.setup(M, appdata and (appdata .. "/Arrowhead/Helldivers/saves"))
	hooks.wrap_require()

	backend_block.setup(M, hooks)
	guardrails.setup(M, hooks)
	lobby.setup(M, hooks, session)
	-- The lobby browser also lists the open pool's lobbies, and the public door keeps content that
	-- can't switch off out of public lobbies with vanilla players.
	pool.bridge(M, hooks, tag)
	status.setup(M, hooks)

	B.rawset(B.G, "obsidian", api)
	for _, mod in B.ipairs(deploy.mods or {}) do
		start(mod)
	end
	-- Pages added before Obsidian API started go to its window now.
	menu.flush()
	for _, cb in B.ipairs(M.after) do
		M.call(cb.state, "after_startup", cb.fn)
	end
	if jit_was_on then B.pcall(jit.on) end
	log.line("boot done")
end

return M
