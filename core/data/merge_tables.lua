-- Made at deploy when several mods change one of the game's generated tables. NAMES, set above this line,
-- are each mod's version of the table, in load order. They all start from the game's table, so the result
-- is everything any of them has. A list is sorted, as the game's are: unit_paths numbers each unit by its
-- place, those numbers cross the network, and sorting gives every player with these mods the same ones
-- whatever their load order. A map keeps the later mod's value for a key.
local out, paths, two_way = {}, {}, false
for _, name in ipairs(NAMES) do
	for key, value in pairs(require(name)) do
		if type(key) == "number" then
			if type(value) == "string" then paths[value] = true end
		elseif type(value) == "number" then
			paths[key] = true
			two_way = true
		else
			out[key] = value
		end
	end
end
local list = {}
for path in pairs(paths) do list[#list + 1] = path end
table.sort(list)
for i, path in ipairs(list) do
	out[i] = path
	if two_way then out[path] = i end
end
return out
