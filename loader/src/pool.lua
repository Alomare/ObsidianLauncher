-- Matchmaking separation (spec 6.3, layer 1). build_info reads the game hash out of
-- BUILD_REVISION_INFORMATION after this chunk runs, so appending a tag to the hash there moves
-- the player into a pool that vanilla players never see.
local B = require("obsidian/loader/builtins")
local log = require("obsidian/loader/log")

local M = {}

-- Returns the tag that was applied, or nil for the vanilla pool. Players who run session content
-- or allow other players' session mods share one pool, so a consenting player's lobby shows up for
-- players with session mods, and players with session mods only find other Obsidian players.
-- Players who run mods every player needs get a pool of their own, shared only with players who
-- run the same ones.
function M.apply(deploy, consent_on)
	local hashes = deploy.hashes or {}
	local tag, pool
	if hashes.every then
		tag, pool = hashes.every, "every"
	elseif deploy.has_session or consent_on then
		tag, pool = hashes.open, "open"
	else
		log.line("pool: vanilla")
		return nil
	end
	if B.type(tag) ~= "string" or not B.match(tag, "^[0-9a-f]+$") then
		log.line("pool: bad tag for " .. pool .. ", left vanilla")
		return nil
	end
	local revision = B.rawget(B.G, "BUILD_REVISION_INFORMATION")
	local rewritten, n = B.gsub(revision, "(parent: [0-9]+:)([0-9a-f]+)", function(head, hash) return head .. hash .. tag end, 1)
	if n ~= 1 then
		log.line("pool: no hash in the revision string, left vanilla")
		return nil
	end
	B.rawset(B.G, "BUILD_REVISION_INFORMATION", rewritten)
	log.line("pool: " .. pool .. " " .. tag)
	return tag
end

return M
