-- Matchmaking separation (spec 6.3, layer 1). build_info reads the game hash out of
-- BUILD_REVISION_INFORMATION after this chunk runs, so appending a tag to the hash there moves
-- the player into a pool that vanilla players never see.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = {}

-- Every player stays in the vanilla matchmaking pool. Session mods switch off live where the session
-- gate says so, mods every player needs included, so splitting players up only kept them apart. The
-- public door below covers content that can't switch off. Answers nil, the vanilla pool's tag.
function M.apply(deploy)
	log.line("pool: vanilla")
	return nil
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

-- Whether a lobby privacy value is public. One that can't be read counts as public.
local function is_public(privacy)
	local settings = B.rawget(B.G, "GameSettings")
	local kinds = B.type(settings) == "table" and settings.room_privacy_settings or nil
	local n = B.tonumber(privacy)
	if n == nil or B.type(kinds) ~= "table" then return true end
	return n == kinds.public
end
M.is_public = is_public

-- The hash a join request to the lobby being joined, not hosted, carries: `other` (the open pool's)
-- for a room in the open pool, by the room picked in the list or else (an invite) the room data,
-- and for any public room when this player runs session content that can't switch off live. Nil
-- leaves the game's own hash.
local function join_hash(other, screen, listed, static)
	local ok, hash = B.pcall(function()
		local network = B.rawget(B.G, "NetworkGlobal")
		local l = network and network.lobby
		if not (l and l.lobby) or l:is_host() then return nil end
		local entry = B.type(screen) == "table" and screen.join_address ~= nil and listed[B.tostring(screen.join_address)] or nil
		if entry and entry.open then return other end
		local data = M.room(l)
		if data ~= nil and data.build_game_hash == other then return other end
		if static then
			local privacy = data and data.privacy
			if privacy == nil and entry then privacy = entry.privacy end
			if is_public(privacy) then return other end
		end
		return nil
	end)
	if not ok then log.limited("pool join", "pool: the lobby being joined couldn't be read: " .. B.tostring(hash)) end
	return ok and hash or nil
end

local function in_public_lobby_as_host()
	local ok, yes = B.pcall(function()
		local network = B.rawget(B.G, "NetworkGlobal")
		local l = network and network.lobby
		if not (l and l:is_host()) then return false end
		return is_public(l:get_data("privacy"))
	end)
	return not ok or yes == true
end

-- Every player in the vanilla pool finds the open pool's lobbies: Obsidian 1.3.0 put players with
-- session mods there, and a public lobby whose host runs content that can't switch off live carries
-- its hash now. The lobby browser lists them beside the vanilla ones, and a join request to one
-- carries the open pool's hash, the one its room shows. As the host, a join request with that hash
-- counts as the game's own.
-- The public door: a player whose session content can't switch off live (data overrides, legacy
-- mods) keeps vanilla players out of their public lobby, whose room shows the open pool's hash, and
-- doesn't take that content into another public lobby. Private and friends-only lobbies are open to
-- anyone. Answers whether the bridge applies.
function M.bridge(core, hooks, tag)
	local deploy = core.deploy
	if tag ~= nil then return false end
	local open = (deploy.hashes or {}).open
	local base = B.match(B.tostring(deploy.revision), ":(%x+)$")
	if not base or B.type(open) ~= "string" or not B.match(open, "^[0-9a-f]+$") then return false end
	local other = base .. open
	local static = deploy.has_static_session == true
	-- The rooms the browser listed, by address as a string: whether in the open pool, and privacy.
	local listed, told = {}, nil
	hooks.loader_hook("MenuScreenCampaignMultiplayer", "add_room", function(original, self, room, ...)
		local own = B.rawget(B.G, "BUILD_GAME_HASH")
		if B.type(room) == "table" and B.type(own) == "string" then
			local is_open = room.build_game_hash == other
			if is_open then room.build_game_hash = own end
			if room.address ~= nil then listed[B.tostring(room.address)] = { open = is_open, privacy = room.privacy } end
		end
		return original(self, room, ...)
	end)
	hooks.loader_hook("MenuScreenCampaignMultiplayer", "update_join", function(original, self, ...)
		local own = B.rawget(B.G, "BUILD_GAME_HASH")
		local hash = B.type(own) == "string" and join_hash(other, self, listed, static) or nil
		if hash == nil then return original(self, ...) end
		local address = B.type(self) == "table" and B.tostring(self.join_address) or "?"
		if told ~= address then
			told = address
			log.line("pool: joining with the open pool's hash")
		end
		B.rawset(B.G, "BUILD_GAME_HASH", hash)
		local r = { n = 0 }
		local function keep(...)
			r = { n = B.select("#", ...), ... }
		end
		keep(B.pcall(original, self, ...))
		B.rawset(B.G, "BUILD_GAME_HASH", own)
		if not r[1] then B.error(r[2], 0) end
		return B.unpack(r, 2, r.n)
	end)
	local function on_join_request(original, self, peer, game_hash, ...)
		local own = B.rawget(B.G, "BUILD_GAME_HASH")
		local public = static and in_public_lobby_as_host()
		if game_hash == other and B.type(own) == "string" then
			game_hash = own
		elseif public and game_hash == own then
			-- The game turns it away as another version.
			game_hash = B.tostring(own) .. "-public"
			log.line("pool: turned away a player without Obsidian from a public lobby")
		end
		return original(self, peer, game_hash, ...)
	end
	for _, class in B.ipairs({ "StateMenu", "StateMissionStart", "StateGame", "StateMissionEnd" }) do
		hooks.loader_hook(class, "rpc_on_join_request", on_join_request)
	end
	if static then
		-- The game writes the room hash before the privacy, and the privacy again when it changes.
		hooks.loader_hook("NetworkRoom", "set_data", function(original, self, key, value, ...)
			local own = B.rawget(B.G, "BUILD_GAME_HASH")
			local function privacy()
				local ok, v = B.pcall(self.get_data, self, "privacy")
				return ok and v or nil
			end
			if key == "build_game_hash" and value == own and is_public(privacy()) then value = other end
			-- NetworkRoom.set_data answers nothing.
			original(self, key, value, ...)
			if key == "privacy" then
				local ok, cur = B.pcall(self.get_data, self, "build_game_hash")
				local want = is_public(value) and other or own
				if ok and (cur == own or cur == other) and cur ~= want then
					original(self, "build_game_hash", want)
					log.line("pool: lobby " .. (want == other and "public, open pool hash" or "not public, game hash"))
				end
			end
			return
		end)
	end
	log.line("pool: open pool lobbies listed too" .. (static and ", public door on" or ""))
	return true
end

return M
