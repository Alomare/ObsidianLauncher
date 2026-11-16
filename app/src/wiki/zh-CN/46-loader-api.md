---
title: Loader API
group: 制作模组
summary: obsidian 表的所有函数，版本 1。
---

游戏启动时，Obsidian Loader 会按加载顺序启动每个已启用模组的 `entry` 脚本，各一次。每个脚本都能看到名为 `obsidian` 的全局表。模组通过它注册的一切都在保护下运行：出错会记录到日志，但永远不会让游戏停止。

## 挂钩 {#hooks}

### obsidian.hook(target, method, fn, opts)

包装游戏的一个函数。`target` 是一个表，或以字符串表示的全局表名称。`fn` 首先收到原函数，然后是调用的参数：

```lua
obsidian.hook("PlayerUnitMovement", "update", function(original, self, dt)
  -- before the game code
  local result = original(self, dt)
  -- after the game code
  return result
end)
```

如果该全局表尚不存在，挂钩会等待，并在游戏定义它时立即挂上。如果 `fn` 出错，会改为运行原函数。

在会话模组中，房间未获准时挂钩只调用原函数（[编写会话模组](wiki:session-mods)）。在扩展中，内容开始之前挂钩只调用原函数（[编写扩展](wiki:session-mods#expansions)）。`opts.secondary_scope` 把挂钩放进模组的某个[次要作用范围](#hybrid-scopes)。

### obsidian.wrap(target, method, fn)

与 `obsidian.hook` 相同，`fn` 出错时同样安全，区别是在房间未获准时以及在扩展内容之外也会继续运行。它用于进入扩展的入口，以及替换的文件同样无法关闭的模组，在那种情况下关闭挂钩会让游戏处于半修改状态。其他情况都应使用 `obsidian.hook`。在旧版加载器上请检查 `obsidian.capabilities.wrap`。

### obsidian.on_require(name, fn)

当游戏加载脚本 `name`（例如 `"lua/settings/perk_settings"`）时调用 `fn(module)`。用它在游戏构建设置表之后立即修改该表。如果脚本已经加载，`fn` 会立即运行。

### obsidian.on_frame(fn, opts)

每帧调用一次 `fn`。返回 `true` 可停止调用。会话模组的帧函数只在已获准的房间中运行，扩展的帧函数只在内容运行期间运行。`opts.secondary_scope` 把函数放进模组的某个[次要作用范围](#hybrid-scopes)。`opts.always` 让它在任何房间中运行，就像 `obsidian.wrap` 之于挂钩，适用于只有本玩家能看到的工作；请先检查 `obsidian.capabilities.secondary_scopes`，因为旧版加载器仍会像以前那样限制它。

### obsidian.after_startup(fn)

在所有模组都启动后调用 `fn`，让模组无需依赖加载顺序就能协同工作。

## 设置和按键 {#settings-and-keys}

### obsidian.settings.get(id, mod_id)

此模组选项 `id` 的值：玩家在应用中选择的值，或默认值（[选项和按键](wiki:options-and-keys)）。模组替换的游戏脚本在模组之外运行，所以在那里请把模组 id 作为 `mod_id` 传入。

### obsidian.keybind(id, fn, opts)

每次绑定 `id` 按玩家选择的[输入方式](wiki:options-and-keys#input-modes)（未选择时为按下）触发时，调用 `fn(device)`。按键来自应用，或来自 `obsidian.json` 中的默认值。`device` 为 `"key"`，或在手柄输入触发时为 `"pad"`：手柄输入是玩家在应用中设置的那个，否则为 `opts.pad`，即一个返回该绑定的手柄输入在本帧是否被按住的函数。会话模组的按键要等房间获准，扩展的按键要等内容开始，与模组的挂钩一样。`opts.always` 让按键不受限制，就像 `obsidian.wrap` 之于挂钩，适用于只作用于本玩家画面的按键。`opts.secondary_scope` 把按键放进模组的某个[次要作用范围](#hybrid-scopes)。

## 联机 {#multiplayer}

### obsidian.session.cleared(id)

调用它的模组可以运行时为 `true`：会话已获准，并且对于标记了 `every_player` 的模组，其他每位玩家也在运行它（[所有玩家都需要的模组](wiki:session-mods#every-player)）。对于扩展，内容运行期间为 `true`。客户端模组很少需要它。传入某个[次要作用范围](#hybrid-scopes)的 id 时，返回该作用范围的结果：客户端次要作用范围始终为 `true`，已关闭的次要作用范围为 `false`。

### obsidian.session.guard(t, free)

表 `t` 的一个视图，用于交给其他模组。`free` 列表中列出的函数照常工作。调用它的模组以客户端模组运行时，其他所有函数返回 `false, "needs_session"`，会话未获准时返回 `false, "not_cleared"`（[编写库](wiki:session-mods#libraries)）。扩展调用它们时，在内容之外得到 `false, "not_started"`，在内容运行期间可以完整使用它所需要的库，无论房间中有谁。

### obsidian.session.members()

房间中其他玩家的列表。每个条目包含 `peer`、`obsidian`（该玩家是否运行 Obsidian）、`modset` 和 `consent`（除非该玩家在公开房间中阻止其他玩家的会话模组，否则为 true）。

要让模组只在所有玩家都拥有它时运行，请在清单中设置 [`every_player`](wiki:session-mods#every-player)，而不要自己比较这些信息。

### obsidian.session.forfeit_progress(), release_progress(key) and hold_progress()

这些函数什么都不做。存档属于玩家，所以无论模组做什么，Obsidian 都不会扣留任务的任何进度。保留它们是为了让调用它们的模组继续工作。只有银河战争和排行榜的上报仍被阻止，即部署了会话内容时，以及运行过扩展内容的任务（[银河战争](wiki:galactic-war)）。要让某次游戏不计入，玩家可以在[防护](wiki:guardrails)中开启**冻结账号进度**。

### obsidian.guardrails()

玩家为本次启动在**防护**中所做的选择，形式为 `{ consent, block_achievements, freeze_progress }`，其中玩家在公开房间中阻止其他玩家的会话模组时 `consent` 为 false。模组可以读取它们，例如告诉玩家账号已冻结。只有玩家能更改它们，加载器会对所有模组强制执行（[防护](wiki:guardrails)）。

### obsidian.war()

本次启动所玩的 [Commander](wiki:commander) 自定义战争，形式为 `{ id, model }`，其中 `model` 为 `"vanilla"` 或 `"enhanced"`；在官方战争中为 `nil`。Enhanced 战争的配套模组读取它来判断何时行动。请先检查 `obsidian.capabilities.war`：旧版加载器没有这个调用。

### obsidian.on_session(fn, opts)

每当加载器开启或关闭此模组时调用 `fn(on)`，让模组可以撤销其他玩家会看到的内容。标记了 `every_player` 的模组还会在没有它的玩家进出时收到通知。扩展会在内容开始时收到 `true`，结束时收到 `false`，房间中的每个游戏都是如此，只被扩展需要的库也一样。带 `opts.secondary_scope` 时，`fn` 只收到该[次要作用范围](#hybrid-scopes)的开关：客户端次要作用范围永远不会关闭。

### 混合作用范围 {#hybrid-scopes}

带有 [`secondary_scopes`](wiki:manifest#hybrid-scopes) 的模组会把每个注册标记为其所属的次要作用范围，作为 `obsidian.hook`、`obsidian.on_frame`、`obsidian.keybind` 和 `obsidian.on_session` 的最后一个参数：

```lua
local view = { secondary_scope = "view" }
obsidian.hook("CameraManager", "update", on_camera, view)
obsidian.on_frame(tick_view, view)
obsidian.keybind("toggle_view", toggle, { secondary_scope = "view", always = true })
obsidian.on_session(function(on) if not on then undo_range() end end, { secondary_scope = "weapon_range" })
```

客户端次要作用范围的代码在任何房间中运行，会话次要作用范围的代码只在已获准的房间中运行。在扩展中，会话次要作用范围的代码在内容运行期间运行，在内容之外则在已获准的房间中运行。在会话模组中，未标记的代码遵循已开启的最严格的次要作用范围；在扩展中，则归属扩展本身。已关闭的次要作用范围不会注册任何东西。只运行一次的代码（例如入口脚本和 `on_require`）不做标记。所有次要作用范围共用一个 Lua 状态，所以客户端次要作用范围中的代码仍可直接调用会话代码：受限的只是注册。

当会话模组同时开启了客户端和会话次要作用范围时，`obsidian.scope()` 返回 `"hybrid"`。库所保护的调用会把混合模组视为会话模组，无论从哪个次要作用范围调用。旧版加载器会忽略 `opts.secondary_scope`，并用主作用范围限制一切，所以在告诉玩家个人代码会继续运行之前，请检查 `obsidian.capabilities.secondary_scopes`。在混合作用范围得名之前的加载器读取 `opts.part`，当前加载器仍会读取。

## 扩展 {#expansions}

适用于 `"scope": "expansion"` 的模组（[编写扩展](wiki:session-mods#expansions)）。请先检查 `obsidian.capabilities.expansion`：旧版加载器没有这些函数。

### obsidian.expansion.start()

在主机的游戏中启动调用它的扩展的内容。房间中的其他每位玩家都必须运行同一扩展、同一版本以及相同的共享选项值。返回 `true`，或 `false` 和原因：`"players_missing"`，并以第三个值给出未运行它的玩家名单；`"not_host"`；另一个扩展的内容正在运行时为 `"busy"`；或 `"not_expansion"`。被拒绝的启动还会向主机显示一条带名单的通知。

### obsidian.expansion.stop()

在主机的游戏中结束内容。返回 `true`，或 `false` 和 `"not_running"` 或 `"not_host"`。离开房间也会结束内容。

### obsidian.expansion.running(id)

扩展 `id` 的内容在本游戏中运行时为 `true`。不传 `id` 时指调用它的模组。

## 文字和日志 {#text-and-logs}

### obsidian.text(key, ...)

模组 `locales` 文件夹中键为 `key` 的文字，使用游戏语言，以英文为后备。额外参数会填入 `%s` 和 `%d` 占位符。目录文件以语言命名：`en.json`、`fr.json`、`es.json`、`pt-BR.json`、`de.json`、`it.json`、`ru.json`、`pl.json`、`ko.json`、`ja.json`、`zh-CN.json`、`zh-TW.json`、`sv.json`、`da.json`、`nl.json`、`fi.json` 或 `no.json`。

### obsidian.language()

游戏运行所用的语言，使用上述相同的名称，例如 `"ru"`。Obsidian 不认识的语言返回 `"en"`。在旧版加载器上请检查 `obsidian.capabilities.language`。

### obsidian.game_text(key, words)

让游戏为某个游戏文字键，或模组自创的键（例如新军衔的称号）显示 `words`。`words` 是字符串，或按语言划分的表，例如 `{ en = "Warden", ru = "Страж" }`，以英文为后备。`nil` 会把该键交还给游戏。其他情况返回 `false`。文字只影响本玩家看到的内容，所以在房间未获准时仍然有效。在旧版加载器上请检查 `obsidian.capabilities.game_text`。

### obsidian.log(...)

向此模组的日志写入一行，位于 `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`。

### obsidian.profiler.start(), stop(), running() and read()

测量每个模组的脚本占用每帧多少时间。`start()` 为调用它的模组开启测量，直到它调用 `stop()`；只要有任何模组请求，测量就保持开启，并会给每次进入模组的调用增加少许开销。`read()` 返回最近一次结果（大约每秒生成两次），在第一次结果之前返回 `nil`：`{ frame_ms, frames, total_ms, clock, mods = { { id, ms, peak_ms, calls } } }`，开销最大的模组排在最前。`ms` 是每帧平均时间，`peak_ms` 是单帧最长时间，`calls` 是每帧平均调用次数。挂钩在游戏原函数中花费的时间计入游戏，而不是模组。`obsidian.loader` 代表加载器自身的挂钩。F6 窗口的 Script Profiler 页面会把这些数字显示在游戏画面上。在旧版加载器上请检查 `obsidian.capabilities.profiler`。

## 其他模组和更多文件 {#other-mods}

### obsidian.module(name)

加载此模组的另一个 Lua 文件并返回它的返回值。该文件必须列在 `obsidian.json` 的 `modules` 中，`name` 是不带 `.lua` 的文件路径，例如 `"lib/parts"`。

### obsidian.import(id)

另一个模组的 `entry` 脚本返回的内容，该模组未运行时为 `nil`。请把该模组列在 `requires` 中，使它先启动（[Obsidian API](wiki:obsidian-api)）。

### obsidian.scope()

调用它的模组在本次部署中如何运行：`"client"`、`"session"`、`"hybrid"`（同时开启了客户端和会话次要作用范围的会话模组，[混合作用范围](#hybrid-scopes)）或 `"expansion"`。库会得到 `"client"` 或 `"session"`，当扩展需要它时得到 `"session"`，以便库为扩展准备可用的功能（[编写库](wiki:session-mods#libraries)）。

### obsidian.caller()

当前正在运行其代码的模组的 id，让共享模组可以记录是谁发出了请求。

### obsidian.info(id)

正在运行的模组的信息：`{ id, name, version, scope }`。`name` 来自该模组的 `obsidian.json`，使用游戏语言，否则用英文，`scope` 是 `obsidian.scope()` 在该模组中的返回值。不传 `id` 时指调用它的模组。模组未运行时为 `nil`。

## 游戏内菜单 {#menu}

游戏内窗口属于 Obsidian API，所以会随 API 的发布而变化，新模组请使用 `api.menu`（[Obsidian API](wiki:obsidian-api)）。**F6** 打开它，这个按键属于 Obsidian API，玩家可以在 Obsidian API 的按键标签页中更改，每个模组的页面放在以该模组命名的文件夹中。`obsidian.menu` 为以前编写的模组保留，它会以发起调用的模组的身份把每个调用转交给 Obsidian API 窗口。请把 `obsidian.api` 列在 `requires` 中：没有运行 Obsidian API 时就没有窗口，`open()` 返回 `false`。

### obsidian.menu.add(title, build, opts)

在游戏内菜单左侧的列表中、模组的文件夹里添加一个页面。`title` 是文字，或返回文字的函数。每次打开页面时都会运行 `build(page)`。`opts.icon` 是 `obsidian.ui.glyphs()` 中的一个：

```lua
obsidian.menu.add("Example Mod", function(page)
  page:header("Tuning")
  page:toggle("Enabled", { get = function() return on end, set = function(v) on = v end })
  page:number("Power", { min = 1, max = 10, step = 1, get = get_power, set = set_power })
  page:choice("Mode", { choices = { { id = "a", label = "First" } }, get = get_mode, set = set_mode })
  page:action("Do it", function() return "Done" end)
  page:input("Name", function(text) end)
  page:page("More", function(sub) sub:label("Deeper") end)
end, { icon = "player" })
```

任何行都接受 `help`（选中时显示的文字）和 `enabled`（返回 `false` 和原因的函数）。操作返回的字符串会以提示消息显示。操作可以调用 `page:refresh()` 重新构建页面。出错的行会显示错误，且不会计入该模组的失败次数。

`toggle`、`number` 或 `choice` 行接受 `default`，即初始值。这样行末会出现重置按钮，值改变后亮起，按 R 也可以在键盘上重置。如果写入值不能恢复原状，请提供 `reset`（撤销更改的函数）和 `changed`（判断是否有可撤销内容的函数）。

房间未获准时，会话模组的页面不会列出，已打开的也会关闭。扩展的页面会保持列出，以便从其中某个页面启动内容。

### obsidian.menu.open(), close(), toggle(), is_open()

只要列出了页面，Obsidian API 的按键就能打开窗口，所以模组不需要为此设置按键。这些函数供需要在自选时机打开窗口的模组使用。

### obsidian.menu.overlay(fn)

每帧调用 `fn(draw, width, height)`，在菜单之外的屏幕上绘制内容。`draw.text(text, x, y, color)` 和 `draw.rect(x, y, w, h, color, alpha)` 以从左下角起算的像素为单位。颜色为 `{ r, g, b }`。

### obsidian.toast(text, seconds)

在屏幕左上角显示一条简短消息，游戏内菜单打开时显示在菜单旁边。运行 Obsidian API 时，由 API 窗口绘制它。

### obsidian.notify(text, seconds) {#obsidian-notify-text-seconds}

在游戏的提示框中显示 `text`，带有提示图标和音效，即使关闭了游戏提示也会显示。省略 `seconds` 时为 7。在没有提示框的地方（例如主菜单），会改为以提示消息显示。

提示框一次只显示一条消息，所以请把它用于少见而重要的内容，快速反馈（例如按键）请用 `obsidian.toast`。已在等待中的文字不会重复添加，每个模组最多可以有 3 条在等待：超过时 `notify` 返回 `false` 和 `"queue_full"`。Obsidian 自己的消息优先。

## 替换游戏界面 {#ui}

`obsidian.ui` 用于重新绘制游戏菜单和 HUD 的模组。任何作用范围的模组都可以使用它，无需在 `requires` 中添加任何内容，所以界面换肤仍是客户端模组，并保留银河战争和匹配。在模组请求之前，这里的任何内容都不会运行。

思路是保留游戏的逻辑和输入，停止游戏自身的绘制，读取游戏数据并在上面绘制。游戏的更新和绘制函数始终运行，因为按键、鼠标和网络代码都在其中。

```lua
local ui = obsidian.ui
ui.frame(1920, 1080)

ui.on("loadout", {
  mute = true,
  draw = function(kit, data, dt)
    kit:panel{ 120, 140, 1680, 800, cut = 24, corners = "tr bl", fill = "#38383ee0", line = "#ffc433" }
    kit:text("LOADOUT", 960, 880, { font = "heading", size = 48, tracking = 4, align = "center" })

    for i, player in ipairs(data.players()) do
      local panel = data.loadout(player.slot)
      local x = 140 + (i - 1) * 420
      kit:band(x, 780, 400, 40, player.color)
      kit:text(player.name, x + 12, 792, { font = "body_large" })
      if panel then kit:text(panel.ready and "READY" or "PICKING", x + 12, 740) end
    end

    kit:text(ui.prompt("menu_loadout_select"), 140, 150)
  end,
})
```

如果某个界面的代码出错，该界面会在本次会话中停止，游戏重新绘制原界面，错误写入 `loader.log`。

### obsidian.ui.current()

玩家所在的位置：`{ state, screen, sub }`。`screen` 是本节其余部分使用的 id：按名称表示的菜单（`"loadout"`、`"armory_profile"`、`"main"`），没有打开菜单时为 `"title"` 和 `"ship"`，任务中为 `"hud"`，或状态名称，例如 `"mission_end"`。`sub` 是其中的步骤（如果有）。

### obsidian.ui.on(id, handlers), off(id)

在某个界面显示时运行代码。`id` 是 `screen` 的一个值，或表示所有界面的 `"*"`。

| 处理函数 | 时机 |
|---|---|
| `enter(screen)` | 界面出现时。它收到 `current()` 返回的内容 |
| `draw(kit, data, dt)` | 每帧。`kit` 是单独的画布，`data` 是 `obsidian.ui.data` |
| `exit()` | 界面离开时 |
| `mute` | 不是函数：对此界面为 `true`，或一个 id 列表，仅在界面显示期间生效 |

房间未获准时，会话模组的处理函数会停止，该模组的静音也会解除。扩展的处理函数在内容之外继续运行。`off(id)` 移除调用它的模组的处理函数。

### obsidian.ui.mute(id), unmute(id)

让游戏的某个部分停止绘制，直到 `unmute`。该部分的逻辑继续运行。

| Id | 隐藏的内容 |
|---|---|
| 菜单名称 | 该菜单：`"loadout"`、`"armory"`、`"options"`、`"mission_briefing"` 等 |
| `"hud"` | 以下所有部分 |
| `"hud.panels"` | 玩家面板 |
| `"hud.map"` | 地图 |
| `"hud.stratagems"` | 战略配备列表和冷却 |
| `"hud.names"` | 玩家头顶的名字 |
| `"hud.world"` | 世界中物体上的标签 |
| `"hud.prompts"` | 可互动物体上的提示 |
| `"hud.dialogue"` | 对话 |
| `"chat"`、`"hints"`、`"samples"`、`"news"`、`"save_icon"` | 分别对应各项 |
| `"mission_end"` | 结算界面的各个步骤 |

各部分是嵌套的：静音 `"hud.panels"` 会保留地图，直到也将其静音。舰船上的 3D 控制台、银河地图和开场影片不是这样绘制的，所以会保留。战役界面有几帧是直接绘制的，这些也会保留。

### obsidian.ui.scope(id, target, method)

添加一个自定义部分。此后 `mute(id)` 会停止该游戏函数绘制的所有内容：`target` 是一个表或全局表的名称。它不会改变其他任何东西。

### obsidian.ui.data

游戏已知信息的副本，所以写入它们不会改变任何东西。不适用时各函数返回 `nil`。`player` 是 1 到 4 的位置编号，`nil` 表示使用这台电脑键盘的玩家。

| 函数 | 返回 |
|---|---|
| `players()` | 每位玩家的 `name`、`title`、`rank_icon`、`xp`、`color`、`index`、`slot`、`is_local`、`pad` |
| `loadout(player)` | 在装备界面中：`items`、`available`、`selection`、`submenu`、`slot`、`ready`、`random`、`loadout`、`presets`、`new_unlocks`、`upgrades` |
| `hud(player)` | `state`、`health`、`shield`、`inventory`、`character`、`stratagems`、`cooldowns`、`respawn`。这些表使用游戏的字段名 |
| `map(player)` | `zoom`、`offset`、`size`、`pings`、`markers` |
| `mission()` | `objectives`、`enemy`、`difficulty`、`environment`、`type`、`seed`、`game_over`、`exit` |
| `campaign()` | `status`、`statistics`、`season`。数据量大，请只取一次 |
| `mission_end()` | `sub`，以及包含该状态所有普通字段的 `values` |

### obsidian.ui.canvas(id, opts)

供非 `draw` 处理函数的代码使用的绘制工具。`opts.layer` 取值 0 到 875，`opts.over_fades` 让它在游戏淡入黑屏时保持完全显示。默认情况下工具会随游戏一起淡出。

坐标是从左下角起算的像素，或在调用 `obsidian.ui.frame` 之后为模组的设计空间。颜色可以是 `{ r, g, b }`、`{ r, g, b, a }`、`"#rrggbb"` 或 `"#rrggbbaa"`。

| 方法 | 绘制内容 |
|---|---|
| `kit:rect(x, y, w, h, color)` | 矩形 |
| `kit:poly(points, color)` | 凸多边形，由 `{ x1, y1, x2, y2, ... }` 给出 |
| `kit:outline(points, color, width)` | 形状的内侧边框 |
| `kit:panel{ x, y, w, h, cut, corners, fill, line, line_width }` | 以 45 度切角的面板。`corners` 为 `"tl tr br bl"` 中的任意组合，默认 `"tr bl"` |
| `kit:hex(x, y, r, color, icon)` | 围绕某点的六边形，如提供图标则显示在其中 |
| `kit:glyph(name, x, y, size, color)` | 由形状绘制的图标之一，从左下角起算。`obsidian.ui.glyphs()` 列出名称 |
| `kit:text(text, x, y, opts)` | 文字。`y` 是基线，换行符开始新的一行。`opts`：`font`、`size`、`tracking`、`align`、`color`、`leading` |
| `kit:measure(text, opts)` | 文字的宽度和高度（含字距）。不绘制任何内容 |
| `kit:point(x, y)` | 把像素坐标点（例如鼠标位置）转换为此工具的坐标 |
| `kit:icon(name, x, y, w, h, color, uv)` | 游戏图片之一。`uv` 为 `{ u0, v0, u1, v1 }`。`obsidian.ui.icons()` 列出名称 |
| `kit:art(name, x, y, w, h, color, uv)` | 模组自带材质中、通过 `obsidian.ui.art` 添加的图片。`uv` 为 `{ u0, v0, u1, v1 }`，`u0` 大于 `u1` 时会水平翻转 |
| `kit:band(x, y, w, h, color)` | 游戏的玩家色带，带色带渐变并着色 |
| `kit:bar(x, y, w, h, value, color, back)` | 从 0 到 1 填充的进度条，绘制在 `back` 之上 |
| `kit:clip(x, y, w, h, fn)` | 运行 `fn(kit)`，绘制限制在矩形内 |
| `kit:layer(n)`、`kit:opacity(a)` | 之后绘制内容的图层和不透明度 |

字形图标不需要美术资源，所以在 `kit:icon` 不可用的地方仍然有效。游戏内菜单就是用这些调用构建的，所以菜单能画的，模组也能画。

字体：`"body"`、`"body_large"`、`"heading"`、`"heading_large"`、`"ui"`、`"ui_heading"`、`"chat"`、`"debug"`、`"debug_medium"`。`"ui"` 和 `"ui_heading"` 是 Barlow 和 Saira，即游戏内菜单使用的字体，包含带重音符号的拉丁字母。`"heading"` 是 Insignia，`"chat"` 包含中文、日文和韩文，当所选字体缺少某个字符时，文字会改用它。`"debug"` 不需要加载任何内容，在游戏加载其他字体之前，其他字体都会改用它。

裁剪会精确裁切形状和图标，文字则按整个字符裁切。装备界面的菜单美术不可用：该材质会在状态之间卸载，而列出未加载材质的 gui 可能导致游戏停止。自定义美术也需要同样的检查，所以画布目前只接受游戏自带的图片。

### obsidian.ui.font(id, opts)

为 `kit:text` 添加自定义字体。Obsidian 仓库中的 `tools/bake_font.py` 会把 TrueType 文件转换为游戏可读取的格式：

```
pip install pillow uharfbuzz
python tools/bake_font.py --out res --base mymod/fonts title=MyFont.ttf
```

然后在清单的 `resources` 中列出每个文件，放进启动资源包：

```json
"resources": [
  { "type": "font", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.font" },
  { "type": "texture", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.texture" },
  { "type": "material", "name": "mymod/fonts/fonts", "bundle": "9cf71054ea26dddf", "data": "res/fonts.material" }
]
```

在模组启动时添加字体，并按 id 使用：

```lua
obsidian.ui.font("title", { path = "mymod/fonts/title", size = 32 })
kit:text("HELLDIVERS", 100, 200, { font = "title" })
```

`size` 是未指定大小的文字所用的大小。对于部署中没有的字体，返回 `false`，使用它的文字会改用调试字体。字体必须放在启动资源包中，因为列出未加载材质的 gui 可能导致游戏停止。文字在烘焙时的大小以内保持清晰（默认 48），字体缺少的字符取自 `"chat"`。请检查字体许可证：烘焙出的字体属于修改版本，因此带有保留名称的字体不能沿用该名称。

### obsidian.ui.art(name, opts)

为 `kit:art` 添加模组在启动资源包中提供的材质的子材质。`opts.path` 是该材质的资源名称。对于部署中没有的材质，或者在游戏开始绘制后才请求新材质时，返回 `false`，所以请在模组启动时请求。

### obsidian.ui.list_with(path, with)

让游戏创建的每个列出材质 `with` 的 gui 也列出 `path`，使游戏可以在绘制 `with` 的子材质的地方绘制 `path` 的子材质。省略 `with` 时为 `"hud/hud"`，即包含军衔和战略配备图标的材质。`path` 是模组在启动资源包中提供的材质，部署中没有时返回 `false`。请在模组启动时、游戏创建 gui 之前调用。把多个图标放进一个材质，并列出该材质。

### obsidian.ui.frame(w, h), safe()

`frame(1920, 1080)` 把此模组的坐标设为设计空间，按玩家的窗口和界面缩放，以与游戏相同的方式缩放并居中。`frame()` 恢复为像素。调用一次即可，在模组启动时调用也可以。界面缩放小于 1 时，游戏会把多出的空间留在窗口底部；`frame(1920, 1080, { centered = true })` 会把它保持在中间。

`safe()` 返回 `{ x, y, w, h, scale }`，即游戏放置其菜单的区域，使用模组的坐标。

### obsidian.ui.remap_hits(id, fn)

游戏根据绘制位置检测鼠标。当模组的布局移动了某些内容时，`fn(x, y, w, h)` 会收到每个以像素表示的矩形，并返回它现在的位置，返回 `nil` 表示保持不变，返回 `false` 表示哪里都没有。`remap_hits(id)` 停止此功能。

### obsidian.ui.prompt(name, pad)

玩家设备上的按键，以可绘制的文字表示。`name` 是一个操作，例如 `"select"`、`"back"`、`"fire"`、`"reload"` 或 `"map"`，或一个标记，例如 `"#mod_map"`，或游戏文字键之一，返回时会填入按键。`pad` 是 `data.players()` 中的手柄编号，`nil` 表示主玩家。

## 保存的值 {#storage}

### obsidian.storage.get(key, default), set(key, value), all()

此模组在多次启动之间保存的字符串、数字和布尔值。`set` 传入 `nil` 会删除一个键。文件会立即写入，所以不要每帧调用。

### obsidian.save_data.get(key, default), set(key, value), sandbox() {#save-data}

此模组随当前存档保存的字符串、数字和布尔值，存放在存档旁边的文件中。每个[沙盒存档](wiki:saves)都有单独的文件，Obsidian 切换官方存档时，官方存档的文件会随存档一起移动。请把它用于属于某个角色的内容（例如手动解锁），把 `obsidian.storage` 用于属于整个安装的内容。`set` 传入 `nil` 会删除一个键，文件会立即写入。`sandbox()` 返回本次启动所用沙盒存档的 id，在官方存档上为 `nil`。请先检查 `obsidian.capabilities.save_data`：旧版加载器没有它。

玩家应能在存档编辑器中编辑的值，请放在清单的 [`save_editor` 键](wiki:manifest#save-editor)中。

## 版本和功能 {#version-and-features}

`obsidian.version` 是 API 版本，目前为 `1`。需要更新 API 的模组在 `obsidian.json` 中设置 `loader_api`，旧版加载器会拒绝启动它。

`obsidian.capabilities` 是一个只读表，列出正在运行的加载器支持的功能，每项功能一个条目：`hook`、`wrap`、`on_require`、`on_frame`、`on_session`、`after_startup`、`settings`、`keybind`、`input_modes`、`pad_binds`、`session`、`guardrails`、`war`、`scope`、`text`、`log`、`profiler`、`module`、`import`、`caller`、`info`、`menu`、`toast`、`notify`、`storage`、`save_data`、`ui`、`expansion`、`language`、`game_text`、`secondary_scopes`（以及旧名称 `parts`）。使用较晚加入的功能之前，请按名称检查：

```lua
if obsidian.capabilities.on_frame then
  obsidian.on_frame(update)
end
```

## 模组出错时 {#when-a-mod-fails}

- 启动时出错会让模组在本次会话中关闭，并把错误写入 `loader.log`。
- 挂钩和回调中的错误在每个位置最多记录三次，所以出错的帧函数不会刷满日志。
- 反复出错的模组会在本次会话剩余时间内关闭。
- 如果游戏在某个模组启动时崩溃，应用会在下次启动时指出该模组。
