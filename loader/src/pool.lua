-- Matchmaking separation (spec 6.3, layer 1). build_info reads the game hash out of
-- BUILD_REVISION_INFORMATION after this chunk runs, so appending a tag to the hash there moves
-- the player into a pool that vanilla players never see.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = {}

-- Returns the tag that was applied, or nil for the vanilla pool. Players who run session content
-- or allow other players' session mods share one pool, so a consenting player's lobby shows up for
-- players with session mods, and players with session mods only find other Obsidian players.
-- Players who run mods every player needs get a pool of their own, shared only with players who
-- run the same ones.
function M.apply(deploy, consent_on)
	local hashes = deploy.hashes or {}
	local tag, pool
	if hashes.every then
		tag, pool = hashes.every, "every"
	elseif deploy.has_session or consent_on then
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

-- Whether the lobby being joined, not hosted, carries the vanilla hash: a room the browser listed as
-- vanilla, at the address the join screen keeps, or else (an invite) the room data of the lobby,
-- the string its host's NetworkRoom writes under the lobby key "data".
local seen = { raw = nil, vanilla = false }
local function joining_vanilla(base, screen, listed)
	local ok, yes = B.pcall(function()
		local network = B.rawget(B.G, "NetworkGlobal")
		local l = network and network.lobby
		if not (l and l.lobby) or l:is_host() then return false end
		if B.type(screen) == "table" and screen.join_address ~= nil and listed[B.tostring(screen.join_address)] then return true end
		local raw = l:data("data")
		if raw ~= seen.raw then
			local data = network.data_string_to_table(raw, B.rawget(B.G, "GameSettings").room_data_values)
			seen.raw, seen.vanilla = raw, B.type(data) == "table" and data.build_game_hash == base
		end
		return seen.vanilla
	end)
	if not ok then log.limited("pool join", "pool: the lobby being joined couldn't be read: " .. B.tostring(yes)) end
	return ok and yes == true
end

-- A player in the open pool for consent alone runs no session content, so a vanilla lobby is as safe
-- for them as for a vanilla player. The lobby browser lists vanilla rooms beside the pool's, and a
-- join request to a vanilla lobby carries the vanilla hash, the one its host checks. Their own lobby
-- stays in the open pool, where players with session mods find it. Answers whether it applies.
function M.bridge(core, hooks, tag)
	local deploy = core.deploy
	if tag == nil or tag ~= (deploy.hashes or {}).open or deploy.has_session or not core.consent() then return false end
	local base = B.match(B.tostring(deploy.revision), ":(%x+)$")
	if not base then return false end
	-- Addresses of the vanilla rooms the browser listed, as strings.
	local listed, told = {}, nil
	hooks.loader_hook("MenuScreenCampaignMultiplayer", "add_room", function(original, self, room, ...)
		local own = B.rawget(B.G, "BUILD_GAME_HASH")
		if B.type(room) == "table" and room.build_game_hash == base and B.type(own) == "string" then
			room.build_game_hash = own
			if room.address ~= nil then listed[B.tostring(room.address)] = true end
		end
		return original(self, room, ...)
	end)
	hooks.loader_hook("MenuScreenCampaignMultiplayer", "update_join", function(original, self, ...)
		local own = B.rawget(B.G, "BUILD_GAME_HASH")
		if B.type(own) ~= "string" or not joining_vanilla(base, self, listed) then return original(self, ...) end
		local address = B.type(self) == "table" and B.tostring(self.join_address) or "?"
		if told ~= address then
			told = address
			log.line("pool: joining a vanilla lobby, with the vanilla hash")
		end
		B.rawset(B.G, "BUILD_GAME_HASH", base)
		local r = { n = 0 }
		local function keep(...)
			r = { n = B.select("#", ...), ... }
		end
		keep(B.pcall(original, self, ...))
		B.rawset(B.G, "BUILD_GAME_HASH", own)
		if not r[1] then B.error(r[2], 0) end
		return B.unpack(r, 2, r.n)
	end)
	log.line("pool: vanilla lobbies listed too, consent alone")
	return true
end

return M
