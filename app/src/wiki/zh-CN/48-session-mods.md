---
title: 编写会话模组
group: 制作模组
summary: 如何修改共享游戏，同时让防护措施继续为你工作。
---

会话模组会改变其他玩家能察觉的内容：伤害、敌人、生成、任务。加载器决定这类模组何时可以运行（[联机游戏](wiki:multiplayer)），并随玩家进出开启或关闭它。你要做的是让每项更改都经过加载器，这样它才能被关闭。

## 设置作用范围 {#set-the-scope}

```json
"scope": "session"
```

省略该字段效果相同。只有当模组所做的一切都不会影响其他玩家时，才使用 `client`。

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

## 会话模组中的选项 {#options-in-session-mods}

比较玩家的组合时，每个选项值都会计入，除非你把它标记为 `"scope": "client"`。请标记那些只改变该玩家自己所见内容的选项（[选项和按键](wiki:options-and-keys#personal-options-in-session-mods)）。
