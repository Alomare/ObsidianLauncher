-- Icons drawn from shapes, so a mod can use them with no art of its own and they stay sharp at any
-- size. Each is a list of convex polygons on a grid of 16, with y up, as flat point lists.
local B = require("obsidian/loader/builtins")

local cos, sin, pi = math.cos, math.sin, math.pi

-- A line as a quad.
local function line(ax, ay, bx, by, width)
	local dx, dy = bx - ax, by - ay
	local length = (dx * dx + dy * dy) ^ 0.5
	local nx, ny = -dy / length * width / 2, dx / length * width / 2
	return { ax + nx, ay + ny, ax - nx, ay - ny, bx - nx, by - ny, bx + nx, by + ny }
end

-- A filled circle, as sides.
local function disc(cx, cy, r, sides)
	local out = {}
	for i = 0, (sides or 8) - 1 do
		local a = (i + 0.5) * 2 * pi / (sides or 8)
		out[#out + 1] = cx + r * cos(a)
		out[#out + 1] = cy + r * sin(a)
	end
	return out
end

-- Part of a hollow circle, as one quad per side, between two angles.
local function arc(cx, cy, r, width, sides, from, to)
	local out, inner = {}, r - width
	for i = 0, sides - 1 do
		local a, b = from + i * (to - from) / sides, from + (i + 1) * (to - from) / sides
		out[#out + 1] = { cx + inner * cos(a), cy + inner * sin(a), cx + r * cos(a), cy + r * sin(a),
			cx + r * cos(b), cy + r * sin(b), cx + inner * cos(b), cy + inner * sin(b) }
	end
	return out
end

-- A hollow circle. turn: the angle of the first corner.
local function ring(cx, cy, r, width, sides, turn)
	return arc(cx, cy, r, width, sides, turn or 0, (turn or 0) + 2 * pi)
end

local function join(...)
	local out = {}
	for _, part in B.ipairs({ ... }) do
		if B.type(part[1]) == "table" then
			for _, poly in B.ipairs(part) do out[#out + 1] = poly end
		else
			out[#out + 1] = part
		end
	end
	return out
end

local function chevron(y)
	return join({ 2, y, 8, y + 4, 8, y + 6.2, 2, y + 2.2 }, { 8, y + 4, 14, y, 14, y + 2.2, 8, y + 6.2 })
end

local function rays()
	local out = {}
	for i = 0, 7 do
		local a = i * pi / 4
		out[#out + 1] = line(8 + 5.2 * cos(a), 8 + 5.2 * sin(a), 8 + 7.6 * cos(a), 8 + 7.6 * sin(a), 1.5)
	end
	return out
end

return {
	shield = { { 8, 15.5, 2, 13, 2, 7, 8, 0.5, 14, 7, 14, 13 } },
	player = { disc(8, 11.4, 3.2), { 2.5, 1, 13.5, 1, 12, 6.6, 4, 6.6 } },
	bots = { disc(5.8, 11, 2.8), { 1, 1, 10.6, 1, 9.4, 5.9, 2.2, 5.9 }, disc(11.6, 12.6, 2.2), { 11.6, 5.4, 15.4, 5.4, 14.6, 9.2, 11.8, 9.2 } },
	weapons = join(ring(8, 8, 5.4, 1.5, 12), line(8, 0.5, 8, 4.6, 1.5), line(8, 11.4, 8, 15.5, 1.5), line(0.5, 8, 4.6, 8, 1.5), line(11.4, 8, 15.5, 8, 1.5), disc(8, 8, 1.2)),
	stratagems = join(ring(8, 8, 7.4, 1.6, 6, pi / 2), { 8, 12.2, 11.2, 8.6, 4.8, 8.6 }, line(8, 8.9, 8, 3.8, 2)),
	enemies = join(disc(8, 8.5, 3.4), disc(8, 13.2, 1.9), { 5.4, 6, 10.6, 6, 8, 1.2 }, line(4.8, 9.6, 1, 12.6, 1.3), line(11.2, 9.6, 15, 12.6, 1.3),
		line(4.6, 7.6, 0.8, 7, 1.3), line(11.4, 7.6, 15.2, 7, 1.3), line(5, 5.6, 1.8, 2, 1.3), line(11, 5.6, 14.2, 2, 1.3)),
	mission = join(line(3.5, 0.5, 3.5, 15.5, 1.6), { 4.3, 15, 14, 11, 4.3, 7 }),
	world = join(disc(8, 8, 3.5), rays()),
	editor = join(line(4.6, 4.6, 12.4, 12.4, 3.6), { 1.4, 1.4, 5.8, 2.6, 2.6, 5.8 }),
	progress = join(chevron(1.4), chevron(7)),
	console = join(line(2, 12.6, 7, 8, 1.8), line(7, 8, 2, 3.4, 1.8), line(8.4, 3, 14.2, 3, 1.8)),
	changes = join(line(5.4, 13, 15, 13, 1.7), line(5.4, 8, 15, 8, 1.7), line(5.4, 3, 15, 3, 1.7), disc(2.2, 13, 1.1), disc(2.2, 8, 1.1), disc(2.2, 3, 1.1)),
	vehicles = join({ 1, 3.4, 15, 3.4, 13.4, 7.4, 2.6, 7.4 }, { 4.6, 7.4, 11, 7.4, 10, 10.8, 5.6, 10.8 }, line(10, 9.4, 15.6, 11.2, 1.3), line(2, 2, 14, 2, 1.6)),
	spawn = join({ 2, 1, 14, 1, 14, 7.4, 2, 7.4 }, { 4.2, 11.8, 11.8, 11.8, 8, 8.2 }, line(8, 15.5, 8, 11.4, 2.2)),
	page = join(line(3, 1.4, 13, 1.4, 1.5), line(3, 14.6, 13, 14.6, 1.5), line(3, 1.4, 3, 14.6, 1.5), line(13, 1.4, 13, 14.6, 1.5), line(5.6, 10.4, 10.4, 10.4, 1.4), line(5.6, 6, 10.4, 6, 1.4)),
	back = join(line(10.6, 2.4, 5, 8, 2.1), line(5, 8, 10.6, 13.6, 2.1)),
	forward = join(line(5.4, 2.4, 11, 8, 2.1), line(11, 8, 5.4, 13.6, 2.1)),
	close = join(line(3, 3, 13, 13, 2), line(3, 13, 13, 3, 2)),
	search = join(ring(6.8, 9.2, 5, 1.7, 10), line(10.2, 5.8, 14.6, 1.4, 2.1)),
	run = { { 4, 2, 13.6, 8, 4, 14 } },
	edit = join(line(2, 2, 14, 2, 1.6), line(4.6, 6, 11, 12.4, 2.6)),
	reset = join(arc(8.4, 7.4, 6.4, 1.7, 14, pi * 1.14, pi * 2.75), { 2.07, 8.92, 6.53, 9.27, 2.43, 13.37 }),
}
