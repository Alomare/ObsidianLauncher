-- When a session is cleared for session mods (spec 6.2), and the lobby handshake (spec 6.3, layer 2).
-- Only a public lobby holds session mods back: friends-only and private lobbies are the players' own
-- choice of company, so session mods run there with anyone.
-- Every Obsidian player publishes lobby member data "obsidian" = "1;<mod-set hash>;<consent 0|1>",
-- where consent 1 means other players' session mods may run with them (the default; 0 is the
-- guardrail "Block session mods from others in my public lobby"),
-- and "obsidian_every" = "<id>:<hash>,..." for the mods every player has to run. Expansions follow
-- their content instead (expansion.lua).
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local expansion = require("obsidian/loader/expansion")
local pool = require("obsidian/loader/pool")

local EVERY = 30
local EVERY_KEY = "obsidian_every"

local M = { state = true, missing = {}, members = {}, listeners = {}, sent = {}, sent_to = nil, sent_state = nil, warned = nil, privacy = nil }

function M.setup(core)
	M.core = core
end

-- Anything but exactly version 1, 64 lowercase hex digits and 0 or 1 counts as not a Obsidian player.
function M.parse(value)
	if B.type(value) ~= "string" then return nil end
	local version, hash, consent = B.match(value, "^(%d+);([0-9a-f]+);([01])$")
	if version ~= "1" or #hash ~= 64 then return nil end
	return { modset = hash, consent = consent == "1" }
end

local function lobby()
	local network = B.rawget(B.G, "NetworkGlobal")
	-- Between leaving one lobby and joining the next, the room has no engine lobby in it.
	if B.type(network) == "table" and network.lobby and network.lobby.lobby then return network.lobby end
	return nil
end
M.lobby = lobby

-- Whether the lobby is public, by the privacy its host set (GameSettings.room_privacy_settings, in the
-- room data every member reads). A lobby whose privacy can't be read counts as public.
function M.public(l)
	local settings = B.rawget(B.G, "GameSettings")
	local kinds = B.type(settings) == "table" and settings.room_privacy_settings or nil
	local room = pool.room(l)
	local privacy = room and B.tonumber(room.privacy)
	local public = privacy == nil or B.type(kinds) ~= "table" or privacy == kinds.public
	local name = privacy == nil and "unknown, counted as public" or public and "public" or "not public"
	if name ~= M.privacy then
		M.privacy = name
		log.line("session: lobby privacy " .. name)
	end
	return public
end

-- Member data belongs to the engine's lobby. The game keeps one NetworkRoom when a player leaves
-- their own lobby to join another and swaps the engine's lobby inside it, so that's what counts as
-- a new lobby here. Everything goes out again once it's joined, in case a write while joining is lost.
local function publish(l, key, value)
	local to, state = l.lobby, l:state()
	if M.sent_to ~= to or M.sent_state ~= state then M.sent, M.sent_to, M.sent_state = {}, to, state end
	if M.sent[key] == value then return false end
	l:set_member_data(key, value)
	M.sent[key] = value
	return true
end
M.publish = publish

-- Whether a mod's gated code may run: a cleared session, and for a mod every player has to run,
-- every other player running it. An expansion's runs only while its content does; the code of a
-- session secondary scope of an expansion (`part`) also runs in a cleared session outside it. A
-- library that only expansions require runs as a client library, and serves them while their content
-- runs. A library a session mod requires runs while the session is cleared, and while the content of
-- an expansion that requires it runs, whoever else is in the lobby.
function M.allows(state, part)
	local mod = state and state.mod
	if mod and mod.scope == "expansion" then
		if not expansion.is_own(state.id) or expansion.serves(mod.expansions) then return true end
		if not (part and part.scope == "session") then return false end
		return M.state and not M.missing[state.id] or false
	end
	if mod and mod.expansions and expansion.serves(mod.expansions) then return true end
	return M.state and not (state and M.missing[state.id])
end

-- The secondary scope a registration names (opts.secondary_scope, or opts.part from before Hybrid
-- Scopes had the name), with what the code tagged with it runs as: "client", "session", or "off" for a
-- secondary scope that is off, whose code never runs. nil without one, or with one the mod doesn't
-- declare: then the primary scope decides, a hybrid mod's as a session mod's.
function M.part_scope(state, opts)
	local id = B.type(opts) == "table" and (opts.secondary_scope or opts.part) or nil
	if id == nil or not state then return nil end
	local part = state.parts and state.parts[B.tostring(id)]
	if not part then
		log.limited(state.id .. "\0part " .. B.tostring(id), "secondary_scope: " .. state.id .. " names " .. B.tostring(id) .. ", which is not one of the mod's secondary scopes")
		return nil
	end
	if not part.on then return "off", part end
	return part.scope == "client" and "client" or "session", part
end

-- Whether code of `state` tagged with the secondary scope `part` (nil: untagged) runs only in a cleared session.
function M.gated(state, part)
	if part then return part.scope ~= "client" end
	return not (state and state.mod.scope == "client")
end

-- What obsidian.on_session reports: whether the loader has the mod on. For a library that only
-- expansions require, that's their content, so it can undo what an expansion changed once it ends.
-- A client secondary scope is always on.
local function switched(state, part)
	if part and part.scope == "client" then return true end
	local mod = state and state.mod
	if mod and mod.scope == "expansion" and not (part and part.scope == "session") then return expansion.serves(mod.expansions) end
	return M.allows(state, part)
end

-- Whether a mod has a session secondary scope that is on, which stops while the others keep running:
-- a hybrid mod, or an expansion outside the content.
local SESSION = { scope = "session" }
local function has_session_secondary(state)
	if state.mod.scope == "hybrid" then return true end
	if state.mod.scope ~= "expansion" then return false end
	for _, p in B.ipairs(state.part_list or {}) do
		if p.on and p.scope == "session" then return true end
	end
	return false
end

-- A notice when the session secondary scopes of a mod stop and the rest keeps running.
local function parts_notice()
	for _, state in B.ipairs(M.core.mods or {}) do
		if has_session_secondary(state) and not state.off then
			local now = M.allows(state, SESSION)
			if state.parts_were ~= false and not now then
				local lang, names = M.core.language(), {}
				for _, p in B.ipairs(state.part_list or {}) do
					if p.on and p.scope ~= "client" then
						local n = B.type(p.name) == "table" and (p.name[lang] or p.name.en) or nil
						B.insert(names, B.type(n) == "string" and n ~= "" and n or p.id)
					end
				end
				if #names > 0 then
					log.line("session: " .. state.id .. " session secondary scopes off, the rest on")
					local list = #names == 1 and names[1] or (B.concat(names, ", ", 1, #names - 1) .. " and " .. names[#names])
					M.core.notice(#names == 1 and "part_off" or "parts_off", M.core.name_of(state), list)
				end
			end
			state.parts_were = now
		end
	end
end

-- Tells each on_session listener whose answer changed.
function M.refresh()
	for _, l in B.ipairs(M.listeners) do
		local now = switched(l.state, l.part)
		if now ~= l.on then
			l.on = now
			M.core.call(l.state, "on_session", l.fn, now)
		end
	end
	parts_notice()
end

-- The mods every player has to run, as "<id>:<hash>", sorted.
local function every_pairs()
	local out = {}
	for id, hash in B.pairs(M.core.deploy.every or {}) do B.insert(out, B.tostring(id) .. ":" .. B.tostring(hash)) end
	B.sort(out)
	return out
end

-- missing: the mods every player has to run that some other player doesn't.
local function set_cleared(on, missing)
	missing = missing or M.missing
	if on ~= M.state then log.line("session: " .. (on and "cleared" or "not cleared")) end
	for id in B.pairs(missing) do
		if not M.missing[id] then
			log.line("session: " .. id .. " off, a player here doesn't run it")
			M.core.notice("every_player_off", id)
		end
	end
	for id in B.pairs(M.missing) do
		if not missing[id] then log.line("session: every player here runs " .. id) end
	end
	M.state, M.missing = on, missing
	M.refresh()
end

function M.evaluate()
	local core, deploy = M.core, M.core.deploy
	local own_consent = core.consent()
	local l = lobby()
	if not l then
		M.members = {}
		set_cleared(true, {})
		return
	end
	local value = "1;" .. B.tostring(deploy.modset) .. ";" .. (own_consent and "1" or "0")
	if publish(l, "obsidian", value) then log.line("session: published member data " .. value) end
	local own_every = every_pairs()
	publish(l, EVERY_KEY, B.concat(own_every, ","))
	local me, list, signature = l.my_id, {}, {}
	-- In a public lobby my session mods run when every other player runs the same set or allows
	-- other players' mods; a player without Obsidian allows nothing. Anywhere else they run. My own
	-- consent is about their mods in my game, so it plays no part here.
	local public = M.public(l)
	local allowed, missing = true, {}
	for _, peer in B.ipairs(l:members() or {}) do
		if peer ~= me then
			local d = M.parse(l:member_data(peer, "obsidian"))
			B.insert(list, { peer = peer, obsidian = d ~= nil, modset = d and d.modset, consent = d and d.consent or false })
			B.insert(signature, B.tostring(peer))
			if public and not (d and (d.modset == deploy.modset or d.consent)) then allowed = false end
			-- Agreeing to other players' mods isn't enough here: they have to run the same one.
			if #own_every > 0 then
				local theirs = {}
				local data = d and l:member_data(peer, EVERY_KEY)
				if B.type(data) == "string" then
					for pair in B.gmatch(data, "[^,]+") do theirs[pair] = true end
				end
				for _, pair in B.ipairs(own_every) do
					if not theirs[pair] then missing[B.match(pair, "^(.*):")] = true end
				end
			end
		end
	end
	M.members = list
	local on = allowed
	set_cleared(on, missing)
	-- Data overrides and legacy mods can't be switched off live, so everyone running Obsidian gets told.
	if not on and deploy.has_static_session then
		B.sort(signature)
		local key = B.concat(signature, ",")
		if M.warned ~= key then
			M.warned = key
			core.notice("session_not_cleared")
		end
	elseif on then
		M.warned = nil
	end
end

function M.tick()
	local n = M.core.frame_count
	if n % EVERY == 0 then
		local ok, err = B.pcall(M.evaluate)
		if not ok then
			log.limited("session", "session check failed: " .. B.tostring(err))
			-- Not knowing who's here can't count as cleared.
			set_cleared(false)
		end
	end
end

function M.cleared()
	return M.state
end

-- opts.secondary_scope: the listener hears only the switch of that secondary scope. A client one never
-- switches off, and one that is off registers nothing.
function M.on_session(fn, opts)
	local state = M.core.current
	local scope, part = M.part_scope(state, opts)
	if scope == "off" then return end
	B.insert(M.listeners, { state = state, fn = fn, part = part, on = switched(state, part) })
end

-- obsidian.session.cleared(id): for the calling mod, or for one of the calling mod's secondary scopes.
function M.cleared_for(part)
	local state = M.core.current
	if part ~= nil then
		local scope, p = M.part_scope(state, { secondary_scope = part })
		if scope == "off" then return false end
		if p then return switched(state, p) end
	end
	return M.allows(state)
end

-- A guarded function of the mod `state`, called by another mod. An expansion gets full use of the
-- libraries it requires while its content runs, whatever the lobby, and none of anyone's outside it.
local function guarded(state, v)
	return function(...)
		local caller, mod = M.core.current, state.mod
		if caller and caller ~= state and expansion.is_own(caller.id) then
			if not expansion.running(caller.id) then return false, "not_started" end
			for _, id in B.ipairs(mod.expansions or {}) do
				if id == caller.id then return v(...) end
			end
		end
		if mod.scope == "client" then return false, "needs_session" end
		if mod.scope == "expansion" then
			-- An expansion's own table opens with its content. A library only expansions require
			-- refuses everyone else, as a client library does.
			if expansion.is_own(state.id) then
				if expansion.serves(mod.expansions) then return v(...) end
				return false, "not_started"
			end
			if caller == state and expansion.serves(mod.expansions) then return v(...) end
			return false, "needs_session"
		end
		if not M.allows(state) then return false, "not_cleared" end
		return v(...)
	end
end

-- A view of a mod's table for other mods to call. Every function not named in `free` answers
-- false and a reason when the calling mod's scope or the lobby doesn't allow it, so a function
-- nobody sorted is refused rather than let through. The mod's own code keeps using the real table,
-- because undoing a change has to work after the session stops being cleared.
function M.guard(t, free)
	local state = M.core.current
	if not state then B.error("obsidian.session.guard needs a running mod", 2) end
	local open, wrapped = {}, {}
	for _, name in B.ipairs(free or {}) do
		open[name] = true
		if t[name] == nil then log.line("guard: " .. state.id .. " lists " .. B.tostring(name) .. ", which doesn't exist") end
	end
	return B.setmetatable({}, {
		__index = function(_, k)
			local v = t[k]
			if B.type(v) ~= "function" or open[k] then return v end
			if not wrapped[v] then wrapped[v] = guarded(state, v) end
			return wrapped[v]
		end,
		__newindex = t,
	})
end

function M.api()
	return {
		guard = M.guard,
		-- For the calling mod, so a mod every player has to run hears when one doesn't. With a part
		-- id: for that part of the calling mod, true for a client part, false for a part that is off.
		cleared = function(part) return M.cleared_for(part) end,
		-- A player's own save is theirs, so Obsidian holds none of it back. These stay for the mods that
		-- still ask, and do nothing.
		forfeit_progress = function() end,
		release_progress = function() return false end,
		hold_progress = function() end,
		members = function()
			local copy = {}
			for i, m in B.ipairs(M.members) do
				copy[i] = { peer = m.peer, obsidian = m.obsidian, modset = m.modset, consent = m.consent }
			end
			return copy
		end,
	}
end

return M
