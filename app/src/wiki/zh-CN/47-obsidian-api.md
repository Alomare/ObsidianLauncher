---
title: Obsidian API
group: 制作模组
summary: 供模组使用的库：按名称访问游戏数据、经过检查的修改、效果、事件和现成的操作。
---

Obsidian API 是其他模组所依赖的模组。它按名称查找游戏中的内容，以游戏接受的方式修改它们，并在需要时撤销修改。Obsidian Trainer 就是基于它构建的，而且没有使用任何其他模组无法使用的功能。

本页是概览。每个调用的参数、返回值和示例都在 [REFERENCE.md](https://github.com/Alomare/ObsidianAPI/blob/main/REFERENCE.md) 中，它也随附在 Obsidian Kit 的 Obsidian API 文件夹里，供开发模组的 AI 工具读取。该参考文档为英文。

## 使用方法 {#using-it}

把它列在 `requires` 中，Obsidian 就会随模组一起启用它，并让它先启动：

```json
"scope": "session",
"entry": "main.lua",
"requires": ["obsidian.api"]
```

然后在脚本中获取它：

```lua
local api = obsidian.import("obsidian.api")

obsidian.keybind("panic", function()
  api.player.heal()
  api.enemies.kill_near(20)
end)
```

## 客户端模组和会话模组 {#client-mods-and-session-mods}

Obsidian API 是一个[库](wiki:session-mods#libraries)，所以采用需要它的模组的作用范围。客户端模组可以依赖它并仍然是客户端模组，不影响匹配和银河战争。客户端模组可以读取任何内容，使用只有本玩家能看到或听到的功能（`api.ui`、`api.map`、`api.audio` 和 `api.music`），并监听 `api.events`。在客户端模组中，所有改变共享游戏的调用都返回 `false, "needs_session"`。

在会话模组中，这些调用在房间获准时可用。房间不再获准时，Obsidian API 会移除该模组的效果和修改，调用返回 `false, "not_cleared"`。房间再次获准时，它会恢复这些内容。扩展在内容运行期间可以使用所有调用，在内容之外得到 `false, "not_started"`（[编写扩展](wiki:session-mods#expansions)）。

## 调用如何返回 {#how-calls-answer}

操作返回 `true`，或 `false` 和简短的原因，例如 `"host_only"`、`"no_avatar"` 或 `"not_in_mission"`。创建内容的调用（例如 `api.bots.add`）返回新创建的东西，或 `nil` 和原因。普通的拒绝永远不会抛出错误：

```lua
local ok, why = api.stratagems.call("stratagem_resupply", api.world.cursor())
if not ok then obsidian.toast("No resupply: " .. why) end
```

## 包含的内容 {#what-it-holds}

| 表 | 用途 |
|---|---|
| `api.game` | 当前状态、本地玩家、主机，以及对受保护数值的安全写入 |
| `api.lobby` | 房间中的玩家，以及与房间中每台电脑共享的短文本 |
| `api.catalog` | 按名称访问武器、战略配备、特长、敌人、载具和拾取物：基础游戏以及玩家拥有的 DLC |
| `api.data` | 可以编辑和生成的内容，依据游戏数据生成 |
| `api.edits` | 对设置表的修改，经过检查并记录，可以撤销 |
| `api.effects` | 在重生和任务之间保持的修改，例如无敌或速度，以及自定义效果 |
| `api.events` | 游戏报告击杀、玩家死亡、目标、战略配备等事件时的回调 |
| `api.player` | 生命值、命数、重生、传送、外观和状态效果 |
| `api.gear` | 武器、背包、特长、无人机和补给，以及单把武器的属性 |
| `api.perks`、`api.cosmetics` | 自定义特长、头盔、护甲和披风，各自基于游戏中的一项；没有同一模组的其他玩家看到原版 |
| `api.vehicles` | 正在驾驶的载具、修理和补给 |
| `api.stratagems` | 冷却、使用次数、呼叫，以及基于游戏中战略配备制作的新战略配备 |
| `api.enemies` | 查找、生成、伤害和移除敌人，以及遭遇战的压力 |
| `api.mission` | 目标、撤离、样本、Proving Grounds 修正以及结束任务 |
| `api.combat` | 玩家和敌人的伤害倍率 |
| `api.world` | 位置、场景道具、镜头、光照、时间以及任务的外观 |
| `api.map` | 本玩家的任务地图：缩放、标记点和地图标记 |
| `api.audio`、`api.music` | 音量、音效和配乐 |
| `api.menu` | 按 **F6** 打开的游戏内窗口，每个模组都有一个页面文件夹 |
| `api.mod_settings` | 游戏暂停菜单 **Mods** 下的模组设置页面，可用分组打开单独的页面（[暂停菜单中的选项](wiki:options-and-keys#options-in-the-pause-menu)） |
| `api.ui` | 提示、地图标记、显示在地狱潜兵头上的话语、文字聊天中的一行、舰船菜单，以及用其他文字显示的游戏文本 |
| `api.progress` | 存档：样本、解锁、升级和装备 |
| `api.console` | 游戏的开发者命令，访问共享服务器的命令除外 |
| `api.bots` | 由本机操控的地狱潜兵，包括命令、装备和协助 |
| `api.brain` | 机器人做出的决策，以模组或测试可以调用的简单函数提供 |
| `api.nav` | 供机器人和模组使用的步行路线 |
| `api.capture` | 游戏截图，保存为图片文件 |

## Memory Bridge {#memory-bridge}

Obsidian API 的 **Memory Bridge** 选项默认关闭，开启后会在运行中的游戏里执行开发工具发送的 Lua 代码，无需重启即可试验模组想法。工具是 [HD1 Lua Modding Skill](https://github.com/Alomare/HD1-LuaModdingSkill) 中的 `tools/bridge.py`。它只在单人游戏或非公开大厅中运行，开启它的那次启动不会发送银河战争和排行榜结果。

## 值得了解的规则 {#rules-worth-knowing}

Obsidian API 的调用已经遵守这些规则。当模组绕过 API 直接使用游戏的表和函数时，这些规则才需要注意：

> [!WARNING]
> 游戏会为受保护的数值保存哈希副本，例如 `hitpoints_real`、`rounds`、`cooldown_timer`、`uses_left`、`num_grenades` 和 `extra_lives`。直接写入其中任何一个都会触发游戏的防篡改检查，让玩家断开网络直到游戏重启。请使用 `api.game.protected` 或排队的命令。`api.edits` 会拒绝这些键。

- 游戏不认识的排队命令，或生成资源包未加载的单位，会以任何脚本都无法捕获的错误让游戏停止。
- 游戏在大于零时运行的计时器（例如空降舱的 `drop_time`）永远不会在 0 时结束。请使用 0.01。
- 游戏的向量只在创建它的那一帧有效。要保存，请复制其中的数字。
- 经验值和军衔会随下一次正常结果进入共享排行榜，即使在清除之后也是如此。

完整列表见 REFERENCE.md。
