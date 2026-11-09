-- War mode: a custom Galactic War from Obsidian Commander, played through the relay on a Sandbox.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local SANDBOX = "sandbox\tabcd1234\n"
local RELAY = "http://127.0.0.1:40123/0f1e2d3c4b5a69788796a5b4c3d2e1f0/1.0/"
local POOL = "a1b2c3d4e5f6"
local WAR = "war\tk3x9q2\t" .. RELAY .. "\t" .. POOL .. "\n"
local OFFICIAL = "https://api.helldiversgame.com/1.0/"

local UPLOADS = {
	"upload_mission_stats", "upload_leaderboard_stats", "upload_leaderboard_provinggrounds_stats",
	"complete_planets", "complete_defend_event_mission", "complete_attack_event_mission",
}

local function fake_save()
	local sm = { calls = {} }
	function sm:setup(system, file) self.calls[#self.calls + 1] = { system, file } end
	function sm:set_backup_system() end
	function sm:save(done) if done then done() end end
	function sm:user_save(done) if done then done() end end
	function sm:get() end
	function sm:set() end
	function sm:user_set() end
	return sm
end

local function boot(settings)
	rawset(_G, "SaveManager", fake_save())
	local e = E.new({ deploy = E.deploy({}), settings = settings })
	e:boot()
	e:start_frames()
	return e
end

-- The class appears when the game requires the script, after the loader booted.
local function load_campaign_manager(e)
	local cm = { sent = {}, feedbacks = 0 }
	function cm:set_environment() self.url = OFFICIAL end
	function cm:feedback() self.feedbacks = self.feedbacks + 1 end
	for _, m in ipairs(UPLOADS) do
		cm[m] = function(self) self.sent[#self.sent + 1] = m end
	end
	rawset(_G, "CampaignManager", cm)
	e.opts.modules["lua/managers/campaign_manager"] = true
	rawget(_G, "require")("lua/managers/campaign_manager")
	return cm
end

local function load_telemetry(e)
	local t = { queued = 0 }
	function t:queue_event() self.queued = self.queued + 1 end
	rawset(_G, "TelemetryWwsgaManager", t)
	e.opts.modules["lua/managers/telemetry_wwsga_manager"] = true
	rawget(_G, "require")("lua/managers/telemetry_wwsga_manager")
	return t
end

test("a_war_points_the_game_at_the_relay_and_lets_its_results_through", function()
	local e = boot(SANDBOX .. WAR)
	local cm = load_campaign_manager(e)
	for _, m in ipairs(UPLOADS) do cm[m](cm) end
	E.eq(#cm.sent, 0, "nothing goes out before the redirect is confirmed")
	cm:set_environment()
	E.eq(cm.url, RELAY)
	for _, m in ipairs(UPLOADS) do cm[m](cm) end
	E.eq(#cm.sent, #UPLOADS, "the war's results reach the relay")
	E.has(e:log(), "war: k3x9q2 through the relay")
	E.has(e:log(), "galactic war reporting: to the custom war through the relay")
end)

test("a_war_keeps_its_own_pool", function()
	boot(SANDBOX .. WAR)
	E.has(rawget(_G, "BUILD_REVISION_INFORMATION"), "3bd83d77c8ae" .. POOL, "the war's tag after the game hash")
end)

test("a_war_line_without_a_sandbox_is_refused", function()
	local e = boot(WAR)
	local cm = load_campaign_manager(e)
	cm:set_environment()
	E.eq(cm.url, OFFICIAL, "the official war stays")
	E.eq(e:core().war, nil)
	E.has(e:log(), "the war line is not valid or has no Sandbox")
end)

test("a_war_line_with_another_host_is_refused", function()
	for _, url in ipairs({ "http://10.0.0.5:40123/abc/1.0/", "https://127.0.0.1:1/abc/1.0/", "http://127.0.0.1:1/ABC/1.0/", "http://127.0.0.1:1/abc/2.0/" }) do
		local e = boot(SANDBOX .. "war\tk3x9q2\t" .. url .. "\t" .. POOL .. "\n")
		local cm = load_campaign_manager(e)
		cm:set_environment()
		E.eq(cm.url, OFFICIAL, url)
		E.eq(e:core().war, nil, url)
	end
end)

test("a_war_line_with_a_bad_id_or_pool_is_refused", function()
	for _, line in ipairs({
		"war\tk3/x\t" .. RELAY .. "\t" .. POOL .. "\n",
		"war\tk3x9q2\t" .. RELAY .. "\tnothex000000\n",
		"war\tk3x9q2\t" .. RELAY .. "\n",
	}) do
		local e = boot(SANDBOX .. line)
		E.eq(e:core().war, nil, line)
	end
end)

test("a_war_drops_analytics_and_the_tamper_report", function()
	local e = boot(SANDBOX .. WAR)
	local cm = load_campaign_manager(e)
	local t = load_telemetry(e)
	t:queue_event("mission_start")
	cm:feedback()
	E.eq(t.queued, 0)
	E.eq(cm.feedbacks, 0)
end)

test("a_war_puts_the_url_back_when_something_changes_it", function()
	local e = boot(SANDBOX .. WAR)
	local cm = load_campaign_manager(e)
	cm:set_environment()
	cm.url = OFFICIAL
	e:frames(31)
	E.eq(cm.url, RELAY)
	E.has(e:log(), "the war URL was changed and is set back")
end)

test("a_sandbox_without_a_war_still_blocks_uploads", function()
	local e = boot(SANDBOX)
	local cm = load_campaign_manager(e)
	cm:set_environment()
	E.eq(cm.url, OFFICIAL)
	for _, m in ipairs(UPLOADS) do cm[m](cm) end
	E.eq(#cm.sent, 0)
end)

test("a_war_writes_the_region_names_for_commander", function()
	local e = boot(SANDBOX .. WAR)
	local words = { region_bugs_01 = "Polaris Region", city_01 = "Sistema Ross", region_superearth_01 = "LOC:region_superearth_01" }
	rawset(_G, "LocalizationManager", { string = function(_, k) return words[k] or ("LOC:" .. k) end })
	rawset(_G, "CampaignSettings", { warfront_info = { warfronts = {
		bugs = { { name = "region_bugs_01", city_name = "city_01" } },
		superearth = { { name = "region_superearth_01", city_name = "city_superearth" } },
	} } })
	e:frames(301)
	local f = assert(io.open(e.obsidian_dir .. "/commander-names.txt", "rb"))
	local text = f:read("*a")
	f:close()
	E.has(text, "region_bugs_01\tPolaris Region\n")
	E.has(text, "city_01\tSistema Ross\n")
	E.lacks(text, "superearth", "a key the game has no words for stays out")
end)

return T
