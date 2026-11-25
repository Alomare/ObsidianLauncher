-- Obsidian API's Memory Bridge: code compiled outside the game, run on the game thread, answered in a file.
local E = require "fake_engine"
local G = require "fake_game"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local BRIDGE = "obsidian/mods/obsidian.api/lib/bridge"
local ON = "option\tobsidian.api\tmemory_bridge\ttoggle\t1\n"
local WINDOWS = package.config:sub(1, 1) == "\\"

local function mkdir(path)
	if WINDOWS then os.execute('mkdir "' .. path:gsub("/", "\\") .. '" >nul 2>nul') else os.execute('mkdir -p "' .. path .. '"') end
end

-- opts.off: the option as it comes, off. opts.lobby: as fake_engine takes it.
local function start(opts)
	opts = opts or {}
	local mods = {}
	local api = G.bundled("obsidian.api", mods)
	local deploy = {
		E.mod("obsidian.api", { scope = "client", texts = api.texts, keybinds = api.keybinds, name = api.name, options = { { id = "memory_bridge", type = "toggle", default = false } } }),
	}
	local e = E.new({ deploy = E.deploy({ mods = deploy }), mods = mods, settings = not opts.off and ON or nil, lobby = opts.lobby })
	e:boot()
	e:start_frames()
	e.bridge_dir = e.obsidian_dir .. "/bridge/"
	mkdir(e.bridge_dir)
	-- The game always runs on Windows paths (natively or under Proton); on a Linux test machine the
	-- backslashes of bridge.dir would name a file instead of a folder.
	local b = e.loaded[BRIDGE]
	if b then b.dir = function() return e.bridge_dir end end
	e:frames(2)
	return e, e.loaded[BRIDGE]
end

local seq = 0
-- The real one, for the tool's side, as a test may stand in for the game's.
local compile = loadstring
-- Sends source as the tool does (bytecode, through a rename), runs a frame, answers the answer's text.
local function send(e, source, args)
	seq = seq + 1
	local id = "t" .. seq
	local code = string.dump(assert(compile("return function(...) " .. source .. "\nend", "=test")))
	local f = assert(io.open(e.bridge_dir .. "request.tmp", "wb"))
	f:write("OBSBRIDGE 1\n", id, "\n", table.concat(args or {}, "\t"), "\n\n", code)
	f:close()
	assert(os.rename(e.bridge_dir .. "request.tmp", e.bridge_dir .. "request"))
	e:frames(1)
	local answer = E.read(e.bridge_dir .. "answer_" .. id)
	os.remove(e.bridge_dir .. "answer_" .. id)
	return answer, id
end

test("off_by_default_installs_nothing", function()
	local e, b = start({ off = true })
	E.eq(b.on, false)
	E.eq(send(e, "return 1"), nil, "no answer")
	E.lacks(e:log(), "CampaignManager", "no hold on the reports")
end)

test("runs_code_and_answers_what_it_printed_and_returned", function()
	local e = start()
	local a, id = send(e, "print('hi', 2) return 1 + 1, { x = 3 }")
	E.has(a, "ok " .. id .. " ")
	E.has(a, "\nhi 2\n")
	E.has(a, "= 2\n")
	E.has(a, "x = 3,")
	E.eq(io.open(e.bridge_dir .. "request", "rb"), nil, "the request is taken")
	E.eq(io.open(e.bridge_dir .. "running", "rb"), nil, "the breadcrumb goes once it ends")
end)

test("errors_answer_with_the_traceback", function()
	local e = start()
	local a, id = send(e, "error('broken on purpose')")
	E.has(a, "error " .. id)
	E.has(a, "broken on purpose")
end)

test("arguments_reach_the_code", function()
	local e = start()
	E.has(send(e, "local a, b = ... return a .. '+' .. b", { "one", "two" }), "= \"one+two\"")
end)

-- A mod's loadstring in the retail game answers nothing, bytecode or not: the interpreter runs it.
local function without_loadstring(fn)
	local real = loadstring
	rawset(_G, "loadstring", function() return nil end)
	local ok, err = pcall(fn)
	rawset(_G, "loadstring", real)
	assert(ok, err)
end

test("where_loadstring_answers_nothing_the_interpreter_runs_the_code", function()
	local e = start()
	without_loadstring(function()
		E.has(send(e, "print('p', 1) return 6 * 7, keep ~= nil"), "p 1\n= 42\n= true")
		local a = send(e, "local t = nil\nreturn t.x")
		E.has(a, "test:2: attempt to index")
		E.has(a, "stack traceback:")
		E.has(send(e, "error('mine')"), "test:1: mine")
	end)
end)

test("interpreted_hooks_and_frame_functions_run_as_real_functions", function()
	local e = start()
	rawset(_G, "LiveTarget", { value = function(_, x) return x end })
	without_loadstring(function()
		send(e, "keep.n = 0 every('n', function(dt) keep.n = keep.n + 1 end) hook('LiveTarget', 'value', function(original, self, x) return original(self, x) * 3 end)")
	end)
	e:frames(4)
	E.eq(rawget(_G, "LiveTarget"):value(5), 15)
	E.has(send(e, "return keep.n >= 4"), "= true")
end)

test("source_text_is_refused_as_the_game_would", function()
	local e = start()
	local f = assert(io.open(e.bridge_dir .. "request", "wb"))
	f:write("OBSBRIDGE 1\nsrc\n\n\n", "return 1")
	f:close()
	local real = loadstring
	-- The game parses no source: loadstring of text answers nothing.
	rawset(_G, "loadstring", function(s, ...) if s:sub(1, 1) ~= "\27" then return nil end return real(s, ...) end)
	e:frames(1)
	rawset(_G, "loadstring", real)
	E.has(E.read(e.bridge_dir .. "answer_src") or "", "the code didn't load")
end)

test("keep_lasts_between_requests_and_globals_stay_in_the_request", function()
	local e = start()
	send(e, "keep.n = 41 mine = true")
	E.has(send(e, "return keep.n + 1"), "= 42")
	E.has(send(e, "return rawget(_G, 'mine')"), "= nil")
end)

test("every_runs_each_frame_until_it_says_true_and_emit_streams", function()
	local e = start()
	send(e, "keep.c = 0 every('count', function() keep.c = keep.c + 1 if keep.c == 5 then emit('five') return true end end)")
	e:frames(10)
	E.has(send(e, "return keep.c"), "= 5")
	E.has(E.read(e.bridge_dir .. "stream.txt") or "", "\tfive")
end)

test("a_failing_frame_function_stops_and_says_so", function()
	local e = start()
	send(e, "every('bad', function() error('nope') end)")
	e:frames(3)
	E.has(E.read(e.bridge_dir .. "stream.txt") or "", "frame function bad stopped")
	E.has(send(e, "return api ~= nil"), "= true")
end)

test("live_hooks_replace_and_come_off", function()
	local e = start()
	rawset(_G, "LiveTarget", { value = function(_, x) return x * 2 end })
	send(e, "hook('LiveTarget', 'value', function(original, self, x) return original(self, x) + 1 end)")
	local t = rawget(_G, "LiveTarget")
	E.eq(t:value(10), 21)
	send(e, "hook('LiveTarget', 'value', function(original, self, x) return 0 end)")
	E.eq(t:value(10), 0, "the same method again replaces it")
	send(e, "unhook('LiveTarget', 'value')")
	E.eq(t:value(10), 20, "off, the game's function")
	send(e, "hook('LiveTarget', 'value', function() error('bad hook') end)")
	E.eq(t:value(10), 20, "a failing hook falls back to the game's function")
	E.has(E.read(e.bridge_dir .. "stream.txt") or "", "hook LiveTarget.value taken off")
end)

test("undo_takes_back_writes_frames_and_hooks", function()
	local e = start()
	rawset(_G, "LiveTable", { speed = 5 })
	rawset(_G, "LiveTarget", { value = function() return 1 end })
	send(e, "set(LiveTable, 'speed', 99) set(LiveTable, 'speed', 100) every('x', function() end) hook('LiveTarget', 'value', function() return 2 end)")
	E.eq(rawget(_G, "LiveTable").speed, 100)
	local a = send(e, "return undo()")
	E.has(a, "writes = 1")
	E.has(a, "frames = 1")
	E.has(a, "hooks = 1")
	E.eq(rawget(_G, "LiveTable").speed, 5, "the first value before")
	E.eq(rawget(_G, "LiveTarget"):value(), 1)
end)

test("a_public_lobby_refuses_code", function()
	local e = start({ lobby = { my_id = "me", members = { "me", "p2" }, data = {} } })
	local a, id = send(e, "return 1")
	E.has(a, "refused " .. id)
	E.has(a, "public_lobby")
end)

test("a_lobby_that_turns_public_undoes_everything", function()
	local e, b = start()
	rawset(_G, "LiveTable", { speed = 5 })
	send(e, "set(LiveTable, 'speed', 9) every('x', function() end)")
	rawset(_G, "NetworkGlobal", { lobby = { lobby = {}, members = function() return { "me", "p2" } end } })
	e:frames(1)
	E.eq(rawget(_G, "LiveTable").speed, 5)
	E.eq(next(b.frames), nil)
	E.has(E.read(e.bridge_dir .. "stream.txt") or "", "public lobby: undid")
end)

test("a_private_lobby_runs_code", function()
	local e, b = start()
	rawset(_G, "GameSettings", { room_privacy_settings = { public = 0, friends = 1 }, room_data_values = {} })
	rawset(_G, "NetworkGlobal", {
		data_string_to_table = function(raw) return { privacy = raw } end,
		lobby = { lobby = { data = function() return "1" end }, members = function() return { "me", "p2" } end },
	})
	E.eq(b.allowed(), true)
	E.has(send(e, "return 7"), "= 7")
end)

test("status_is_written", function()
	local e = start()
	e:frames(130)
	local s = E.read(e.bridge_dir .. "status") or ""
	E.has(s, "version 1")
	E.has(s, "allowed yes")
end)

test("reports_are_held_back_while_on", function()
	local e = start()
	local cm = { sent = {} }
	for _, m in ipairs({ "upload_mission_stats", "upload_leaderboard_stats", "complete_planets" }) do
		cm[m] = function(self) self.sent[#self.sent + 1] = m end
	end
	rawset(_G, "CampaignManager", cm)
	e.opts.modules["lua/managers/campaign_manager"] = true
	rawget(_G, "require")("lua/managers/campaign_manager")
	local stats = { 1, 2 }
	cm:upload_mission_stats(stats)
	cm:complete_planets({})
	E.eq(#cm.sent, 0, "nothing sent")
	E.eq(#stats, 0, "the table handed in is emptied")
	E.has(e:log("obsidianapi"), "bridge: held back upload_mission_stats")
end)

test("shows_tables_to_a_depth_and_lists_fields", function()
	local _, b = start()
	local s = b.show({ a = { b = { c = 1 } } }, 2)
	assert(s:find("a = {"), s)
	assert(s:find("b = <table"), s)
	local obj = setmetatable({ own = 1 }, { __index = { method = function() end } })
	local list = b.fields(obj)
	for k in pairs(list) do assert(type(k) == "number", "only a list: " .. tostring(k)) end
	local f = table.concat(list, ";")
	assert(f:find("own %(number%)") and f:find("method %(function, from metatable 1%)"), f)
end)

return T
