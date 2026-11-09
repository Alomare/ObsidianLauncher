-- Lobby info, for every loader user, Sandbox or not: the host publishes what the lobby runs under the
-- Steam lobby key "obsidian_lobby" = "1;<flags>;<name@version|name@version|+N>", and the multiplayer
-- terminal reads it before joining. Flags: "s", the host is on a Sandbox save. This list is only
-- information: joins are refused by the build hash alone (pool.lua).
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")
local game_text = require("obsidian/loader/game_text")
local pool = require("obsidian/loader/pool")

local KEY = "obsidian_lobby"
local EVERY = 30
-- The Steam limit is larger; a value this size is already far more than the panel shows.
local MAX_VALUE = 4000
local MAX_READ = 4096
local MAX_ITEMS = 64
local MAX_TEXT = 48
local SHOWN = 8

local FILTERS = {
	{ name = "filter_modded_only", text = "lobby_filter_modded_only", hide = function(room) return not (room.obsidian and room.obsidian.modded) end },
	{ name = "filter_hide_modded", text = "lobby_filter_hide_modded", hide = function(room) return room.obsidian ~= nil and room.obsidian.modded end },
}

local M = { sent_to = nil, sent_value = nil }

-- Text going into the lobby key: no separators or control characters.
local function clean(text)
	return B.sub((B.gsub(B.tostring(text), "[%c;|@]", "")), 1, MAX_TEXT)
end

-- Text read from another machine's lobby: printable ASCII only, whatever the host put there.
local function clean_read(text)
	return B.sub((B.gsub(text, "[^\32-\126]", "")), 1, MAX_TEXT)
end

-- A mod as "name@version": English name, or the id when the mod has none.
local function item(state)
	local names = B.type(state.mod.name) == "table" and state.mod.name or {}
	local name = clean(B.type(names.en) == "string" and names.en or state.id)
	if name == "" then name = clean(state.id) end
	if name == "" then return nil end
	return name .. "@" .. clean(state.mod.version or "?")
end

-- Everything a lobby's players see: session and expansion mods always, client mods unless the
-- guardrail hides them. Failed mods aren't running, so they aren't listed.
local function listed(core)
	local out = {}
	for _, state in B.ipairs(core.mods) do
		if not state.off and not (state.mod.scope == "client" and core.hide_client_mods) then
			local text = item(state)
			if text then B.insert(out, text) end
		end
	end
	B.sort(out)
	return out
end

-- The value of the lobby key.
function M.value(core)
	local items = listed(core)
	local head = "1;" .. (core.sandboxed() and "s" or "") .. ";"
	local out, used = {}, #head
	for i, text in B.ipairs(items) do
		local rest = #items - i
		-- Room for this item, and for "+N" after it if more follow.
		if used + #text + 1 + (rest > 0 and 12 or 0) > MAX_VALUE then
			B.insert(out, "+" .. (#items - i + 1))
			break
		end
		B.insert(out, text)
		used = used + #text + 1
	end
	return head .. B.concat(out, "|")
end

function M.parse(value)
	if B.type(value) ~= "string" then return nil end
	local flags, list = B.match(B.sub(value, 1, MAX_READ), "^1;([^;]*);(.*)$")
	if not flags then return nil end
	local info = { sandbox = B.find(flags, "s", 1, true) ~= nil, mods = {}, more = 0 }
	for raw in B.gmatch(list, "[^|]+") do
		local extra = B.match(raw, "^%+(%d+)$")
		if extra then
			info.more = info.more + (B.tonumber(extra) or 0)
		elseif #info.mods >= MAX_ITEMS then
			info.more = info.more + 1
		else
			local name, version = B.match(raw, "^(.*)@([^@]*)$")
			name = clean_read(name or raw)
			version = version and clean_read(version) or ""
			if name ~= "" then B.insert(info.mods, version ~= "" and (name .. " " .. version) or name) end
		end
	end
	info.modded = #info.mods > 0 or info.more > 0
	return info
end

-- The host keeps the key current, and a player who becomes host after a migration takes it over: what
-- the lobby holds is read back, so another host's value is replaced.
local function publish(l)
	if not l:is_host() then return end
	local engine = l.lobby
	local value = M.value(M.core)
	local ok, current = B.pcall(function() return engine:data(KEY) end)
	if ok and current == value then
		M.sent_to, M.sent_value = engine, value
		return
	end
	-- A key that can't be read back isn't written again until it changes.
	if M.sent_to == engine and M.sent_value == value and (not ok or current == nil or current == "") then return end
	-- Steam writes through the backend's table, as NetworkRoom does. Anywhere else there's no such key.
	local backend = B.rawget(B.G, "SteamLobby") or B.rawget(B.G, "LanLobby")
	if B.type(backend) ~= "table" or B.type(backend.set_data) ~= "function" then return end
	backend.set_data(engine, KEY, value)
	M.sent_to, M.sent_value = engine, value
	log.line("lobby: published the lobby info")
end

function M.tick()
	if M.core.frame_count % EVERY ~= 0 then return end
	local ok, err = B.pcall(function()
		local l = M.session.lobby()
		if l then publish(l) end
	end)
	if not ok then log.limited("lobby info", "lobby: could not publish: " .. B.tostring(err)) end
end

-- Whether a join to the lobby this player just joined was refused because of the Sandbox line. Answers
-- the text key, or nil for the game's own text.
function M.refused_text(l)
	local core = M.core
	local base = B.match(B.tostring(core.deploy.revision), ":(%x+)$")
	local room = pool.room(l)
	-- A room on a really different game version keeps the game's words.
	if room and B.type(room.build_game_hash) == "string" and base and B.sub(room.build_game_hash, 1, #base) ~= base then return nil end
	local ok, value = B.pcall(function() return l.lobby:data(KEY) end)
	local info = M.parse(ok and value or nil)
	local theirs = info ~= nil and info.sandbox
	local own = core.sandboxed()
	if own and not theirs then return "lobby_uses_official" end
	if theirs and not own then return "lobby_uses_sandbox" end
	return nil
end

-- The filters are added to the game's list when the terminal opens: the game's settings file isn't
-- loaded when the loader starts.
local function add_filters()
	local settings = B.rawget(B.G, "GameSettings")
	local filters = B.type(settings) == "table" and settings.multiplayer_filters or nil
	if B.type(filters) ~= "table" then return end
	for _, f in B.ipairs(FILTERS) do
		local have = false
		for _, g in B.ipairs(filters) do have = have or g.name == f.name end
		if not have then
			B.insert(filters, { name = f.name, default_value = false, filter_func = function(room, on)
				return on and f.hide(room)
			end })
		end
	end
end

-- The list of mods over the selected room's preview. The game's layout for this panel is still open.
local function draw_mods(self, gui, z)
	local room = B.type(self.displayable_rooms) == "table" and self.displayable_rooms[self.selected_room]
	local info = B.type(room) == "table" and room.obsidian
	if not (info and info.modded) then return end
	local fonts = B.rawget(B.G, "FontSettings")
	local font = B.type(fonts) == "table" and fonts.fonts and fonts.fonts.body
	local vec3, vec2, color = B.rawget(B.G, "Vector3"), B.rawget(B.G, "Vector2"), B.rawget(B.G, "Color")
	if not (font and vec3 and vec2 and color) then return end
	local lines = { M.core.text("lobby_mods_heading") }
	for i = 1, B.min(#info.mods, SHOWN) do B.insert(lines, B.sub(info.mods[i], 1, 34)) end
	local extra = info.more + B.max(0, #info.mods - SHOWN)
	if extra > 0 then B.insert(lines, M.core.text("lobby_more_mods", extra)) end
	local top, step = 402, font.line_height or (font.font_size + 2)
	gui:draw_rect(vec3(218, top + step - (step * #lines), z + 3), vec2(180, step * #lines + 4), color(180, 0, 0, 0))
	gui:begin_template({ to_upper_case = false, color = color(255, 255, 255, 255), font_name = font.class, font_size = font.font_size, anchor = vec2(0, 0) })
	-- Whatever goes wrong drawing, the template has to close or the game's next text is lost.
	local ok, err = B.pcall(function()
		for i, line in B.ipairs(lines) do
			gui:draw_text(line, vec3(222, top - step * (i - 1), z + 4))
		end
	end)
	gui:end_template()
	if not ok then log.limited("lobby list", "lobby: could not draw the mod list: " .. B.tostring(err)) end
end

function M.setup(core, hooks, session)
	M.core, M.session = core, session
	hooks.on_frame(M.tick)
	for _, f in B.ipairs(FILTERS) do game_text.replace(core, hooks, f.name, f.text) end
	if core.sandboxed() then game_text.replace(core, hooks, "campaign_public", "lobby_sandbox_public") end
	local class = "MenuScreenCampaignMultiplayer"
	hooks.loader_hook(class, "on_enter", function(original, self, ...)
		B.pcall(add_filters)
		return original(self, ...)
	end)
	hooks.loader_hook(class, "extract_room_info", function(original, self, ...)
		local room = original(self, ...)
		if B.type(room) == "table" then
			local ok, value = B.pcall(function() return self.room_browser:data(self.current_room_index, KEY) end)
			room.obsidian = M.parse(ok and value or nil)
		end
		return room
	end)
	-- The row's second line, which the game draws for the player count of each room.
	hooks.loader_hook(class, "draw_looping_scroll", function(original, self, gui, z, pos, id, text, ...)
		local row = B.type(id) == "string" and B.tonumber(B.match(id, "^sub(%d+)$"))
		local room = row and B.type(self.displayable_rooms) == "table" and self.displayable_rooms[row]
		if B.type(room) == "table" and room.obsidian and room.obsidian.modded then
			text = B.tostring(text) .. "  " .. core.text("lobby_modded_mark")
		end
		return original(self, gui, z, pos, id, text, ...)
	end)
	hooks.loader_hook(class, "draw_selected_room_info", function(original, self, gui, dt, z, ...)
		original(self, gui, dt, z, ...)
		local ok, err = B.pcall(draw_mods, self, gui, z)
		if not ok then log.limited("lobby list", "lobby: could not draw the mod list: " .. B.tostring(err)) end
	end)
	hooks.loader_hook(class, "rpc_on_join_denied", function(original, self, sender, reason, ...)
		original(self, sender, reason, ...)
		if reason ~= "network_join_denied_wrong_version" then return end
		local network = B.rawget(B.G, "NetworkGlobal")
		local l = B.type(network) == "table" and network.lobby or nil
		if not (l and l.lobby) then return end
		local ok, key = B.pcall(M.refused_text, l)
		if ok and key then self.info_message = core.text(key) end
	end)
end

return M
