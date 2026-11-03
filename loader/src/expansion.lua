-- Expansions: mods with content of their own, such as a game mode, that leave the rest of the game
-- alone. At deploy an expansion counts as a client mod: normal matchmaking, Galactic War results and
-- Proving Grounds. Its gated code (hooks, frame functions, keys, guarded calls) stays off until the
-- host's copy starts the content with obsidian.expansion.start(), which happens only once every
-- player in the lobby runs the same expansion at the same version and shared option values. Every
-- other machine follows the host through member data. stop() ends the content, and so does leaving
-- the lobby.
-- While content runs: the libraries the expansion requires give it full use (session.guard), the
-- results of the mission stay off the Galactic War (backend_block, through M.taint), the host's
-- lobby carries the expansion's own pool, and a player without the expansion is turned away at the
-- join request, at the session request, or removed by the game's kick.
-- Member data: every player with expansions publishes "obsidian_expansions" = "<id>:<hash>,..."
-- (sorted), and the host "obsidian_run" = "<id>:<hash>" while content runs. Who else is in the
-- lobby never switches running content off, so a joiner the host turns away can't stop it.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local LIST_KEY, RUN_KEY = "obsidian_expansions", "obsidian_run"
local EVERY = 30
-- A join request waits this long for the joiner's member data. The joiner gives up after 10 seconds.
local HOLD_SECONDS = 4
-- Checks in a row before the host removes a player in the mission who doesn't run the content.
local STRIKES = 3
-- Checks the content waits for a new host to say it still runs, after the host leaves.
local HANDOVER = 3
-- Reasons the game knows, since the joiner's own game looks them up.
local JOIN_DENIED, SESSION_DENIED = "network_join_denied", "network_join_session_denied"

local M = {}

local function reset()
	M.run, M.taint, M.last, M.held, M.strikes, M.told, M.turned = nil, false, nil, {}, {}, {}, {}
	M.checks, M.host, M.host_since = 0, nil, 0
end
reset()
M.mine, M.tags, M.pools = {}, {}, {}
-- Wall-clock seconds, for held join requests. Tests set it.
M.now = B.time

local function global(name)
	return B.rawget(B.G, name)
end

local function pack(...)
	return { n = B.select("#", ...), ... }
end

local function contains(list, id)
	for _, x in B.ipairs(list or {}) do
		if x == id then return true end
	end
	return false
end

-- Whether id is an expansion of this deploy, as opposed to a library that serves one.
function M.is_own(id)
	return id ~= nil and M.mine[id] ~= nil
end

-- Whether content runs for one of the expansions listed, on this machine.
function M.serves(list)
	return M.run ~= nil and contains(list, M.run.id)
end

local function pair_of(id)
	return id .. ":" .. B.tostring(M.mine[id])
end

local function own_pairs()
	local out = {}
	for id in B.pairs(M.mine) do B.insert(out, pair_of(id)) end
	B.sort(out)
	return B.concat(out, ",")
end

-- The room hash a lobby carries while the content of id runs: the game's own hash and the tag.
local function pool_of(id)
	local tag = M.tags[id]
	if not M.base or B.type(tag) ~= "string" then return nil end
	return M.base .. tag
end

local function current_state()
	local boot = global("Boot")
	local machine = B.type(boot) == "table" and boot.state_machine
	if not machine then return nil end
	local ok, st = B.pcall(function() return machine:get_current_state() end)
	return ok and B.type(st) == "table" and st or nil
end

-- Whether a player runs this content: true, false, or nil while nothing has come from them yet.
local function runs(l, peer, pair)
	local list = l:member_data(peer, LIST_KEY)
	if B.type(list) == "string" and list ~= "" then
		for p in B.gmatch(list, "[^,]+") do
			if p == pair then return true end
		end
		return false
	end
	-- An Obsidian player who publishes no list runs no expansions, or an Obsidian from before them.
	if M.session.parse(l:member_data(peer, "obsidian")) then return false end
	return nil
end

-- A player's name from the game's player objects, which the ship and a mission have.
local function name_of(peer)
	local st = current_state()
	local ok, name = B.pcall(function()
		for _, p in B.pairs(st and st.players or {}) do
			local go = B.type(p) == "table" and p.player_go
			if go and go:exists() and go:owner() == peer then return go:get("name") end
		end
	end)
	if ok and B.type(name) == "string" and name ~= "" then return name end
	return M.core.text("expansion_unknown_player")
end

local function title(id)
	local state = M.core.mods_by_id[id]
	return state and M.core.name_of(state) or id
end

local function set_room_hash(l, hash)
	if not (l and B.type(hash) == "string") then return end
	local ok, err = B.pcall(l.set_data, l, "build_game_hash", hash)
	if ok then
		log.line("expansion: lobby pool " .. hash)
	else
		log.limited("expansion pool", "expansion: the lobby pool wasn't set: " .. B.tostring(err))
	end
end

local function deny(st, peer, rpc, reason)
	local ok, err = B.pcall(function() st.network_router:transmit_to(peer, rpc, reason) end)
	if not ok then log.limited("expansion deny", "expansion: couldn't answer " .. rpc .. ": " .. B.tostring(err)) end
end

local function turned_away(peer, name)
	local run = M.run
	if not run or M.turned[peer] then return end
	M.turned[peer] = true
	local who = B.type(name) == "string" and name ~= "" and name or name_of(peer)
	log.line("expansion: turned away a player who doesn't run " .. run.id)
	M.core.notice("expansion_turned_away", who, title(run.id))
end

-- A held join request goes to the state the host is in now, which may have moved on since it came.
-- With the content off, or the joiner running it, the game's own checks decide.
local function answer(h, accept)
	local st = current_state() or h.state
	if accept or not M.run then
		local handler = B.type(st) == "table" and st.rpc_on_join_request
		if B.type(handler) ~= "function" then return end
		local ok, err = B.pcall(handler, st, h.peer, B.unpack(h.args, 1, h.args.n))
		if not ok then log.limited("expansion join", "expansion: a held join request failed: " .. B.tostring(err)) end
		return
	end
	deny(st, h.peer, "rpc_on_join_denied", JOIN_DENIED)
	turned_away(h.peer, h.name)
end

local function release_held()
	local list = M.held
	M.held = {}
	for _, h in B.ipairs(list) do answer(h, true) end
end

local function changed()
	M.session.refresh()
end

local function begin(id, host, l)
	M.run = { id = id, host = host, lobby = l and l.lobby or false }
	M.taint, M.last, M.strikes, M.turned = true, id, {}, {}
	log.line("expansion: " .. id .. " content on, " .. (host and "as the host" or "following the host"))
	if host and l then set_room_hash(l, pool_of(id)) end
	changed()
end

local function finish(why, l)
	local run = M.run
	if not run then return end
	M.run = nil
	log.line("expansion: " .. run.id .. " content off, " .. why)
	if run.host and l and l.lobby == run.lobby then set_room_hash(l, global("BUILD_GAME_HASH")) end
	M.strikes = {}
	release_held()
	changed()
end

-- The host's join requests, in the ship, while the mission loads and in the mission. The joiner is in
-- the Steam lobby already, so its member data can be read, and nothing is reserved yet.
local function on_join_request(original, self, peer, game_hash, ...)
	local run, l = M.run, M.session.lobby()
	if not (run and run.host and l) then return original(self, peer, game_hash, ...) end
	local yes = runs(l, peer, pair_of(run.id))
	if yes == nil then
		-- Member data comes a moment after a player enters the lobby, so the request waits for it.
		for _, h in B.ipairs(M.held) do
			if h.peer == peer then
				h.state, h.args = self, pack(game_hash, ...)
				return
			end
		end
		B.insert(M.held, { state = self, peer = peer, name = (B.select(2, ...)), args = pack(game_hash, ...), since = M.now() })
		log.line("expansion: a join request waits for the joiner's member data")
		return
	end
	if not yes then
		deny(self, peer, "rpc_on_join_denied", JOIN_DENIED)
		turned_away(peer, (B.select(2, ...)))
		return
	end
	-- A player who runs the expansion may come from another pool of the same build.
	if B.type(game_hash) == "string" and M.base and B.sub(game_hash, 1, #M.base) == M.base then
		game_hash = global("BUILD_GAME_HASH") or game_hash
	end
	return original(self, peer, game_hash, ...)
end

-- The second door: the joiner has loaded the level and asks to be added to the game session.
local function on_session_request(original, self, peer, ...)
	local run, l = M.run, M.session.lobby()
	if run and run.host and l and runs(l, peer, pair_of(run.id)) ~= true then
		deny(self, peer, "rpc_on_join_session_denied", SESSION_DENIED)
		-- The slot the join request reserved would stay taken for 90 seconds.
		B.pcall(function()
			for _, p in B.pairs(self.players or {}) do
				if B.type(p) == "table" and B.type(p.reserved_info) == "table" and p.reserved_info.peer_id == peer then p.reserved_info = nil end
			end
		end)
		turned_away(peer, nil)
		return
	end
	return original(self, peer, ...)
end

-- A player in the mission who doesn't run the content got past both doors somehow. The game's kick
-- goes to that player alone, and the mission's own timer takes them out of the session 5 seconds
-- later. They aren't added to the game's list of kicked players, so they can come back afterwards.
local function remove_strays(l, st)
	local handler = st and st.network_session_handler
	if not (handler and B.type(st.kicked_peer_ids) == "table") then return end
	local me, pair = l.my_id, pair_of(M.run.id)
	for _, peer in B.ipairs(l:members() or {}) do
		if peer ~= me then
			if handler:is_peer_in_session(peer) and runs(l, peer, pair) ~= true then
				M.strikes[peer] = (M.strikes[peer] or 0) + 1
				if M.strikes[peer] == STRIKES then
					st.network_router:transmit_to(peer, "rpc_on_kicked", peer)
					B.insert(st.kicked_peer_ids, { peer_id = peer, timer = 5 })
					log.line("expansion: removed a player in the mission who doesn't run " .. M.run.id)
					turned_away(peer, nil)
				end
			else
				M.strikes[peer] = nil
			end
		end
	end
end

function M.evaluate()
	local session = M.session
	local l = session.lobby()
	M.checks = M.checks + 1
	if not l then
		if M.run and M.run.lobby then finish("left the lobby") end
		M.host = nil
		return
	end
	if M.run and M.run.lobby ~= l.lobby then finish("left the lobby", l) end
	local mine = B.next(M.mine) ~= nil
	if mine then session.publish(l, LIST_KEY, own_pairs()) end
	local host, me = l:lobby_host(), l.my_id
	-- The grace for a new host starts when the host changes, not at the first one seen.
	if host ~= M.host then
		M.host_since = M.host ~= nil and M.checks or -HANDOVER
		M.host = host
	end
	if host == me then
		local run = M.run
		if run and not run.host then
			-- The host left and this game took over the lobby: the content carries on from here.
			run.host = true
			log.line("expansion: " .. run.id .. " carries on with this game as the host")
			set_room_hash(l, pool_of(run.id))
		end
		if mine then session.publish(l, RUN_KEY, run and pair_of(run.id) or "") end
		if run then
			local ok, err = B.pcall(remove_strays, l, current_state())
			if not ok then log.limited("expansion strays", "expansion: check of the players in the mission failed: " .. B.tostring(err)) end
		end
		return
	end
	if mine then session.publish(l, RUN_KEY, "") end
	local data = l:member_data(host, RUN_KEY)
	local id, hash
	if B.type(data) == "string" then id, hash = B.match(data, "^([^:]+):(%x+)$") end
	if id and M.mine[id] == hash then
		if M.run and M.run.id ~= id then finish("the host started other content", l) end
		if not M.run then begin(id, false, l) end
		return
	end
	if M.run and not id and M.checks - M.host_since < HANDOVER then return end
	if M.run then finish("the host stopped it", l) end
	if id and not M.told[data] then
		M.told[data] = true
		log.line("expansion: the host runs " .. id .. ", which this deploy doesn't have at the same version")
		M.core.notice("expansion_missing_here", id)
	end
end

function M.tick()
	if #M.held > 0 then
		local ok, err = B.pcall(function()
			local l, run = M.session.lobby(), M.run
			local list = M.held
			M.held = {}
			for _, h in B.ipairs(list) do
				local yes = (run and l) and runs(l, h.peer, pair_of(run.id))
				if not run then
					answer(h, true)
				elseif yes ~= nil then
					answer(h, yes)
				elseif M.now() - h.since >= HOLD_SECONDS then
					answer(h, false)
				else
					B.insert(M.held, h)
				end
			end
		end)
		if not ok then log.limited("expansion held", "expansion: held join requests failed: " .. B.tostring(err)) end
	end
	if M.core.frame_count % EVERY ~= 0 then return end
	local ok, err = B.pcall(M.evaluate)
	if not ok then log.limited("expansion", "expansion check failed: " .. B.tostring(err)) end
end

-- A mission starts. Once content stops, the results it left wait in the save until the ship sends
-- them, so the block lasts until a mission starts without it. Answers whether the block just ended.
function M.mission_started()
	if M.run then
		M.taint = true
		return false
	end
	local was = M.taint
	M.taint = false
	return was
end

function M.setup(core, hooks, session)
	M.core, M.session = core, session
	reset()
	local deploy = core.deploy
	M.mine = B.type(deploy.expansions) == "table" and deploy.expansions or {}
	M.tags = B.type(deploy.hashes) == "table" and B.type(deploy.hashes.expansions) == "table" and deploy.hashes.expansions or {}
	M.base = B.match(B.tostring(deploy.revision), ":(%x+)$")
	M.pools = {}
	for id in B.pairs(M.mine) do
		local hash = pool_of(id)
		if hash and B.match(M.tags[id], "^[0-9a-f]+$") then M.pools[hash] = id end
	end
	-- A player without expansions still hears when a lobby's host runs one.
	hooks.on_frame(M.tick)
	if B.next(M.mine) == nil then return end
	log.line("expansions: " .. own_pairs())
	for _, class in B.ipairs({ "StateMenu", "StateMissionStart", "StateGame" }) do
		hooks.loader_hook(class, "rpc_on_join_request", on_join_request)
	end
	for _, class in B.ipairs({ "StateMenu", "StateMissionStart", "StateGame", "StateMissionEnd" }) do
		hooks.loader_hook(class, "rpc_on_join_session_request", on_session_request)
	end
	-- The game writes the room hash again on its own, after a host change or a state change.
	hooks.loader_hook("NetworkRoom", "set_data", function(original, self, key, value, ...)
		local run = M.run
		if key == "build_game_hash" and run and run.host then value = pool_of(run.id) or value end
		return original(self, key, value, ...)
	end)
	-- The lobby browser lists rooms with this game's own hash only. A room playing an expansion this
	-- deploy runs, at the same version, counts as one of them.
	hooks.loader_hook("MenuScreenCampaignMultiplayer", "add_room", function(original, self, room, ...)
		if B.type(room) == "table" then
			local id = M.pools[room.build_game_hash]
			local own = global("BUILD_GAME_HASH")
			if id and own then
				room.build_game_hash = own
				room.obsidian_expansion = id
			end
		end
		return original(self, room, ...)
	end)
end

-- obsidian.expansion, for the calling mod.
local function calling()
	local state = M.core.current
	if state and M.is_own(state.id) then return state end
	return nil
end

function M.start()
	local state = calling()
	if not state then return false, "not_expansion" end
	local id = state.id
	if M.run then
		if M.run.id == id then return true end
		return false, "busy"
	end
	local l = M.session.lobby()
	if not l then
		begin(id, true, nil)
		return true
	end
	local ok, host, missing = B.pcall(function()
		if l:lobby_host() ~= l.my_id then return false end
		local pair, out = pair_of(id), {}
		for _, peer in B.ipairs(l:members() or {}) do
			if peer ~= l.my_id and runs(l, peer, pair) ~= true then B.insert(out, name_of(peer)) end
		end
		return true, out
	end)
	if not ok then
		log.limited("expansion start", "expansion: couldn't read the lobby: " .. B.tostring(host))
		return false, "not_available"
	end
	if not host then return false, "not_host" end
	if #missing > 0 then
		log.line("expansion: " .. id .. " didn't start, " .. #missing .. " player(s) here don't run it at this version")
		M.core.notice("expansion_refused", title(id), B.concat(missing, ", "))
		return false, "players_missing", missing
	end
	begin(id, true, l)
	M.session.publish(l, RUN_KEY, pair_of(id))
	return true
end

function M.stop()
	local state = calling()
	if not state then return false, "not_expansion" end
	local run = M.run
	if not run or run.id ~= state.id then return false, "not_running" end
	if run.lobby and not run.host then return false, "not_host" end
	local l = M.session.lobby()
	finish("stopped", l)
	if l then M.session.publish(l, RUN_KEY, "") end
	return true
end

-- Whether the content of id runs on this machine: the calling mod's when id is nil.
function M.running(id)
	if id == nil then id = M.core.current and M.core.current.id end
	return M.run ~= nil and M.run.id == id
end

function M.api()
	return { start = M.start, stop = M.stop, running = M.running }
end

return M
