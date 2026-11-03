---
title: 编写会话模组
group: 制作模组
summary: 如何修改共享游戏，同时让防护措施继续为你工作。
---

会话模组会改变其他玩家能察觉的内容：伤害、敌人、生成、任务。加载器决定这类模组何时可以运行（[联机游戏](wiki:multiplayer)），并随玩家进出开启或关闭它。你要做的是让每项更改都经过加载器，这样它才能被关闭。

如果模组的所有更改都发生在一种独立的内容中，例如带有单独任务界面的游戏模式，它也可以改为[扩展模组](#expansions)，让其他所有任务保持原样。

## 设置作用范围 {#set-the-scope}

```json
"scope": "session"
```

省略该字段会让模组成为未指定模组，作为客户端模组运行。只有当模组所做的一切都不会影响其他玩家时，才使用 `client`。

## 通过挂钩修改游戏 {#change-the-game-through-hooks}

大厅未获许可时，会话模组的挂钩会自动调用游戏原本的代码。完全由挂钩构成的模组不需要额外代码就是安全的：

```lua
-- 每次射击的伤害翻倍，仅在获许可的大厅中生效。
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)
```

`on_frame` 函数和按键也一样：在未获许可的大厅中，它们不会运行。

## 撤销残留的更改 {#undo-what-stays-behind}

有些更改在调用结束后依然存在：写入设置表的值、你生成的单位。监听开关状态并把它们复原：

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

## 应避免的做法 {#things-to-avoid}

- **直接替换全局变量。** `SomeTable.func = mine` 会在任何大厅中持续运行，因为加载器无法关闭它。加载器会在启动时把每个模组的此类更改写入日志。
- **用资源替换游戏脚本来改变玩法。** 被替换的脚本同样无法即时关闭。Obsidian 允许这样做，但在有未同意玩家的大厅中，这项更改仍留在你的游戏里，而你的其他会话模组会关闭。
- **在游戏自身代码之外，向其他玩家发送自定义网络消息。**

> [!NOTE]
> 修改你自己的进度（例如经验或解锁）是允许的。Obsidian 会在每次部署前备份存档，且不会撤销这些更改。

## 编写库 {#libraries}

专门供其他模组使用的模组可以声明为库：

```json
"scope": "library"
```

每次部署时，如果有已启用的会话模组直接或通过其他库需要它，Obsidian 就把它当作会话模组，否则当作客户端模组。作为会话模组时，它遵循通常的规则，并计入玩家比较的组合。用 `resources` 替换游戏文件的库在两种情况下都会保留这些文件，所以不要把玩法更改放在里面。

你的代码必须两种情况都能运行。`obsidian.scope()` 告诉你本次部署的结果，`obsidian.session.guard` 用来区分你提供给其他模组的函数。请列出对任何人都安全的函数：读取、撤销，以及只有该玩家自己能看到或听到的更改。其他函数在库作为客户端模组运行时返回 `false, "needs_session"`，在未获许可的大厅中返回 `false, "not_cleared"`。忘记列出的函数会被拒绝。

```lua
local M = obsidian.module("lib/enemies")

-- 玩法挂钩要等到有会话模组需要时才安装。
if obsidian.scope() == "session" then
  obsidian.hook("Damage", "apply", M.on_damage)
end

return {
  enemies = obsidian.session.guard(M, { "list", "count" }),
}
```

你自己的代码仍使用真实的表，所以大厅不再获许可后，它依然可以撤销更改。

## 每位玩家都需要的模组 {#every-player}

多人游戏开关告诉你的模组，另一位玩家同意它运行，而不是对方也在运行它。对大多数模组来说这已足够，因为由主机的游戏决定发生什么。只有大厅中每个游戏都安装才能工作的模组（例如改变大厅人数的模组）可以提出更高要求：

```json
"scope": "session",
"every_player": true
```

这样的模组只有在其他每位玩家也运行它，且版本和共享选项值都相同时才会运行。只是同意的玩家不算。Obsidian 会像对待其他会话模组一样关闭和重新开启它，并告诉玩家原因。使用它的玩家还有自己的匹配组，因此只会找到彼此。

`obsidian.session.cleared()` 回答发出询问的模组，`obsidian.on_session` 在该模组自己的开关变化时调用它。

检查在玩家进入大厅后才运行。邀请仍可能跨越匹配组，因此在确认每位玩家之前请保持游戏原样：对于更大的大厅，请保持通常的上限，直到 `obsidian.session.cleared()` 为 `true`，并在 `on_session` 返回 `false` 时恢复上限。

## 编写扩展模组 {#expansions}

扩展模组是独立于普通游戏的内容，例如街机模式。在内容之外，它算作客户端模组：玩家保留普通匹配、银河战争和试炼场。只有内容运行过的任务才会离开共享的战争（[作用范围](wiki:scopes#expansions)）。

```json
"scope": "expansion",
"min_obsidian": "1.2.0",
"requires": ["obsidian.api"]
```

`every_player` 不与它一起使用：内容本来就只在每位玩家都运行该扩展模组时才会运行。

### 何时运行什么 {#what-runs-when}

| | 内容之外 | 内容运行期间 |
|---|---|---|
| `obsidian.hook`、`obsidian.on_frame` 和按键 | 关闭：挂钩只调用游戏的函数 | 开启，大厅中的每个游戏都是如此 |
| `obsidian.wrap`，以及带 `opts.always` 的按键 | 开启 | 开启 |
| `obsidian.ui`、菜单页面和叠加层 | 开启 | 开启 |
| 库中受保护的函数，例如 Obsidian API 的大部分函数 | `false, "not_started"` | 完整可用 |
| `obsidian.session.cleared()` | `false` | `true` |

进入内容的入口（菜单页面或用 `obsidian.ui` 绘制的界面）在任何大厅中都可用。内容本身放在挂钩中。

### 启动和停止 {#starting-and-stopping}

```lua
-- 不支持扩展模组的旧版 Obsidian 会在每个大厅中运行所有挂钩。
if not obsidian.capabilities.expansion then return end

local api = obsidian.import("obsidian.api")

api.menu.add("街机", function(page)
  page:action("开始", function()
    local ok, why, names = obsidian.expansion.start()
    if ok then return "已开始" end
    if why == "players_missing" then return "缺少：" .. table.concat(names, ", ") end
    return why
  end)
  page:action("停止", function() obsidian.expansion.stop() end)
end)

-- 只在内容运行期间生效，大厅中的每个游戏都是如此。
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)

obsidian.on_session(function(on)
  if on then set_up() else undo() end
end)
```

`obsidian.expansion.start()` 在主机的游戏中调用。它会检查大厅中其他每位玩家是否运行同一个扩展模组，且版本和共享选项值都相同。然后内容开始，大厅中的其他游戏会在大约一秒内跟上。`obsidian.on_session(true)` 会在每个游戏中运行，内容结束后运行 `obsidian.on_session(false)`，所以设置和撤销都应放在这里。其他游戏只能通过这种方式从主机得知内容。

返回 `true`，或者返回 `false` 和原因：

| 原因 | 时机 |
|---|---|
| `"players_missing"` | 大厅中有人没有运行它。第三个返回值列出这些玩家的名字，主机也会在通知中看到 |
| `"not_host"` | 大厅的主机是另一位玩家 |
| `"busy"` | 另一个扩展模组的内容正在运行。一次只能运行一个 |
| `"not_expansion"` | 发出调用的模组不是扩展模组 |

`obsidian.expansion.stop()` 在主机的游戏中结束内容，内容未运行时返回 `false, "not_running"`。离开大厅也会结束内容。主机离开时，接管大厅的玩家会让内容继续运行。`obsidian.expansion.running()` 返回发出调用的模组的内容当前是否在运行，`running(id)` 用于查询其他模组。

### 加入的玩家 {#players-who-join}

内容运行期间，主机的大厅在匹配中只对运行同一扩展模组、同一版本的玩家显示。没有该扩展模组的玩家通过邀请或好友尝试加入时，会在载入前被拒绝，主机会看到其名字。仍然在没有该扩展模组的情况下进入任务的玩家，会被游戏自带的踢出功能移除，内容结束后可以再回来。玩家加入或离开，都不会为已在内容中的玩家关闭内容。

### 扩展模组依赖的库 {#libraries-an-expansion-requires}

扩展模组依赖的库在部署时算作客户端模组，除非也有会话模组依赖它。`obsidian.scope()` 在其中返回 `"session"`，以便库准备好扩展模组可能用到的一切；其余时候，它像客户端库一样运行。内容运行期间，库中受保护的函数对该扩展模组可用，`obsidian.on_session` 会在内容开始和结束时通知库，以便库撤销所做的更改。使用同一个库的客户端模组仍会从这些函数得到 `"needs_session"`。

### 须知 {#things-to-know}

- 用 `resources` 替换的文件始终留在游戏中，无论内容是否运行。这些文件应只包含客户端模组也能更改的内容，例如字体和图片。
- 与会话模组一样，共享选项值会计入。个人选项应标记为 `"scope": "client"`。
- 内容运行过的任务，其结果永远不会进入银河战争，即使内容在结算画面之前就已停止。下一场不含该内容的任务会重新计入。
- `min_obsidian` 让 Obsidian 1.1 不部署该模组，并提示玩家更新。Obsidian 1.0 不读取该字段，会把它当作未指定模组运行，因此入口脚本顶部的功能检查能让模组在那里不进入游戏。

## 会话模组中的选项 {#options-in-session-mods}

比较玩家的组合时，每个选项值都会计入，除非把它标记为 `"scope": "client"`。扩展模组也一样。只改变该玩家自己所见内容的选项应这样标记（[选项和按键](wiki:options-and-keys#personal-options-in-session-mods)）。
