-- Matchmaking separation (spec 6.3, layer 1). build_info reads the game hash out of
-- BUILD_REVISION_INFORMATION after this chunk runs, so appending a tag to the hash there moves
-- the player into a pool that vanilla players never see.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = {}

-- Returns the tag that was applied, or nil for the vanilla pool. Players who run session content
-- share one pool, so they only find other Obsidian players. A player who merely lets other players'
-- session mods in stays in the vanilla pool, where vanilla players find their lobby, and sees the
-- open pool's lobbies too (bridge). Players who run mods every player needs get a pool of their
-- own, shared only with players who run the same ones.
function M.apply(deploy)
	local hashes = deploy.hashes or {}
	local tag, pool
	if hashes.every then
		tag, pool = hashes.every, "every"
	elseif deploy.has_session then
		tag, pool = hashes.open, "open"
	else
		log.line("pool: vanilla")
		return nil
	end
	if B.type(tag) ~= "string" or not B.match(tag, "^[0-9a-f]+$") then
		log.line("pool: bad tag for " .. pool .. ", left vanilla")
		return nil
	end
	local revision = B.rawget(B.G, "BUILD_REVISION_INFORMATION")
	local rewritten, n = B.gsub(revision, "(parent: [0-9]+:)([0-9a-f]+)", function(head, hash) return head .. hash .. tag end, 1)
	if n ~= 1 then
		log.line("pool: no hash in the revision string, left vanilla")
		return nil
	end
	B.rawset(B.G, "BUILD_REVISION_INFORMATION", rewritten)
	log.line("pool: " .. pool .. " " .. tag)
	return tag
end

-- The room data of the lobby `l` (NetworkGlobal.lobby): the string its host's NetworkRoom writes
-- under the lobby key "data", which every member can read, as a table by the names in
-- GameSettings.room_data_values. Nil when it can't be read. NetworkRoom keeps a table in a field
-- named data, which hides the lobby's data method, so this asks the engine's lobby inside it.
function M.room(l)
	local engine = B.type(l) == "table" and l.lobby or nil
	if engine == nil then return nil end
	local ok, raw = B.pcall(function() return engine:data("data") end)
	local network, settings = B.rawget(B.G, "NetworkGlobal"), B.rawget(B.G, "GameSettings")
	if not ok or B.type(raw) ~= "string" or B.type(network) ~= "table" or B.type(settings) ~= "table" then return nil end
	local parsed, t = B.pcall(network.data_string_to_table, raw, settings.room_data_values)
	return parsed and B.type(t) == "table" and t or nil
end

-- Whether the lobby being joined, not hosted, carries the hash `other`: a room the browser listed
-- from that pool, at the address the join screen keeps, or else (an invite) the room data.
local function joining(other, screen, listed)
	local ok, yes = B.pcall(function()
		local network = B.rawget(B.G, "NetworkGlobal")
		local l = network and network.lobby
		if not (l and l.lobby) or l:is_host() then return false end
		if B.type(screen) == "table" and screen.join_address ~= nil and listed[B.tostring(screen.join_address)] then return true end
		local data = M.room(l)
		return data ~= nil and data.build_game_hash == other
	end)
	if not ok then log.limited("pool join", "pool: the lobby being joined couldn't be read: " .. B.tostring(yes)) end
	return ok and yes == true
end

-- A player in the vanilla pool who runs no session content but lets other players' session mods in
-- (the default) also finds the open pool's lobbies: the lobby browser lists them beside the vanilla
-- ones, and a join request to one carries the open pool's hash, the one its host checks. Their own
-- lobby stays in the vanilla pool, where vanilla players find it. Answers whether it applies.
function M.bridge(core, hooks, tag)
	local deploy = core.deploy
	if tag ~= nil or deploy.has_session or not core.consent() then return false end
	local open = (deploy.hashes or {}).open
	local base = B.match(B.tostring(deploy.revision), ":(%x+)$")
	if not base or B.type(open) ~= "string" or not B.match(open, "^[0-9a-f]+$") then return false end
	local other = base .. open
	-- Addresses of the open pool's rooms the browser listed, as strings.
	local listed, told = {}, nil
	hooks.loader_hook("MenuScreenCampaignMultiplayer", "add_room", function(original, self, room, ...)
		local own = B.rawget(B.G, "BUILD_GAME_HASH")
		if B.type(room) == "table" and room.build_game_hash == other and B.type(own) == "string" then
			room.build_game_hash = own
			if room.address ~= nil then listed[B.tostring(room.address)] = true end
		end
		return original(self, room, ...)
	end)
	hooks.loader_hook("MenuScreenCampaignMultiplayer", "update_join", function(original, self, ...)
		local own = B.rawget(B.G, "BUILD_GAME_HASH")
		if B.type(own) ~= "string" or not joining(other, self, listed) then return original(self, ...) end
		local address = B.type(self) == "table" and B.tostring(self.join_address) or "?"
		if told ~= address then
			told = address
			log.line("pool: joining an Obsidian lobby, with the open pool's hash")
		end
		B.rawset(B.G, "BUILD_GAME_HASH", other)
		local r = { n = 0 }
		local function keep(...)
			r = { n = B.select("#", ...), ... }
		end
		keep(B.pcall(original, self, ...))
		B.rawset(B.G, "BUILD_GAME_HASH", own)
		if not r[1] then B.error(r[2], 0) end
		return B.unpack(r, 2, r.n)
	end)
	log.line("pool: open pool lobbies listed too")
	return true
end

return M
