-- The game's tamper check, told in loader.log: which guarded value was written directly.
local E = require "fake_engine"
local T = {}
local function test(name, fn) T[#T + 1] = { name, fn } end

local TEXTS = { tamper_tripped = "tamper" }

-- A ProtectedValue as the game's: get answers the value and whether the copy is broken.
local function copy(v)
	local c = { v = v }
	function c.get(self) return self.v, false end
	function c.set(self, x) self.v = x end
	return c
end

-- BaseComponent and StateGame as the game's PC build has them, and the hash of a key.
local function game()
	local tripped = 0
	rawset(_G, "Unit", { material_id = function(key) return "#" .. key end })
	local Base = { protected_keys = {} }
	function Base.check_protected(self, state, key, hashed)
		hashed = hashed or self.protected_keys[key] or Unit.material_id(key)
		local v, broken = state[hashed]:get()
		if v ~= state[key] or broken then self.event_delegate.trigger("on_fusk_detected") end
	end
	function Base.set_protected(self, state, key, value)
		local hashed = self.protected_keys[key] or Unit.material_id(key)
		if state[hashed] == nil then
			state[hashed] = copy(value)
			state[key] = value
			return
		end
		self:check_protected(state, key, hashed)
		state[hashed]:set(value)
		state[key] = value
	end
	rawset(_G, "BaseComponent", Base)
	local Game = {}
	function Game.on_fusk_detected(self) tripped = tripped + 1 end
	function Game.set_extra_lives(self, player, n)
		local v = player.extra_lives_protected:get()
		if v ~= player.extra_lives then self:on_fusk_detected() end
		player.extra_lives_protected:set(n)
		player.extra_lives = n
	end
	rawset(_G, "StateGame", Game)
	local e = E.new({ deploy = E.deploy({ texts = { en = TEXTS } }) })
	e:boot()
	e:start_frames()
	-- A component instance, as class() makes it: methods through __index.
	local inventory = setmetatable({ name = "inventory", protected_keys = { cooldown_timer = "#cooldown_timer" } }, { __index = Base })
	inventory.event_delegate = { trigger = function() Game.on_fusk_detected({}) end }
	return e, inventory, function() return tripped end
end

local function notices(e)
	local out = {}
	for _, n in ipairs(e:core().notices) do out[#out + 1] = n.key end
	for _, n in ipairs(e:core().notices_shown or {}) do out[#out + 1] = n.key end
	return table.concat(out, ",")
end

test("a_value_set_through_the_setter_says_nothing", function()
	local e, inv, tripped = game()
	local s = {}
	inv:set_protected(s, "cooldown_timer", 30)
	inv:set_protected(s, "cooldown_timer", 29)
	E.eq(tripped(), 0)
	E.eq(e:log():find("tamper:", 1, true), nil, "nothing logged")
	E.eq(notices(e), "")
end)

test("a_value_written_directly_is_named_with_both_values", function()
	local e, inv, tripped = game()
	local s = {}
	inv:set_protected(s, "cooldown_timer", 30)
	s.cooldown_timer = 0
	inv:set_protected(s, "cooldown_timer", 29)
	E.eq(tripped(), 1, "the game's check still runs")
	E.has(e:log(), "tamper: inventory cooldown_timer is 0, the game's copy 30")
	E.has(e:log(), "tamper: stack:")
end)

test("a_key_without_a_registered_hash_uses_the_material_id", function()
	local e, inv = game()
	local s = {}
	inv:set_protected(s, "uses_left", 2)
	s.uses_left = 1
	inv:set_protected(s, "uses_left", 1)
	E.has(e:log(), "tamper: inventory uses_left is 1, the game's copy 2")
end)

test("extra_lives_written_directly_are_named", function()
	local e = game()
	local player = { extra_lives = 3, extra_lives_protected = copy(1) }
	rawget(_G, "StateGame"):set_extra_lives(player, 0)
	E.has(e:log(), "tamper: StateGame extra_lives is 3, the game's copy 1")
end)

test("the_trip_is_logged_and_told_once", function()
	local e = game()
	local G = rawget(_G, "StateGame")
	G:on_fusk_detected()
	G:on_fusk_detected()
	E.has(e:log(), "tamper: the game's tamper check went off")
	E.eq(notices(e), "tamper_tripped")
end)

test("a_state_the_check_cant_read_passes_through", function()
	local e, inv = game()
	pcall(inv.check_protected, inv, nil, "cooldown_timer")
	pcall(inv.check_protected, inv, {}, "cooldown_timer")
	E.eq(e:log():find("tamper: inventory", 1, true), nil, "nothing to name")
end)

return T
