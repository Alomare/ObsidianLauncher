-- Sandbox saves, per-save mod data, lobby info and the multiplayer terminal.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local ID = "abcd1234"
local SANDBOX = "sandbox\t" .. ID .. "\n"
local TEXTS = { en = { sandbox_failed = "SANDBOX FAILED", lobby_sandbox_public = "Sandbox Public",
	lobby_uses_sandbox = "This lobby uses Sandbox saves.", lobby_uses_official = "This lobby uses Official saves.",
	lobby_modded_mark = "MODDED", lobby_mods_heading = "Mods here", lobby_more_mods = "+%d more", lobby_tab_public = "Public", lobby_tab_modded = "Modded", lobby_mods_range = "%d to %d of %d",
	lobby_filter_hide_modded = "Hide modded", campaign_not_uploaded_sandbox = "Sandbox line" } }

local function fake_save()
	local sm = { calls = {}, saved = 0, backup = 0 }
	function sm:setup(system, file) self.system_type, self.filename = system, file; self.calls[#self.calls + 1] = { system, file } end
	function sm:set_backup_system() self.backup = self.backup + 1 end
	function sm:save(done) self.saved = self.saved + 1; if done then done() end end
	function sm:user_save(done) self.saved = self.saved + 1; if done then done() end end
	function sm:get() end
	function sm:set() end
	function sm:user_set() end
	return sm
end

local function boot(settings, over, save)
	rawset(_G, "SaveManager", save or fake_save())
	local d = { texts = TEXTS }
	for k, v in pairs(over or {}) do d[k] = v end
	local e = E.new({ deploy = E.deploy(d), settings = settings, mods = over and over.src })
	e:boot()
	return e, rawget(_G, "SaveManager")
end

test("a_sandbox_opens_a_separate_file_through_the_default_system", function()
	local e, sm = boot(SANDBOX)
	sm:setup("steam", "helldivers_save", "T")
	sm:set_backup_system("default", "helldivers_steam_1")
	E.eq(sm.calls[1][1], "default")
	E.eq(sm.calls[1][2], "helldivers_sandbox_" .. ID)
	E.eq(sm.backup, 0, "no backup system")
	local done = 0
	sm:save(function() done = done + 1 end)
	E.eq(sm.saved, 1, "the save goes through")
	E.eq(done, 1)
	E.has(e:log(), "sandbox: save redirected to")
end)

test("the_official_path_is_left_alone", function()
	local e, sm = boot(nil)
	sm:setup("steam", "helldivers_save")
	sm:set_backup_system("default", "x")
	E.eq(sm.calls[1][2], "helldivers_save")
	E.eq(sm.backup, 1)
	E.eq(sm.saved, 0)
	sm:save()
	E.eq(sm.saved, 1)
	E.eq(e:core().sandbox, nil)
end)

test("a_setup_that_came_too_early_keeps_saving_off", function()
	local sm = fake_save()
	sm:setup("steam", "helldivers_save")
	local e = boot(SANDBOX, nil, sm)
	-- The redirect never ran, so every save the game tries stays away from the Official file.
	local done = 0
	sm:save(function() done = done + 1 end)
	sm:user_save(function() done = done + 1 end)
	E.eq(sm.saved, 0)
	E.eq(done, 2, "the game's flow doesn't hang")
	e:start_frames()
	e:frames(10)
	E.has(e:log(), "sandbox: the Sandbox save didn't open")
	E.eq(e:core().notices_shown[1] ~= nil, true)
end)

test("a_game_without_save_manager_keeps_the_guard_pending", function()
	local e = E.new({ deploy = E.deploy({ texts = TEXTS }), settings = SANDBOX })
	rawset(_G, "SaveManager", nil)
	e:boot()
	local sm = fake_save()
	rawset(_G, "SaveManager", sm)
	e:start_frames()
	sm:setup("steam", "helldivers_save")
	E.eq(sm.calls[1][2], "helldivers_sandbox_" .. ID, "the hook went in at the next require")
end)

test("a_bad_sandbox_line_never_opens_the_official_save_for_writing", function()
	for _, line in ipairs({ "sandbox\tABCD1234\n", "sandbox\tabc\n", "sandbox\t\n", "sandbox\tabcd1234\textra\n" }) do
		local e, sm = boot(line)
		sm:setup("steam", "helldivers_save")
		E.eq(sm.calls[1][2], "helldivers_save", "no id to redirect to: " .. line)
		sm:save()
		E.eq(sm.saved, 0, "saving stays off")
		E.eq(e:core().sandbox, nil)
		E.eq(e:core().sandboxed(), true)
	end
end)

test("an_unknown_build_on_a_sandbox_keeps_saving_off", function()
	rawset(_G, "SaveManager", fake_save())
	local e = E.new({ deploy = E.deploy({ texts = TEXTS }), settings = SANDBOX, revision = "parent: 1:ffff other" })
	e:boot()
	local sm = rawget(_G, "SaveManager")
	sm:setup("steam", "helldivers_save")
	E.eq(sm.calls[1][2], "helldivers_save", "no redirect on a build that is not known")
	sm:save()
	E.eq(sm.saved, 0)
	e:start_frames()
	e:frames(10)
	E.has(e:log(), "build unknown")
	E.has(e:log(), "sandbox: the Sandbox save didn't open")
end)

test("an_unknown_build_on_the_official_save_stays_vanilla", function()
	local sm = fake_save()
	rawset(_G, "SaveManager", sm)
	local e = E.new({ deploy = E.deploy(), revision = "parent: 1:ffff other" })
	e:boot()
	sm:save()
	E.eq(sm.saved, 1)
end)

test("settings_read_hide_client_mods", function()
	local e = boot("hide_client_mods\t1\n")
	E.eq(e:core().hide_client_mods, true)
	e = boot("hide_client_mods\t2\n")
	E.eq(e:core().hide_client_mods, nil)
end)

-- Per-save mod data.

local function with_mod(settings)
	local e = E.new({
		deploy = E.deploy({ texts = TEXTS, mods = { E.mod("m") } }),
		settings = settings,
		mods = { ["obsidian/mods/m/main"] = "local d = obsidian.save_data; DATA = d; return true" },
	})
	return e
end

local function run_as(e, id, fn, ...)
	local core, old = e:core(), nil
	old = core.current
	core.current = { id = id, mod = { options = {} } }
	local r = { fn(...) }
	core.current = old
	return unpack(r)
end

test("save_data_goes_with_the_official_save_or_the_sandbox", function()
	local e = with_mod(nil)
	e:boot()
	local d = rawget(_G, "DATA")
	E.eq(run_as(e, "m", d.get, "k"), nil)
	E.eq(run_as(e, "m", d.set, "k", "v\tw"), true)
	run_as(e, "m", d.set, "b", true)
	run_as(e, "m", d.set, "n", 1.5)
	run_as(e, "other", d.set, "o", "x")
	E.eq(run_as(e, "m", d.get, "k"), "v\tw")
	E.eq(run_as(e, "other", d.get, "k"), nil, "each mod has separate data")
	E.eq(run_as(e, "m", d.sandbox), nil)
	local text = E.read(e.saves_dir .. "/helldivers_official.obsidian")
	E.has(text, "m\tb\tb\t1\n")
	E.has(text, "m\tk\ts\tv\\tw\n")
	E.has(text, "m\tn\tn\t1.5\n")
	E.has(text, "other\to\ts\tx\n")
	run_as(e, "m", d.set, "b", nil)
	E.lacks(E.read(e.saves_dir .. "/helldivers_official.obsidian"), "m\tb\t")
end)

test("save_data_of_a_sandbox_is_a_separate_file", function()
	local e = with_mod(SANDBOX)
	e:boot()
	local d = rawget(_G, "DATA")
	run_as(e, "m", d.set, "k", "here")
	E.eq(run_as(e, "m", d.sandbox), ID)
	E.has(E.read(e.saves_dir .. "/helldivers_sandbox_" .. ID .. ".obsidian"), "m\tk\ts\there\n")
	E.eq(E.read(e.saves_dir .. "/helldivers_official.obsidian"), nil)
end)

test("save_data_keeps_a_removed_mods_data_and_lines_it_does_not_know", function()
	local e = with_mod(nil)
	local f = assert(io.open(e.saves_dir .. "/helldivers_official.obsidian", "wb"))
	f:write("gone\tvalue\tn\t7\nfuture\tformat\tz\t?\n")
	f:close()
	e:boot()
	local d = rawget(_G, "DATA")
	run_as(e, "m", d.set, "k", "v")
	local text = E.read(e.saves_dir .. "/helldivers_official.obsidian")
	E.has(text, "gone\tvalue\tn\t7\n", "a removed mod's data stays")
	E.has(text, "future\tformat\tz\t?\n", "a line from a newer app stays")
end)

test("a_bad_sandbox_line_has_no_save_data_file", function()
	local e = with_mod("sandbox\tbad\n")
	e:boot()
	local d = rawget(_G, "DATA")
	E.eq(run_as(e, "m", d.set, "k", "v"), false)
	E.eq(E.read(e.saves_dir .. "/helldivers_official.obsidian"), nil)
end)

-- Isolation.

local function revision_hash()
	return string.match(rawget(_G, "BUILD_REVISION_INFORMATION"), "parent: [0-9]+:([0-9a-f]+)")
end

test("a_sandbox_moves_to_a_separate_pool", function()
	local e = boot(SANDBOX, { hashes = { open = "aaaaaaaaaaaa", sandbox = "bbbbbbbbbbbb" } })
	E.eq(revision_hash(), "3bd83d77c8aebbbbbbbbbbbb")
	E.has(e:log(), "pool: sandbox bbbbbbbbbbbb")
	E.lacks(e:log(), "open pool lobbies listed too", "the bridge stays off")
end)

test("a_sandbox_without_a_tag_still_leaves_the_official_pool", function()
	local e = boot(SANDBOX, { hashes = { open = "aaaaaaaaaaaa" } })
	E.eq(revision_hash() ~= "3bd83d77c8ae", true)
	E.has(e:log(), "using the fallback")
end)

test("the_official_pool_stays_as_it_was", function()
	boot(nil, { hashes = { open = "aaaaaaaaaaaa", sandbox = "bbbbbbbbbbbb" } })
	E.eq(revision_hash(), "3bd83d77c8ae")
end)

test("a_sandbox_reports_nothing_and_drops_the_queues", function()
	local sm = fake_save()
	local set = {}
	function sm:set(key, value) set[key] = { value } end
	function sm:user_set(key, value) set[key] = { value } end
	local sent = 0
	rawset(_G, "CampaignManager", { upload_mission_stats = function(_, stats) sent = sent + 1 end })
	rawset(_G, "UserManager", { unlock_trophy = function() sent = sent + 100 end, get_user_id = function() return 1 end })
	boot(SANDBOX, nil, sm)
	sm:setup("steam", "helldivers_save")
	local stats = { 1, 2 }
	rawget(_G, "CampaignManager"):upload_mission_stats(stats)
	E.eq(sent, 0)
	E.eq(#stats, 0, "emptied, so the game can't send them later")
	sm:save()
	E.eq(sm.saved, 1)
	E.eq(set.mission_stats ~= nil, true, "queues dropped at save")
	rawget(_G, "UserManager"):unlock_trophy("x")
	E.eq(sent, 0, "no achievement")
end)

test("achievements_are_blocked_without_a_notice_for_a_sandbox", function()
	rawset(_G, "UserManager", { unlock_trophy = function() error("unlocked") end, get_user_id = function() return 1 end })
	local e = boot(SANDBOX)
	rawget(_G, "UserManager"):unlock_trophy("x")
	e:start_frames()
	e:frames(2)
	for _, n in ipairs(e:core().notices_shown) do E.eq(n.key ~= "achievements_blocked", true, "the guardrail wasn't chosen") end
end)

-- Lobby info.

local function host(settings, mods)
	local list, sources = {}, {}
	for _, m in ipairs(mods or {}) do
		list[#list + 1] = m
		sources[m.entry] = "return true"
	end
	local keyset = {}
	local set_calls = 0
	rawset(_G, "SteamLobby", { set_data = function(_, key, value) keyset[key] = value; set_calls = set_calls + 1 end })
	local engine = { data = function(_, key) return keyset[key] end }
	local is_host = true
	rawset(_G, "NetworkGlobal", { lobby = { lobby = engine, is_host = function() return is_host end,
		my_id = "me", state = function() return "joined" end, members = function() return { "me" } end,
		lobby_host = function() return "me" end, member_data = function() return nil end, set_member_data = function() end } })
	local e = E.new({ deploy = E.deploy({ texts = TEXTS, mods = list }), settings = settings, mods = sources })
	e:boot()
	e:start_frames()
	return e, keyset, function() return set_calls end, function(v) is_host = v end
end

test("the_host_publishes_the_lobby_info", function()
	local e, keys = host(nil, { E.mod("a", { name = { en = "Alpha" }, version = "1.2.0", scope = "session" }), E.mod("b", { version = "2.0.0" }) })
	e:frames(60)
	E.eq(keys.obsidian_lobby, "1;;Alpha@1.2.0|b@2.0.0")
end)

test("a_sandbox_host_sets_the_flag", function()
	local e, keys = host(SANDBOX, { E.mod("a") })
	e:frames(60)
	E.eq(keys.obsidian_lobby, "1;s;a@1.0.0")
end)

test("hide_client_mods_keeps_only_session_and_expansion_mods", function()
	local e, keys = host("hide_client_mods\t1\n", { E.mod("c", { scope = "client" }), E.mod("s", { scope = "session" }), E.mod("x", { scope = "expansion" }) })
	e:frames(60)
	E.eq(keys.obsidian_lobby, "1;;s@1.0.0|x@1.0.0")
end)

test("no_mods_still_publishes", function()
	local e, keys = host(nil, {})
	e:frames(60)
	E.eq(keys.obsidian_lobby, "1;;")
end)

test("names_are_cleaned_of_separators", function()
	local e, keys = host(nil, { E.mod("a", { name = { en = "Na;me|x@\1y" }, version = "1;0" }) })
	e:frames(60)
	E.eq(keys.obsidian_lobby, "1;;Namexy@10")
end)

test("a_long_list_is_capped_with_a_count", function()
	local list = {}
	for i = 1, 200 do list[i] = E.mod("mod" .. i, { name = { en = string.rep("n", 40) .. i } }) end
	local e, keys = host(nil, list)
	e:frames(60)
	local v = keys.obsidian_lobby
	E.eq(#v <= 4096, true, "under the limit")
	E.eq(v:match("|%+(%d+)$") ~= nil, true, "ends with the count: " .. v:sub(-20))
	E.has(v, "1;;")
end)

test("the_info_is_published_once_and_again_after_a_new_host_changes_it", function()
	local e, keys, calls = host(nil, { E.mod("a") })
	e:frames(120)
	E.eq(calls(), 1, "published once")
	keys.obsidian_lobby = "1;;someone_else@1"
	e:frames(60)
	E.eq(keys.obsidian_lobby, "1;;a@1.0.0", "taken over after migration")
	E.eq(calls(), 2)
end)

test("a_member_who_is_not_host_does_not_publish", function()
	local e, keys, calls, set_host = host(nil, { E.mod("a") })
	set_host(false)
	e:frames(120)
	E.eq(keys.obsidian_lobby, nil)
	E.eq(calls(), 0)
end)

-- The terminal.

local function terminal(settings)
	local screen = { displayable_rooms = {}, room_browser = { data = function(_, i, key) return i == 1 and key == "obsidian_lobby" and terminal_value or nil end }, current_room_index = 1, selected_room = 1, info_message = "" }
	local drawn, texts = {}, {}
	local C = { extracted = nil }
	function C.extract_room_info(self, a, b, c, d) return { name = "room", build_game_hash = "h" } end
	function C.add_room(self, room) self.displayable_rooms[#self.displayable_rooms + 1] = room end
	function C.draw_looping_scroll(self, gui, z, pos, id, text) drawn[tostring(id)] = text end
	function C.draw_selected_room_info(self, gui, dt, z) end
	function C.on_enter(self) end
	function C.init(self) self.filters = { public = { name = "public" }, friends = { name = "friends" } } end
	function C.rpc_on_join_denied(self, sender, reason) self.info_message = "game:" .. tostring(reason) end
	function C.update_join(self) end
	rawset(_G, "MenuScreenCampaignMultiplayer", C)
	rawset(_G, "GameSettings", { multiplayer_filters = {}, room_data_values = { "build_game_hash" }, room_privacy_settings = { public = 0 } })
	rawset(_G, "FontSettings", { fonts = { body = { class = "f", font_size = 14, line_height = 18 } } })
	rawset(_G, "Vector2", function(x, y) return { x, y } end)
	rawset(_G, "Vector3", function(x, y, z) return { x, y, z } end)
	rawset(_G, "Color", function(...) return { ... } end)
	rawset(_G, "LocalizationManager", { string = function(_, key) return "game:" .. key end })
	local engine = { data = function(_, key) if key == "data" then return screen.lobby_data end; return key == "obsidian_lobby" and screen.lobby_info or nil end }
	rawset(_G, "NetworkGlobal", { lobby = { lobby = engine, is_host = function() return false end }, data_string_to_table = function(raw, keys) return { build_game_hash = raw } end })
	local e = E.new({ deploy = E.deploy({ texts = TEXTS }), settings = settings })
	e:boot()
	e:start_frames()
	setmetatable(screen, { __index = rawget(_G, "MenuScreenCampaignMultiplayer") })
	return screen, drawn, e
end

test("extract_reads_and_cleans_the_lobby_info", function()
	local screen = terminal(nil)
	rawset(_G, "terminal_value", "1;;Good Mod@1.0.0|Bad\128Name\1@2|+3")
	local room = screen:extract_room_info("n", "d", "id", nil)
	local info = room.obsidian
	E.eq(info.modded, true)
	E.eq(info.sandbox, false)
	E.eq(info.mods[1], "Good Mod 1.0.0")
	E.eq(info.mods[2], "BadName 2")
	E.eq(info.more, 3)
end)

test("a_room_without_info_has_none", function()
	local screen = terminal(nil)
	rawset(_G, "terminal_value", false)
	local room = screen:extract_room_info("n", "d", "id", nil)
	E.eq(room.obsidian, nil)
	rawset(_G, "terminal_value", "garbage")
	room = screen:extract_room_info("n", "d", "id", nil)
	E.eq(room.obsidian, nil)
end)

test("an_empty_list_is_not_modded", function()
	local screen = terminal(nil)
	rawset(_G, "terminal_value", "1;;")
	local room = screen:extract_room_info("n", "d", "id", nil)
	E.eq(room.obsidian.modded, false)
end)

test("the_room_row_shows_the_mark_only_for_modded_rooms", function()
	local screen, drawn = terminal(nil)
	screen.displayable_rooms = { { obsidian = { modded = true } }, { obsidian = { modded = false } }, { } }
	screen:draw_looping_scroll("gui", 1, "p", "sub1", "2/4")
	screen:draw_looping_scroll("gui", 1, "p", "sub2", "1/4")
	screen:draw_looping_scroll("gui", 1, "p", "sub3", "3/4")
	screen:draw_looping_scroll("gui", 1, "p", 1, "Host name")
	E.eq(drawn.sub1, "2/4  MODDED")
	E.eq(drawn.sub2, "1/4")
	E.eq(drawn.sub3, "3/4")
	E.eq(drawn["1"], "Host name")
end)

test("the_hide_modded_filter_is_added_once_and_decides_by_the_mark", function()
	local screen = terminal(nil)
	screen:on_enter()
	screen:on_enter()
	local filters = rawget(_G, "GameSettings").multiplayer_filters
	E.eq(#filters, 1, "added once; the Modded tab replaced Modded only")
	local hide = filters[1]
	E.eq(hide.name, "filter_hide_modded")
	local modded, plain = { obsidian = { modded = true } }, { obsidian = { modded = false } }
	E.eq(hide.filter_func(modded, true), true, "dropped")
	E.eq(not hide.filter_func(plain, true), true, "kept")
	E.eq(hide.filter_func(modded, false), false, "off")
	E.eq(rawget(_G, "LocalizationManager"):string("filter_hide_modded"), "Hide modded")
end)

test("the_terminal_gets_a_modded_tab_between_public_and_friends", function()
	local e = select(3, terminal(nil))
	local lobby = e.loaded["obsidian/loader/lobby"]
	local state = {}
	local list = { { id = "matchmaking", name = "matchmaking", state = state, params = { filter = "public" } },
		{ id = "friends", name = "friends", state = state, params = { filter = "friends" } } }
	local out, index = lobby.tabs(list, 2)
	E.eq(#out, 3)
	E.eq(out[1].name, "obsidian_lobby_public") E.eq(out[1].params.filter, "public")
	E.eq(out[2].name, "obsidian_lobby_modded") E.eq(out[2].params.filter, "modded") E.eq(out[2].state, state, "the same screen")
	E.eq(out[3].name, "friends")
	E.eq(index, 3, "Friends asked for is still Friends")
	E.eq(select(2, lobby.tabs(list, 1)), 1)
	E.eq(#lobby.tabs(out, 1), 3, "never twice")
	local lm = rawget(_G, "LocalizationManager")
	E.eq(lm:string("obsidian_lobby_public"), "Public")
	E.eq(lm:string("obsidian_lobby_modded"), "Modded")
end)

test("the_main_screen_registers_the_three_tabs", function()
	local got = nil
	rawset(_G, "MenuScreenBase", { register_substates = function(self, list, index) got = { list = list, index = index } end })
	local main = {}
	function main.on_enter(self)
		rawget(_G, "MenuScreenBase").register_substates(self, { { id = "matchmaking", name = "matchmaking" }, { id = "friends", name = "friends" } }, 1)
		return "entered"
	end
	rawset(_G, "MenuScreenCampaignMultiplayerMain", main)
	terminal(nil)
	local screen = setmetatable({}, { __index = rawget(_G, "MenuScreenCampaignMultiplayerMain") })
	E.eq(screen:on_enter(), "entered")
	E.eq(#got.list, 3)
	E.eq(got.list[2].name, "obsidian_lobby_modded")
	-- Other screens keep their tabs, and the base function is the game's again.
	rawget(_G, "MenuScreenBase").register_substates({}, { { id = "matchmaking" } }, 1)
	E.eq(#got.list, 1)
end)

test("the_modded_tab_lists_only_modded_rooms", function()
	local screen = terminal(nil)
	screen:init()
	E.eq(screen.filters.modded.name, "public", "the game's public search")
	E.eq(screen.filters.modded.obsidian_modded, true)
	screen.filter = screen.filters.modded
	screen:add_room({ name = "plain", obsidian = { modded = false } })
	screen:add_room({ name = "bare" })
	screen:add_room({ name = "modded", obsidian = { modded = true } })
	E.eq(#screen.displayable_rooms, 1)
	E.eq(screen.displayable_rooms[1].name, "modded")
	screen.filter = screen.filters.public
	screen:add_room({ name = "plain", obsidian = { modded = false } })
	E.eq(#screen.displayable_rooms, 2, "Public lists every room")
end)

local function gui_recorder()
	local g = { texts = {}, opened = 0, closed = 0 }
	function g:begin_template() self.opened = self.opened + 1 end
	function g:end_template() self.closed = self.closed + 1 end
	function g:draw_text(text) self.texts[#self.texts + 1] = text end
	function g:draw_rect() end
	return g
end

test("the_selected_rooms_mods_scroll_eight_at_a_time_with_the_wheel", function()
	local screen = terminal(nil)
	local mods = {}
	for i = 1, 12 do mods[i] = "mod" .. i end
	screen.displayable_rooms = { { obsidian = { modded = true, mods = mods, more = 3 } } }
	screen.user_pad_number = 99
	rawset(_G, "KEYBOARD_MOUSE_PAD_NUMBER", 99)
	local over = true
	rawset(_G, "MenuScreenBase", { is_box_selected_by_cursor = function() return over end })
	local g = gui_recorder()
	screen:draw_selected_room_info(g, 0.1, 5)
	E.eq(g.texts[1], "Mods here")
	E.eq(g.texts[2], "mod1")
	E.eq(g.texts[9], "mod8")
	E.eq(g.texts[10], "1 to 8 of 13", "the host's +3 is the last line of the list")
	E.eq(g.opened, g.closed, "the template closes")
	screen.mouse_scroll_input = 1
	g = gui_recorder()
	screen:draw_selected_room_info(g, 0.1, 5)
	E.eq(g.texts[2], "mod2", "one line down")
	E.eq(screen.mouse_scroll_input, nil, "the turn is taken")
	for _ = 1, 10 do
		screen.mouse_scroll_input = 1
		screen:draw_selected_room_info(gui_recorder(), 0.1, 5)
	end
	g = gui_recorder()
	screen:draw_selected_room_info(g, 0.1, 5)
	E.eq(g.texts[9], "+3 more", "stops at the end")
	E.eq(g.texts[10], "6 to 13 of 13")
	over = false
	screen.mouse_scroll_input = -1
	screen:draw_selected_room_info(gui_recorder(), 0.1, 5)
	E.eq(screen.mouse_scroll_input, -1, "a turn away from the list is the game's")
end)

test("a_pad_moves_the_mod_list_on_by_itself_and_starts_over", function()
	local screen = terminal(nil)
	local mods = {}
	for i = 1, 9 do mods[i] = "mod" .. i end
	screen.displayable_rooms = { { obsidian = { modded = true, mods = mods, more = 0 } } }
	screen.user_pad_number = 1
	rawset(_G, "KEYBOARD_MOUSE_PAD_NUMBER", 99)
	local g = gui_recorder()
	screen:draw_selected_room_info(g, 1.6, 5)
	E.eq(g.texts[2], "mod2")
	g = gui_recorder()
	screen:draw_selected_room_info(g, 1.6, 5)
	E.eq(g.texts[2], "mod1", "back to the top after the end")
end)

test("a_short_list_has_no_range_line", function()
	local screen = terminal(nil)
	screen.displayable_rooms = { { obsidian = { modded = true, mods = { "a", "b" }, more = 0 } } }
	local g = gui_recorder()
	screen:draw_selected_room_info(g, 0.1, 5)
	E.eq(#g.texts, 3)
end)

test("a_room_with_no_mods_draws_no_list", function()
	local screen = terminal(nil)
	screen.displayable_rooms = { { obsidian = { modded = false, mods = {}, more = 0 } } }
	local g = gui_recorder()
	screen:draw_selected_room_info(g, 0.1, 5)
	E.eq(#g.texts, 0)
end)

test("a_gui_failure_closes_the_template_and_leaves_the_game_alone", function()
	local screen = terminal(nil)
	screen.displayable_rooms = { { obsidian = { modded = true, mods = { "m" }, more = 0 } } }
	local g = gui_recorder()
	function g:draw_text() error("drawing failed") end
	screen:draw_selected_room_info(g, 0.1, 5)
	E.eq(g.opened, 1)
	E.eq(g.closed, 1)
end)

test("sandbox_public_spells_on_sandbox_saves_only", function()
	terminal(SANDBOX)
	E.eq(rawget(_G, "LocalizationManager"):string("campaign_public"), "Sandbox Public")
end)

test("the_official_public_label_stays", function()
	terminal(nil)
	E.eq(rawget(_G, "LocalizationManager"):string("campaign_public"), "game:campaign_public")
end)

test("a_refused_join_across_the_line_names_the_saves", function()
	local screen = terminal(nil)
	screen.lobby_data = "3bd83d77c8aeaabbcc"
	screen.lobby_info = "1;s;x@1"
	screen:rpc_on_join_denied("p", "network_join_denied_wrong_version")
	E.eq(screen.info_message, "This lobby uses Sandbox saves.")
end)

test("a_sandbox_player_refused_by_an_official_lobby_is_told_so", function()
	local screen = terminal(SANDBOX)
	screen.lobby_data = "3bd83d77c8ae"
	screen:rpc_on_join_denied("p", "network_join_denied_wrong_version")
	E.eq(screen.info_message, "This lobby uses Official saves.")
end)

test("another_game_version_keeps_the_games_words", function()
	local screen = terminal(nil)
	screen.lobby_data = "ffffffffffff"
	screen.lobby_info = "1;s;x@1"
	screen:rpc_on_join_denied("p", "network_join_denied_wrong_version")
	E.eq(screen.info_message, "game:network_join_denied_wrong_version")
end)

test("same_side_refusals_keep_the_games_words", function()
	local screen = terminal(nil)
	screen.lobby_data = "3bd83d77c8ae"
	screen.lobby_info = "1;;x@1"
	screen:rpc_on_join_denied("p", "network_join_denied_wrong_version")
	E.eq(screen.info_message, "game:network_join_denied_wrong_version", "a public door, say")
end)

test("other_refusals_are_not_touched", function()
	local screen = terminal(SANDBOX)
	screen.lobby_data = "3bd83d77c8ae"
	screen:rpc_on_join_denied("p", "network_join_denied_game_full")
	E.eq(screen.info_message, "game:network_join_denied_game_full")
end)

return T
