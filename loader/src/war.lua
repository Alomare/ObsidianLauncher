-- War mode: a custom Galactic War from Obsidian Commander. With a war line in the settings file and
-- that war Sandbox save selected, the game sends war requests to the Obsidian relay on this PC, which
-- carries them to the war server without the Steam ticket. The analytics events and the in-game
-- tamper report don't go out at all. The war players keep a separate matchmaking pool (pool.lua
-- takes the tag). Without a Sandbox, the war line is refused and the game stays on the official war:
-- war results never touch the Official save.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { active = false, confirmed = false, line = nil, names_written = false }

-- Seconds between war status requests after an answer (the game waits 60).
M.POLL = 5

-- The relay is always on loopback, with the launch token in the path.
local URL_PATTERN = "^http://127%.0%.0%.1:%d+/[0-9a-f]+/1%.0/$"

-- A war line from the settings file, checked. Nil when any field is off.
function M.parse(fields)
	if #fields ~= 4 then return nil end
	local id, url, pool = fields[2], fields[3], fields[4]
	if not (B.match(id, "^[%w_%-]+$") and #id <= 32) then return nil end
	if not B.match(url, URL_PATTERN) then return nil end
	if not (B.match(pool, "^[0-9a-f]+$") and #pool == 12) then return nil end
	return { id = id, url = url, pool = pool }
end

-- The pool tag of the war this launch plays, or nil.
function M.pool_tag(core)
	return M.active and core.war and core.war.pool or nil
end

-- obsidian.war(): the custom war this launch plays, as { id, model }, or nil on the official war. The
-- model is "vanilla" or "enhanced", for the companion mod of an Enhanced war. Never the relay address.
function M.api(core)
	return function()
		if not (M.active and core.war) then return nil end
		return { id = core.war.id, model = core.war_model or "vanilla" }
	end
end

-- Whether the in-game war requests can only reach the relay: uploads then belong to the war.
function M.reporting(core)
	return M.active and M.confirmed and core.war ~= nil
end

-- A war plays a game season far above the official ones (the lobby browser pairs players of one season
-- only), while Commander counts the war seasons from 1. The war server puts `shown_season` next to
-- `season` in the campaign status rows; the difference is the offset, the same for every season of a war.
-- Nil when the rows carry no such field, or one that makes no sense.
function M.offset_from(rows)
	local row = B.type(rows) == "table" and rows[1] or nil
	if B.type(row) ~= "table" then return nil end
	local season, shown = B.tonumber(row.season), B.tonumber(row.shown_season)
	if not (season and shown) or shown < 1 or shown > season or B.floor(shown) ~= shown or B.floor(season) ~= season then return nil end
	return season - shown
end

-- A season number as the war counts it. A number at or under the offset belongs to no season of the
-- war and stays as it is, like anything that is not a number. A string stays a string.
function M.shown(value, offset)
	local n = B.tonumber(value)
	if not (offset and offset > 0 and n and n > offset) then return value end
	if B.type(value) == "string" then return B.tostring(n - offset) end
	return n - offset
end

-- The arguments of a LocalizationManager.string_format call with each value after "#SEASON" as the
-- war counts it, and their count, or nil when no "#SEASON" is there. offset() runs only then.
function M.season_args(offset, ...)
	local n = B.select("#", ...)
	local a, o
	for i = 1, n - 1 do
		if B.select(i, ...) == "#SEASON" then
			if not a then
				a, o = { ... }, offset()
				if not o then return nil end
			end
			a[i + 1] = M.shown(a[i + 1], o)
		end
	end
	return a, n
end

-- The offset of the war played now, from the campaign status the game keeps. The last one known stays
-- while the status is missing.
local function current_offset()
	local cm = B.rawget(B.G, "CampaignManager")
	local cache = B.type(cm) == "table" and cm.cache or nil
	local o = M.offset_from(B.type(cache) == "table" and cache.status or nil)
	if o and o ~= M.offset then
		M.offset = o
		log.line("war: the game texts show the war season, game season minus " .. o)
	end
	return M.offset
end

-- The region and city names in the player language, for the Commander map: once a launch, as soon as
-- the game text and settings are there. Plain key<TAB>words lines.
local function write_names(game_side)
	if M.names_written or not game_side then return end
	local lm, cs = B.rawget(B.G, "LocalizationManager"), B.rawget(B.G, "CampaignSettings")
	if B.type(lm) ~= "table" or B.type(cs) ~= "table" or B.type(cs.warfront_info) ~= "table" then return end
	local keys, seen = {}, {}
	local function add(k)
		if B.type(k) == "string" and not seen[k] then
			seen[k] = true
			keys[#keys + 1] = k
		end
	end
	for _, list in B.pairs(cs.warfront_info.warfronts or {}) do
		for _, region in B.ipairs(list) do
			add(region.name)
			add(region.city_name)
		end
	end
	if #keys == 0 then return end
	local lines = {}
	for _, k in B.ipairs(keys) do
		local ok, words = B.pcall(function() return lm:string(k) end)
		if ok and B.type(words) == "string" and words ~= "" and not B.match(words, "^LOC:") then
			lines[#lines + 1] = k .. "\t" .. B.gsub(words, "[\t\r\n]", " ")
		end
	end
	M.names_written = true
	local f = B.open(game_side .. "/commander-names.txt", "wb")
	if not f then return end
	f:write(B.concat(lines, "\n"), "\n")
	f:close()
	log.line("war: wrote " .. #lines .. " region names")
end

function M.setup(core, hooks, game_side)
	local w = core.war
	if not w then return end
	if not core.sandbox then
		-- settings.lua drops a war line with no Sandbox; this is the second lock.
		log.line("war: no Sandbox save selected, war mode refused")
		core.war = nil
		return
	end
	M.active = true
	log.line("war: " .. w.id .. " through the relay")
	hooks.loader_hook("CampaignManager", "set_environment", function(original, self, ...)
		local function done(...)
			self.url = w.url
			M.confirmed = true
			log.line("war: game war requests go to the relay")
			return ...
		end
		return done(original(self, ...))
	end)
	-- The game asks for the war status 60 s after each answer, so a Game Master change took up to a
	-- minute to show. The relay is on this PC: the war asks again sooner. A failure keeps the game's
	-- own retry times.
	hooks.loader_hook("CampaignManager", "update_status_callback", function(original, self, ...)
		local function done(...)
			if self.status == "online" and B.type(self.update_timer) == "number" and self.update_timer > M.POLL then
				self.update_timer = M.POLL
			end
			return ...
		end
		return done(original(self, ...))
	end)
	-- The in-game tamper report goes straight to a queue the upload hooks never see.
	hooks.loader_hook("CampaignManager", "feedback", function()
		log.limited("war feedback", "war: the tamper report stays home")
	end)
	-- Analytics go to another host, which a custom war has no business reaching.
	hooks.loader_hook("TelemetryWwsgaManager", "queue_event", function() end)
	-- The season number on screen: the recap date on the ship ("recap_date") and the title of the
	-- season end statistics ("season_end_statistics_season_title") are the only game texts with
	-- "#SEASON". Only the words change: the game season itself still goes to the lobby, the requests
	-- and the save.
	hooks.loader_hook("LocalizationManager", "string_format", function(original, self, key, ...)
		local a, n = M.season_args(current_offset, ...)
		if a then return original(self, key, B.unpack(a, 1, n)) end
		return original(self, key, ...)
	end)
	local frames = 0
	hooks.on_frame(function()
		frames = frames + 1
		if frames % 30 ~= 0 then return end
		local cm = B.rawget(B.G, "CampaignManager")
		if B.type(cm) == "table" and cm.url ~= nil and cm.url ~= w.url then
			-- Something set the URL again: war requests must never reach another server.
			cm.url = w.url
			log.limited("war url", "war: the war URL was changed and is set back")
		end
		if not M.names_written and frames % 300 == 0 then B.pcall(write_names, game_side) end
	end)
end

return M
