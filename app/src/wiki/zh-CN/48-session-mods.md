---
title: 编写会话模组
group: 制作模组
summary: 如何改变共享游戏，同时让防护继续生效。
---

会话模组改变的是其他玩家能注意到的内容：伤害、敌人、生成、任务。加载器决定这样的模组何时可以运行（[联机游戏](wiki:multiplayer)），并在玩家进出时开启和关闭它。模组要做的就是通过加载器进行每一项修改，使加载器能够关闭它们。

如果模组的所有修改都发生在独立的内容中（例如有单独任务界面的游戏模式），它可以做成[扩展](#expansions)，不影响其他任何任务。

## 设置作用范围 {#set-the-scope}

```json
"scope": "session"
```

省略该字段会让模组成为未指定模组，以客户端模组运行。只有当模组所做的一切都不会传到其他玩家时，才使用 `client`。

## 通过挂钩修改游戏 {#change-the-game-through-hooks}

房间未获准时，会话模组的挂钩会自动调用游戏的原始代码。完全由挂钩组成的模组无需额外代码就是安全的：

```lua
-- Double the damage of every shot, in cleared lobbies only.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)
```

`on_frame` 函数和按键也一样：在未获准的房间中它们不会运行。

## 撤销残留的修改 {#undo-what-stays-behind}

有些修改会在发出它的调用结束后继续存在：写入设置表的值、生成的单位。请监听开关，并把东西恢复原样：

```lua
local original_speed

obsidian.on_require("lua/settings/player_settings", function(settings)
  original_speed = settings.run_speed
end)

obsidian.on_session(function(on)
  local settings = rawget(_G, "PlayerSettings")
  if settings and original_speed then
    settings.run_speed = on and original_speed * 1.5 or original_speed
  end
end)
```

## 混合作用范围 {#hybrid-scopes}

会话模组常常同时有只有本玩家能看到的代码（例如镜头）和改变共享游戏的代码。保持 `session` 为主作用范围，声明 [`secondary_scopes`](wiki:manifest#hybrid-scopes) 并为各部分代码加上标记，这样房间不再获准时，个人代码会继续运行：

```json
"scope": "session",
"secondary_scopes": [
  { "id": "view", "scope": "client", "name": { "en": "Camera" } },
  { "id": "weapon_range", "scope": "session", "option": "max_weapon_range", "name": { "en": "Max weapon range" } }
]
```

```lua
local VIEW = { secondary_scope = "view" }
obsidian.hook("CameraManager", "update", update_camera, VIEW)
obsidian.on_frame(steer, VIEW)
obsidian.hook("WeaponSettings", "range", longer_range, { secondary_scope = "weapon_range" })
obsidian.on_session(function(on) restore_range(on) end, { secondary_scope = "weapon_range" })
```

- 客户端次要作用范围遵循客户端模组的规则：只涉及本玩家看到、听到或按下的内容。镜头是个人的；时间流速和重力则不是，即使它们只在一台电脑上运行。
- 会话次要作用范围关闭时，游戏会告诉玩家是哪些，并说明其余部分仍然有效。
- 带有 `option` 的次要作用范围只在该开关开启时才开启。所有会话次要作用范围都关闭时，模组以客户端模组部署，玩家会重新获得银河战争结果。
- 会话次要作用范围开启时，模组所需的库以会话模组运行，并在其保护的调用中把整个模组视为会话模组。
- 所有次要作用范围共用一个 Lua 状态。客户端次要作用范围中的代码仍可直接访问会话代码，所以请每次都读取会话代码修改的内容，而不要保留副本。
- 扩展也可以声明会话次要作用范围，用于在内容之外、在已获准的房间中也会运行的代码。扩展中未标记的代码归属内容本身。
- [Loader API：混合作用范围](wiki:loader-api#hybrid-scopes)列出了相关参数。

## 应避免的做法 {#things-to-avoid}

- **直接替换全局函数。** `SomeTable.func = mine` 会在每个房间中继续运行，因为加载器无法关闭它。加载器日志会在启动时为每个模组列出此类修改。
- **为了玩法修改而用资源替换游戏脚本。** 被替换的脚本同样无法实时关闭。Obsidian 会接受，但在有玩家阻止会话模组的公开房间中，其他会话模组关闭时，这项修改仍留在游戏中。
- **在游戏代码之外向其他玩家发送自定义网络消息。**

> [!NOTE]
> 允许修改运行该模组的玩家自己的进度，例如经验值或解锁。Obsidian 会在每次部署前备份存档，并且不会撤销这些修改。

## 编写库 {#libraries}

专门供其他模组依赖的模组可以声明自己为库：

```json
"scope": "library"
```

每次部署时，如果某个已启用的会话模组直接或通过另一个库需要它，Obsidian 就会把它作为会话模组。否则它是客户端模组。它作为会话模组所做的一切都遵循通常规则，并加入玩家之间比较的组合。用 `resources` 替换游戏文件的库在两种情况下都会保留这些文件，所以请不要把玩法修改放在其中。

库的代码必须在两种情况下都能工作。`obsidian.scope()` 说明本次部署给出的是哪一种，`obsidian.session.guard` 对库交给其他模组的函数进行分类。请列出对任何人都安全的函数：读取、撤销，以及只有本玩家能看到或听到的修改。库以客户端模组运行时，其他所有函数返回 `false, "needs_session"`，在未获准的房间中返回 `false, "not_cleared"`。未列出的函数会被拒绝。

```lua
local M = obsidian.module("lib/enemies")

-- The gameplay hooks wait for a session mod that needs them.
if obsidian.scope() == "session" then
  obsidian.hook("Damage", "apply", M.on_damage)
end

return {
  enemies = obsidian.session.guard(M, { "list", "count" }),
}
```

库本身保留着真正的表，所以在房间不再获准后仍然可以撤销它所做的修改。

## 所有玩家都需要的模组 {#every-player}

玩家的 `consent` 告诉模组该玩家允许它，而不是该玩家在运行它。对大多数模组来说这已经足够，因为由主机的游戏决定发生什么。只有当房间中每个游戏都有它时才能工作的模组（例如改变房间可容纳人数的模组）可以提出更高要求：

```json
"scope": "session",
"every_player": true
```

这样的模组只有在其他每位玩家也运行它、版本和共享选项值都相同时才会运行。仅仅允许它的玩家不算数。Obsidian 会像对待任何会话模组一样关闭和重新开启它，并告诉玩家原因。

`obsidian.session.cleared()` 为发出询问的模组作答，`obsidian.on_session` 在该模组自身的开关变化时调用模组。

检查在玩家进入房间后才运行，而任何人都可以先加入，所以在所有玩家得到确认之前，请保持游戏原样：对于更大的房间，在 `obsidian.session.cleared()` 为 true 之前保持通常的人数上限，并在 `on_session` 返回 `false` 时恢复该上限。

## 编写扩展 {#expansions}

扩展是独立于正常游戏的内容，例如街机模式。在内容之外，它视为客户端模组：玩家保留正常匹配、银河战争和 Proving Grounds。只有运行过该内容的任务会脱离共享战争（[作用范围](wiki:scopes#expansions)）。

```json
"scope": "expansion",
"min_obsidian": "1.2.0",
"requires": ["obsidian.api"]
```

扩展不搭配 `every_player`：内容本来就只在所有玩家都运行该扩展时才会运行。

### 何时运行什么 {#what-runs-when}

| | 内容之外 | 内容运行期间 |
|---|---|---|
| `obsidian.hook`、`obsidian.on_frame` 和按键 | 关闭：挂钩只调用原函数 | 开启，在房间的每个游戏中 |
| `obsidian.wrap`，以及带 `opts.always` 的按键 | 开启 | 开启 |
| `obsidian.ui`、菜单页面和覆盖层 | 开启 | 开启 |
| 库的受保护函数，例如 Obsidian API 的大部分函数 | `false, "not_started"` | 完整使用 |
| `obsidian.session.cleared()` | `false` | `true` |

进入内容的入口（菜单页面或用 `obsidian.ui` 绘制的界面）在任何房间中都可用。内容本身放在挂钩中。

### 启动和停止 {#starting-and-stopping}

```lua
-- An Obsidian from before expansions would run every hook in every lobby.
if not obsidian.capabilities.expansion then return end

local api = obsidian.import("obsidian.api")

api.menu.add("Arcade", function(page)
  page:action("Start", function()
    local ok, why, names = obsidian.expansion.start()
    if ok then return "Started" end
    if why == "players_missing" then return "Missing: " .. table.concat(names, ", ") end
    return why
  end)
  page:action("Stop", function() obsidian.expansion.stop() end)
end)

-- Only while the content runs, in every game in the lobby.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)

obsidian.on_session(function(on)
  if on then set_up() else undo() end
end)
```

`obsidian.expansion.start()` 在主机的游戏中运行。它会检查房间中的其他每位玩家是否运行同一扩展、同一版本以及相同的共享选项值。然后内容开始，房间中的其他所有游戏会在大约一秒内跟上。`obsidian.on_session(true)` 会在每个游戏中运行，内容结束时运行 `obsidian.on_session(false)`，所以这两处就是进行准备和撤销的地方。其他游戏只会通过这种方式从主机得知内容。

返回值为 `true`，或 `false` 和原因：

| 原因 | 时机 |
|---|---|
| `"players_missing"` | 房间中有人没有运行它。第三个值列出这些名字，主机会在通知中看到 |
| `"not_host"` | 房间的主机是另一位玩家 |
| `"busy"` | 另一个扩展的内容正在运行。一次只能运行一个 |
| `"not_expansion"` | 调用它的模组不是扩展 |

`obsidian.expansion.stop()` 在主机的游戏中结束内容，内容未在运行时返回 `false, "not_running"`。离开房间也会结束内容。主机离开时，接管房间的玩家会继续该内容。`obsidian.expansion.running()` 说明调用它的模组的内容当前是否在运行，`running(id)` 用于询问另一个模组。

### 加入的玩家 {#players-who-join}

内容运行期间，主机的房间在匹配中只对运行同一版本该扩展的玩家可见。没有该扩展的玩家通过邀请或好友尝试加入时，会在加载前被拒绝，主机会看到其名称。如果有人仍然在没有它的情况下进入了任务，会被游戏自带的踢出功能移除，并可以在内容结束后回来。玩家的加入或离开永远不会为已在内容中的玩家关闭内容。

### 扩展所需的库 {#libraries-an-expansion-requires}

扩展所需的库在部署时视为客户端模组，除非还有会话模组需要它。`obsidian.scope()` 在其中返回 `"session"`，以便库准备好扩展可以使用的一切，其余时间它像客户端库一样运行。内容运行期间，库的受保护函数对扩展可用，`obsidian.on_session` 会告诉库内容何时开始和结束，使它可以撤销所做的修改。使用同一个库的客户端模组仍会从这些函数得到 `"needs_session"`。

### 须知事项 {#things-to-know}

- 用 `resources` 替换的文件始终在游戏中，无论内容是否运行。请把它们限制在客户端模组也可以改变的范围内，例如字体和图片。
- 共享选项值会计入，与会话模组相同。请把个人选项标记为 `"scope": "client"`。
- 运行过该内容的任务结果永远不会进入银河战争，即使内容在结算界面之前就停止了。下一场不含该内容的任务会重新计入。
- `min_obsidian` 会阻止 Obsidian 1.1 部署该模组，并要求玩家更新。Obsidian 1.0 不读取它，会把模组作为未指定模组运行，所以入口脚本顶部的功能检查会在那种情况下让模组不进入游戏。

## 会话模组中的选项 {#options-in-session-mods}

Obsidian 比较玩家的组合时，每个选项值都会计入，除非将其标记为 `"scope": "client"` 或放在客户端次要作用范围中。扩展也是如此。请标记只改变本玩家所见内容的选项（[选项和按键](wiki:options-and-keys#personal-options-in-session-mods)）。
