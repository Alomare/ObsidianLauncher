-- Game texts the loader replaces: LocalizationManager.string returns a loader text (in the game
-- language), or a mod's own words, for the chosen keys. That function runs for every text on screen,
-- every frame, so the wrapper stays lean instead of going through the protected hook path.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = { map = {}, args = {}, when = {}, words = {}, caps = {}, installed = false, waiting = false }

-- A text in capitals, as the game draws titles: the engine's Utf8.upper knows letters beyond ASCII.
-- Without it only a to z change, so the bytes of other letters are never split.
function M.upper(text)
	if B.type(text) ~= "string" then return text end
	local utf8 = B.rawget(B.G, "Utf8")
	if B.type(utf8) == "table" and B.type(utf8.upper) == "function" then
		local ok, out = B.pcall(utf8.upper, text)
		if ok and B.type(out) == "string" then return out end
	end
	return (B.gsub(text, "[a-z]+", B.upper))
end

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
	local original, map, args, when, words, caps, core = lm.string, M.map, M.args, M.when, M.words, M.caps, M.core
	local upper = M.upper
	local function look(self, key, ...)
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
	lm.string = function(self, key, ...)
		local from = caps[key]
		if from == nil then return look(self, key, ...) end
		return upper(look(self, from, ...))
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

-- From then on the key shows in capitals the text of `from` (the key itself when nil), whatever
-- gives that text: a loader text, a mod's words or the game's own.
function M.capitals(core, hooks, key, from)
	M.caps[key] = from or key
	ensure(core, hooks)
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
