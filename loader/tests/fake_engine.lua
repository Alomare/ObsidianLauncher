-- A stand-in for the parts of the game the loader touches, so loader tests run under plain LuaJIT 2.0.
-- Each test runs in a fresh process (see run.lua), so the fake can change globals freely.

local E = {}
local SRC = "loader/src/"

local WINDOWS = package.config:sub(1, 1) == "\\"

local function mkdir(path)
	if WINDOWS then
		os.execute('mkdir "' .. path:gsub("/", "\\") .. '" >nul 2>nul')
	else
		os.execute('mkdir -p "' .. path .. '"')
	end
end

local function read(path)
	local f = io.open(path, "rb")
	if not f then return nil end
	local s = f:read("*a")
	f:close()
	return s
end

E.read = read

function E.deploy(over)
	local d = {
		version = 1,
		revision = "parent: 32148:3bd83d77c8ae",
		hashes = { open = "aaaaaaaaaaaa" },
		has_session = false,
		mods = {},
		texts = { en = { mod_failed = "Obsidian: %s failed to start", build_unknown = "Obsidian: unknown game build" } },
	}
	for k, v in pairs(over or {}) do d[k] = v end
	return d
end

-- opts: deploy (table), mods ({ [resource name] = source }), modules ({ [name] = value }), revision (string)
function E.new(opts)
	opts = opts or {}
	local e = { opts = opts, loaded = {} }
	e.root = (os.getenv("TEMP") or (WINDOWS and "." or "/tmp")) .. "/obsidian-test-" .. tostring(os.time()) .. "-" .. tostring(math.random(1, 1e9))
	e.obsidian_dir = e.root .. "/Arrowhead/Helldivers/ObsidianModManager"
	mkdir(e.obsidian_dir .. "/logs")
	-- opts.settings: the text of settings.txt; nil leaves the file missing.
	-- Removed when not given: two tests started in the same second can share the folder.
	os.remove(e.obsidian_dir .. "/settings.txt")
	if opts.settings then
		local f = assert(io.open(e.obsidian_dir .. "/settings.txt", "wb"))
		f:write(opts.settings)
		f:close()
	end

	local real_getenv = os.getenv
	os.getenv = function(k)
		if k == "APPDATA" then return e.root end
		return real_getenv(k)
	end

	local deploy = opts.deploy or E.deploy()
	rawset(_G, "BUILD_REVISION_INFORMATION", opts.revision or "parent: 32148:3bd83d77c8ae tip\n [code] ok")
	rawset(_G, "Application", { platform = function() return "win32" end })
	rawset(_G, "ARGUMENTS", {})
	rawset(_G, "Boot", {})
	rawset(_G, "update", nil)
	rawset(_G, "shutdown", nil)

	rawset(_G, "require", function(name)
		if e.loaded[name] ~= nil then return e.loaded[name] end
		local value
		local rest = name:match("^obsidian/loader/(.+)$")
		if rest then
			value = assert(loadfile(SRC .. rest .. ".lua"))(name)
		elseif name == "obsidian/deploy" then
			value = deploy
		elseif opts.mods and opts.mods[name] then
			value = assert(loadstring(opts.mods[name], name))(name)
		elseif opts.modules and opts.modules[name] ~= nil then
			value = opts.modules[name]
		else
			error("module '" .. name .. "' not found", 2)
		end
		if value == nil then value = true end
		e.loaded[name] = value
		return value
	end)

	-- foundation/lua/util/strict: new globals only from a main chunk or C, undeclared reads are errors.
	setmetatable(_G, {
		__newindex = function(t, k, v)
			local w = debug.getinfo(2, "S").what
			if w ~= "main" and w ~= "C" then error("Undeclared global " .. tostring(k), 2) end
			rawset(t, k, v)
		end,
		__index = function(_, k) error("Undeclared global " .. tostring(k), 2) end,
	})

	-- opts.lobby: { my_id = "me", members = { "me", "p2" }, data = { p2 = "1;..." } }, and optionally
	-- host (the lobby host's peer, my_id when left out) and extra ({ [peer] = { [key] = value } }).
	-- What this machine publishes lands in published, and room data the host sets in room.
	if opts.lobby then
		local l = opts.lobby
		l.published, l.room = {}, {}
		rawset(_G, "NetworkGlobal", {
			lobby = {
				my_id = l.my_id,
				lobby = {},
				state = function() return l.state or "joined" end,
				members = function() return l.members end,
				lobby_host = function() return l.host or l.my_id end,
				member_data = function(_, peer, key)
					if key == "obsidian" then return l.data[peer] end
					if peer == l.my_id and l.published[key] ~= nil then return l.published[key] end
					return l.extra and l.extra[peer] and l.extra[peer][key]
				end,
				set_member_data = function(_, key, value) l.published[key] = value end,
				set_data = function(_, key, value) l.room[key] = value end,
			},
		})
	end

	-- Keys held down, by name; pressed() stays true while a key is held, the worst case for keybinds.
	e.held = {}
	local names = { "f8", "f9", "g" }
	rawset(_G, "Keyboard", {
		button_index = function(name)
			for i, n in ipairs(names) do
				if n == name then return i end
			end
			return nil
		end,
		pressed = function(i) return e.held[names[i]] == true end,
	})

	function e:boot()
		assert(loadfile(SRC .. "boot.lua"))("foundation/lua/build_date_information")
	end

	-- What the boot script does after the loader: define shutdown and update, then require game modules during init.
	function e:start_frames()
		rawset(_G, "FRAMES", 0)
		rawset(_G, "SHUTDOWNS", 0)
		rawset(_G, "shutdown", function() rawset(_G, "SHUTDOWNS", rawget(_G, "SHUTDOWNS") + 1) end)
		rawset(_G, "update", function(dt)
			rawset(_G, "FRAMES", rawget(_G, "FRAMES") + 1)
			-- The game's update ends with the state machine's post_update, in every state.
			local boot = rawget(_G, "Boot")
			local machine = type(boot) == "table" and rawget(boot, "state_machine")
			if type(machine) == "table" and type(machine.post_update) == "function" then machine:post_update(dt) end
		end)
		opts.modules = opts.modules or {}
		opts.modules["lua/network/network_global"] = true
		rawget(_G, "require")("lua/network/network_global")
	end

	function e:frames(n)
		for _ = 1, n do rawget(_G, "update")(0.016) end
	end

	function e:log(name)
		return read(e.obsidian_dir .. "/logs/" .. (name or "loader") .. ".log") or ""
	end

	function e:core()
		return e.loaded["obsidian/loader/core"]
	end

	return e
end

function E.eq(a, b, what)
	if a ~= b then error((what or "value") .. ": expected " .. tostring(b) .. ", got " .. tostring(a), 2) end
end

function E.has(text, needle, what)
	if not string.find(text, needle, 1, true) then
		error((what or "text") .. " lacks " .. needle .. "\n--- text ---\n" .. tostring(text), 2)
	end
end

function E.lacks(text, needle, what)
	if string.find(text, needle, 1, true) then
		error((what or "text") .. " should not contain " .. needle .. "\n--- text ---\n" .. tostring(text), 2)
	end
end

function E.count(text, needle)
	local n, pos = 0, 1
	while true do
		local s, f = string.find(text, needle, pos, true)
		if not s then return n end
		n, pos = n + 1, f + 1
	end
end

function E.mod(id, over)
	local m = { id = id, version = "1.0.0", scope = "client", entry = "obsidian/mods/" .. id .. "/main", loader_api = 1 }
	for k, v in pairs(over or {}) do m[k] = v end
	return m
end

return E
