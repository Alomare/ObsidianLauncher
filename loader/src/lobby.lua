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

-- The Modded tab lists the modded lobbies, so the Public tab keeps one filter: hiding them.
local FILTERS = {
	{ name = "filter_hide_modded", text = "lobby_filter_hide_modded", hide = function(room) return room.obsidian ~= nil and room.obsidian.modded end },
}

-- The terminal's tabs: the game's Public and Friends, with Modded between them. The tab names are
-- text keys; these two are the loader's.
local TAB_PUBLIC, TAB_MODDED = "obsidian_lobby_public", "obsidian_lobby_modded"
-- A pad has no wheel: the list moves on by a line this often instead.
local PAD_STEP = 1.5

local M = { sent_to = nil, sent_value = nil, scroll = { room = nil, offset = 0, timer = 0 } }

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

-- The terminal's tab list with Modded put in after Public, and the Public tab under the loader's name.
-- index: the tab to open, which moves one on past Public. Answers the list and the index; a list that
-- already has the tab, or has no Public tab, comes back as it was.
function M.tabs(list, index)
	if B.type(list) ~= "table" then return list, index end
	local at = nil
	for i, tab in B.ipairs(list) do
		if B.type(tab) ~= "table" then return list, index end
		if tab.name == TAB_MODDED then return list, index end
		if tab.id == "matchmaking" then at = i end
	end
	if not at then return list, index end
	local out = {}
	for i, tab in B.ipairs(list) do
		if i == at then
			local public = {}
			for k, v in B.pairs(tab) do public[k] = v end
			public.name = TAB_PUBLIC
			B.insert(out, public)
			B.insert(out, { id = "modded", name = TAB_MODDED, state = tab.state, params = { filter = "modded" } })
		else
			B.insert(out, tab)
		end
	end
	if B.type(index) == "number" and index > at then index = index + 1 end
	return out, index
end

-- The first line of the mods shown, for `count` lines in a box of `shown`, moved by `by`.
function M.window(count, offset, by, shown)
	local last = B.max(0, count - shown)
	return B.max(0, B.min(last, (offset or 0) + (by or 0)))
end

-- The mod list of a room, the host's "+N" at its end.
local function mod_lines(info)
	local out = {}
	for _, m in B.ipairs(info.mods) do B.insert(out, B.sub(m, 1, 34)) end
	if info.more > 0 then B.insert(out, M.core.text("lobby_more_mods", info.more)) end
	return out
end

-- The list of mods over the selected room's preview, SHOWN lines at a time. The mouse wheel over the
-- list scrolls it (a turn the game's room list didn't take); with a pad it moves on by itself.
local function draw_mods(self, gui, z, dt)
	local room = B.type(self.displayable_rooms) == "table" and self.displayable_rooms[self.selected_room]
	local info = B.type(room) == "table" and room.obsidian
	if not (info and info.modded) then return end
	local fonts = B.rawget(B.G, "FontSettings")
	local font = B.type(fonts) == "table" and fonts.fonts and fonts.fonts.body
	local vec3, vec2, color = B.rawget(B.G, "Vector3"), B.rawget(B.G, "Vector2"), B.rawget(B.G, "Color")
	if not (font and vec3 and vec2 and color) then return end
	local mods = mod_lines(info)
	local scroll = M.scroll
	if scroll.room ~= room then scroll.room, scroll.offset, scroll.timer = room, 0, 0 end
	local lines = { M.core.text("lobby_mods_heading") }
	local shown = B.min(#mods, SHOWN)
	local long = #mods > SHOWN
	local rows = shown + 1 + (long and 1 or 0)
	local top, step = 402, font.line_height or (font.font_size + 2)
	local at, size = vec3(218, top + step - (step * rows), z + 3), vec2(180, step * rows + 4)
	if long then
		local by = 0
		local base = B.rawget(B.G, "MenuScreenBase")
		local wheel = self.mouse_scroll_input
		if wheel and B.type(base) == "table" and B.type(base.is_box_selected_by_cursor) == "function" then
			local ok, over = B.pcall(base.is_box_selected_by_cursor, self, gui, at, size)
			if ok and over then
				by = wheel
				self.mouse_scroll_input = nil
			end
		end
		if self.user_pad_number ~= nil and self.user_pad_number ~= B.rawget(B.G, "KEYBOARD_MOUSE_PAD_NUMBER") then
			scroll.timer = scroll.timer + (B.type(dt) == "number" and dt or 0)
			if scroll.timer >= PAD_STEP then
				scroll.timer = 0
				by = scroll.offset >= #mods - SHOWN and -scroll.offset or 1
			end
		end
		scroll.offset = M.window(#mods, scroll.offset, by, SHOWN)
	end
	for i = scroll.offset + 1, scroll.offset + shown do B.insert(lines, mods[i]) end
	if long then B.insert(lines, M.core.text("lobby_mods_range", scroll.offset + 1, scroll.offset + shown, #mods)) end
	gui:draw_rect(at, size, color(180, 0, 0, 0))
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
	game_text.replace(core, hooks, TAB_PUBLIC, core.sandboxed() and "lobby_sandbox_public" or "lobby_tab_public")
	game_text.replace(core, hooks, TAB_MODDED, "lobby_tab_modded")
	local class = "MenuScreenCampaignMultiplayer"

	-- The tabs: the terminal's main screen hands its list to MenuScreenBase.register_substates while it
	-- opens (menu_screen_campaign_multiplayer_main on_enter); for that one call the list gains Modded.
	hooks.loader_hook("MenuScreenCampaignMultiplayerMain", "on_enter", function(original, self, ...)
		local base = B.rawget(B.G, "MenuScreenBase")
		local real = B.type(base) == "table" and base.register_substates
		if B.type(real) ~= "function" then return original(self, ...) end
		base.register_substates = function(screen, list, index, ...)
			if screen == self then
				local ok, out, at = B.pcall(M.tabs, list, index)
				if ok then list, index = out, at end
			end
			return real(screen, list, index, ...)
		end
		local function pack(...) return { n = B.select("#", ...), ... } end
		local r = pack(B.pcall(original, self, ...))
		base.register_substates = real
		if not r[1] then B.error(r[2], 0) end
		return B.unpack(r, 2, r.n)
	end)
	-- The Modded tab is the game's public search, with its own filter object: the game picks the filter
	-- by the tab's params and tests filter.name == "public" for the search, the camera and the layout.
	hooks.loader_hook(class, "init", function(original, self, ...)
		local r = { original(self, ...) }
		if B.type(self.filters) == "table" and not self.filters.modded then
			self.filters.modded = { name = "public", title = TAB_MODDED, obsidian_modded = true }
		end
		return B.unpack(r)
	end)
	hooks.loader_hook(class, "add_room", function(original, self, room, ...)
		local filter = self.filter
		if B.type(filter) == "table" and filter.obsidian_modded and B.type(room) == "table"
			and not (room.obsidian and room.obsidian.modded) then
			return
		end
		return original(self, room, ...)
	end)
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
		local ok, err = B.pcall(draw_mods, self, gui, z, dt)
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
