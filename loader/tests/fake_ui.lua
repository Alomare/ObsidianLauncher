-- The parts of the game the menu touches: the brightness world, Gui, the input stacks and an
-- edge-triggered keyboard. pressed() is true for the one frame a key goes down, as in the game.
local F = {}

local KEYS = { "up", "down", "left", "right", "enter", "esc", "tab", "e", "q", "w", "a", "s", "d", "r", "left shift", "f6", "backspace" }

function F.install(e, opts)
	opts = opts or {}
	local ui = { rects = {}, texts = {}, focus = {}, interfaces = {}, down = {}, tapped = {}, state = opts.state or "game", destroyed = 0, typed = {} }
	e.ui = ui

	rawset(_G, "Vector3", function(x, y, z) return { x = x, y = y, z = z } end)
	rawset(_G, "Vector2", function(x, y) return { x = x, y = y } end)
	rawset(_G, "Color", function(a, r, g, b) return { a, r, g, b } end)
	-- What the title screen run of 2026-10-30 showed: a triangle with its points as (x, y, 0) draws
	-- nothing, so only (x, 0, y) lands in the list; text_extents gives min and max, and counts tracking.
	ui.triangles, ui.bitmaps, ui.guis = {}, {}, {}
	rawset(_G, "Gui", {
		rect = function(gui, pos, size, color) ui.rects[#ui.rects + 1] = { gui = gui, pos = pos, size = size, color = color } end,
		text = function(gui, text, font, size, material, pos, color, option, value)
			ui.texts[#ui.texts + 1] = { gui = gui, text = text, font = font, size = size, material = material, pos = pos, color = color, option = option, value = value }
		end,
		triangle = function(gui, a, b, c, layer, color)
			if a.y == 0 and b.y == 0 and c.y == 0 then
				ui.triangles[#ui.triangles + 1] = { gui = gui, a = a, b = b, c = c, layer = layer, color = color }
			end
		end,
		text_extents = function(_, text, _, size, option, value)
			local gaps = option == "letter_spacing" and (#text - 1) * value or 0
			return { x = 0, y = 0, z = 0 }, { x = #text * size * 0.5 + gaps, y = 0, z = size }
		end,
		has_all_glyphs = function(_, text) return not ui.missing_glyphs or not string.find(text, ui.missing_glyphs, 1, true) end,
		bitmap_uv = function(gui, name, uv0, uv1, pos, size, color)
			ui.bitmaps[#ui.bitmaps + 1] = { gui = gui, name = name, uv0 = uv0, uv1 = uv1, pos = pos, size = size, color = color }
		end,
	})
	rawset(_G, "World", {
		create_screen_gui = function(world, ...)
			local args = { world, ... }
			ui.gui_args = args
			local name = args[3]
			for i = 3, #args - 1, 2 do
				if args[i] == "fonts/fonts" then name = args[i] end
			end
			ui.guis[#ui.guis + 1] = name
			return "gui " .. tostring(name)
		end,
		destroy_gui = function() ui.destroyed = ui.destroyed + 1 end,
	})
	local application = rawget(_G, "Application")
	application.resolution = function() return ui.width or 1920, ui.height or 1080 end
	rawset(_G, "Script", { temp_count = function() return 1, 2, 3 end, set_temp_count = function() end })
	rawset(_G, "ENGINE_DT", 0.016)
	rawset(_G, "KEYBOARD_MOUSE_USER_NAME", "keyboard_mouse")
	rawset(_G, "ControllerSettings", { pad = {} })

	local boot = rawget(_G, "Boot")
	boot.brightness_world = "brightness"
	-- The game's StateMachine class, whose post_update the boot script calls at the end of every frame.
	local machine_class = { post_update = function() end }
	rawset(_G, "StateMachine", machine_class)
	boot.state_machine = setmetatable({
		get_current_state_name = function() return ui.state end,
		get_current_state = function() return ui.state_object or {} end,
	}, { __index = machine_class })

	rawset(_G, "InputManager", {
		has_interface = function(_, user, name) return ui.interfaces[user .. "/" .. name] == true end,
		create_controller_interface = function(_, user, name) ui.interfaces[user .. "/" .. name] = true end,
		take_focus = function(_, user, name) ui.focus[user] = name end,
		release_focus = function(_, user) ui.focus[user] = nil end,
	})
	rawset(_G, "PadManager", {
		users = { { name = "user1" } },
		create_rebindable_interface = function(_, name) ui.interfaces["user1/" .. name] = true end,
		get_inputs = function() return ui.pad or {} end,
	})

	local keyboard = rawget(_G, "Keyboard")
	keyboard.button_index = function(name)
		for i, n in ipairs(KEYS) do
			if n == name then return i end
		end
		return nil
	end
	keyboard.pressed = function(i) return ui.tapped[KEYS[i]] == true end
	keyboard.button = function(i) return (ui.down[KEYS[i]] or ui.tapped[KEYS[i]]) and 1 or 0 end

	-- The mouse, read raw. The cursor shares the gui's origin, the bottom left of the screen.
	ui.mouse = { x = 10, y = 10, wheel = 0, held = false }
	rawset(_G, "Mouse", {
		axis_index = function(name) return name end,
		button_index = function(name) return name end,
		axis = function(i)
			if i == "wheel" then return { x = 0, y = ui.mouse.wheel, z = 0 } end
			return rawget(_G, "Vector3")(ui.mouse.x, ui.mouse.y, 0)
		end,
		button = function(i) return ((i == "left" and ui.mouse.held) or (i == "right" and ui.mouse.right_held)) and 1 or 0 end,
		pressed = function(i) return ui.mouse[i] == true end,
	})

	-- The game's cursor manager, and the engine under it. system is true while the arrow is the
	-- operating system's, which an empty path gives. Anything but a string crashes the real engine.
	ui.cursor = { name = "hud/cursors/game", shown = true, clipped = true, system = false, refuse_system = false, crashed = false }
	rawset(_G, "Window", {
		set_cursor = function(path)
			if type(path) ~= "string" then ui.cursor.crashed = true end
			if path == "" and ui.cursor.refuse_system then error("set_cursor refused the path") end
			ui.cursor.system = path == ""
		end,
	})
	rawset(_G, "CursorManager", {
		set_cursor = function(_, name) ui.cursor.name, ui.cursor.shown, ui.cursor.system = name, true, false end,
		get_current_cursor = function() return ui.cursor.name end,
		set_show_cursor = function(_, on)
			if ui.cursor.shown ~= on then ui.cursor.shown, ui.cursor.system = on, false end
		end,
		is_shown = function() return ui.cursor.shown end,
		set_clip_cursor = function(_, on) ui.cursor.clipped = on end,
	})

	-- A frame with the cursor here.
	function ui.move(x, y)
		ui.mouse.x, ui.mouse.y = x, y
		ui.frame()
	end

	-- A frame with a button going down, which stays held until ui.release().
	function ui.press(x, y, button)
		ui.mouse.x, ui.mouse.y = x, y
		button = button or "left"
		ui.mouse[button] = true
		ui.mouse[button == "left" and "held" or "right_held"] = true
		ui.frame()
		ui.mouse[button] = false
	end

	function ui.release()
		ui.mouse.held, ui.mouse.right_held = false, false
		ui.frame()
	end

	function ui.click(x, y, button)
		ui.press(x, y, button)
		ui.release()
	end

	-- A frame with the wheel turned, positive away from the player.
	function ui.scroll(amount)
		ui.mouse.wheel = amount
		ui.frame()
		ui.mouse.wheel = 0
		ui.frame()
	end

	-- A text drawn in the last frame, by part of it, or by all of it when exact.
	function ui.find(needle, exact)
		for _, t in ipairs(ui.texts) do
			if (exact and t.text == needle) or (not exact and string.find(t.text, needle, 1, true)) then return t end
		end
		error("not drawn: " .. needle, 2)
	end

	-- One frame with these keys going down.
	function ui.tap(...)
		for _, k in ipairs({ ... }) do ui.tapped[k] = true end
		ui.rects, ui.texts = {}, {}
		e:frames(1)
		ui.tapped = {}
	end

	function ui.frame()
		ui.rects, ui.texts, ui.triangles, ui.bitmaps = {}, {}, {}, {}
		e:frames(1)
	end

	function ui.shown(needle)
		for _, t in ipairs(ui.texts) do
			if string.find(t.text, needle, 1, true) then return true end
		end
		return false
	end

	-- A stand-in for the game's text field. Keys typed since the last frame arrive in ui.typed.
	rawset(_G, "GuiItems", {
		InputField = {
			do_input = function(text)
				local out, finished, cancelled = text, false, false
				for _, k in ipairs(ui.typed) do
					if k == "enter" then
						finished = true
					elseif k == "esc" then
						finished, cancelled = true, true
					else
						out = out .. k
					end
				end
				ui.typed = {}
				return out, out ~= text, finished, cancelled
			end,
		},
	})
	-- The game requires modules all the time, and the loader puts waiting hooks in at each, such as one on
	-- the StateMachine just made.
	pcall(rawget(_G, "require"), "lua/network/network_global")
	return ui
end

return F
