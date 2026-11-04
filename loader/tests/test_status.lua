-- The message the player gets the first time they load into the ship, about who they can play with.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local TEXTS = { status_clear = "clear", status_session = "session", status_isolated = "isolated" }

-- Boots with the deploy over, a StateMenu to enter, and the toast the notice falls back to.
local function ship(over)
	local entered = 0
	rawset(_G, "StateMenu", { on_enter = function() entered = entered + 1 return "entered" end })
	local d = { texts = { en = TEXTS } }
	for k, v in pairs(over) do d[k] = v end
	local e = E.new({ deploy = E.deploy(d) })
	e:boot()
	e:start_frames()
	return e, function() return rawget(_G, "StateMenu"):on_enter() end, function() return entered end
end

local function notices(e)
	local out = {}
	for _, n in ipairs(e:core().notices) do out[#out + 1] = n.key end
	for _, n in ipairs(e:core().notices_shown or {}) do out[#out + 1] = n.key end
	return table.concat(out, ",")
end

test("nothing_is_said_before_the_ship", function()
	local e = ship({ has_session = false })
	E.eq(notices(e), "", "the title screen and main menu stay quiet")
end)

test("client_mods_only_say_the_player_can_play_with_anyone", function()
	local e, enter, entered = ship({ has_session = false })
	E.eq(enter(), "entered", "the game's own on_enter runs and answers")
	E.eq(entered(), 1)
	E.eq(notices(e), "status_clear")
	E.has(e:log(), "status: status_clear")
end)

test("session_mods_say_they_switch_off_in_some_public_lobbies", function()
	local e, enter = ship({ has_session = true })
	enter()
	E.eq(notices(e), "status_session")
end)

test("content_that_cant_switch_off_says_it_keeps_vanilla_out_of_public_lobbies", function()
	local e, enter = ship({ has_session = true, has_static_session = true })
	enter()
	E.eq(notices(e), "status_isolated")
end)

test("only_the_first_ship_says_it", function()
	local e, enter, entered = ship({ has_session = true })
	enter()
	enter()
	E.eq(entered(), 2)
	E.eq(notices(e), "status_session", "back from a mission, nothing new")
end)

return T
