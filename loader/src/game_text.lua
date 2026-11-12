-- Game texts the loader replaces: LocalizationManager.string returns a loader text (in the game
-- language), or a mod's own words, for the chosen keys. That function runs for every text on screen,
-- every frame, so the wrapper stays lean instead of going through the protected hook path.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { map = {}, args = {}, when = {}, words = {}, installed = false, waiting = false }

-- A mod's words for a text key, in the game's language, else English, else any.
local function chosen(core, w)
	if B.type(w) ~= "table" then return w end
	local lang = core.language()
	if w[lang] then return w[lang] end
	if w.en then return w.en end
	local _, first = B.next(w)
	return first
end

local function install()
	local lm = B.rawget(B.G, "LocalizationManager")
	if B.type(lm) ~= "table" or B.type(lm.string) ~= "function" then return false end
	local original, map, args, when, words, core = lm.string, M.map, M.args, M.when, M.words, M.core
	lm.string = function(self, key, ...)
		local own = map[key]
		if own then
			local a = args[key]
			local w = when[key]
			if w then
				local ok, got = B.pcall(w)
				if not (ok and got) then return original(self, key, ...) end
				a = B.type(got) == "table" and got or nil
			end
			if a then return core.text(own, B.unpack(a)) end
			return core.text(own)
		end
		local w = words[key]
		if w then return chosen(core, w) end
		return original(self, key, ...)
	end
	M.installed = true
	log.line("hook installed: LocalizationManager.string")
	return true
end

local function ensure(core, hooks)
	M.core = core
	if M.installed or M.waiting or install() then return end
	M.waiting = true
	hooks.on_frame(function()
		if not M.installed then install() end
	end)
end

-- From then on the game key shows the loader text `own` from the shared catalog, filled with `args`.
function M.replace(core, hooks, key, own, args)
	M.map[key], M.args[key], M.when[key] = own, args, nil
	ensure(core, hooks)
end

-- The same, only while when() answers: true, or the args as a list. Otherwise the game's own text.
function M.replace_when(core, hooks, key, own, when)
	M.replace(core, hooks, key, own, nil)
	M.when[key] = when
end

-- obsidian.game_text: the key shows `words`, a string or a table of strings by language. nil takes
-- the words away. false for anything else.
function M.set(core, hooks, key, words)
	if B.type(key) ~= "string" or key == "" then return false end
	if words == nil then
		M.words[key] = nil
		return true
	end
	if B.type(words) == "table" then
		if B.next(words) == nil then return false end
		for lang, text in B.pairs(words) do
			if B.type(lang) ~= "string" or B.type(text) ~= "string" then return false end
		end
	elseif B.type(words) ~= "string" then
		return false
	end
	M.words[key] = words
	ensure(core, hooks)
	return true
end

return M
