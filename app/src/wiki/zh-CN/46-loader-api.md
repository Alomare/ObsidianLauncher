---
title: 加载器 API
group: 制作模组
summary: obsidian 表中的每个函数，第 1 版。
---

游戏启动时，Obsidian Loader 按加载顺序启动每个已启用模组的 `entry` 脚本，各一次。每个脚本都能看到名为 `obsidian` 的全局表。模组通过它注册的所有内容都受保护运行：出错只会写入日志，绝不会阻止游戏启动。

## 挂钩 {#hooks}

### obsidian.hook(target, method, fn) {#obsidian-hook-target-method-fn}

包裹游戏中的一个函数。`target` 是一个表，或以字符串表示的全局表名。`fn` 首先收到原函数，然后是调用参数：

```lua
obsidian.hook("PlayerUnitMovement", "update", function(original, self, dt)
  -- 游戏代码之前
  local result = original(self, dt)
  -- 游戏代码之后
  return result
end)
```

如果该全局还不存在，挂钩会等待，在游戏定义它后立即生效。如果 `fn` 出错，则改为运行原函数。

在会话模组中，大厅未获许可时挂钩只会调用原函数（[编写会话模组](wiki:session-mods)）。

### obsidian.on_require(name, fn) {#obsidian-on-require-name-fn}

游戏加载脚本 `name`（例如 `"lua/settings/perk_settings"`）时调用 `fn(module)`。可用于在游戏构建设置表之后立即修改它。

### obsidian.on_frame(fn) {#obsidian-on-frame-fn}

每帧调用一次 `fn`。返回 `true` 即停止调用。会话模组的每帧函数只在获许可的大厅中运行。

### obsidian.after_startup(fn) {#obsidian-after-startup-fn}

所有模组启动后调用一次 `fn`，让模组之间无需依赖加载顺序就能协作。

## 设置和按键 {#settings-and-keys}

### obsidian.settings.get(id) {#obsidian-settings-get-id}

本模组选项 `id` 的值：玩家在应用中所选的值，或默认值（[选项和按键](wiki:options-and-keys)）。

### obsidian.keybind(id, fn) {#obsidian-keybind-id-fn}

每次绑定到 `id` 的按键按下时调用 `fn`。按键来自应用，或来自 `obsidian.json` 中的默认值。

## 多人游戏 {#multiplayer}

### obsidian.session.cleared() {#obsidian-session-cleared}

该玩家的会话模组可以运行时为 `true`。客户端模组很少需要它。

### obsidian.session.guard(t, free) {#obsidian-session-guard}

返回表 `t` 的一个视图，用于提供给其他模组。列表 `free` 中的函数照常工作。其他函数在模组作为客户端模组运行时返回 `false, "needs_session"`，在会话未获许可时返回 `false, "not_cleared"`（[编写库](wiki:session-mods#libraries)）。

### obsidian.session.members() {#obsidian-session-members}

大厅中其他玩家的列表。每项包含 `peer`、`obsidian`（该玩家是否使用 Obsidian）、`modset` 和 `consent`。

### obsidian.session.forfeit_progress() {#obsidian-session-forfeit-progress}

正在进行的任务不会向存档写入任何内容，并会告知玩家：任务中的所有人都没有经验值、样本、解锁、统计或成就。适用于改变任务进行方式的模组。效果随任务结束而结束，因此在飞船上军械库仍然可用。

### obsidian.session.hold_progress() {#obsidian-session-hold-progress}

对每个任务都是如此，只要你的模组在运行，任务的加载画面会用这一提示取代原来的小贴士。请在模组启动时调用一次。

### obsidian.on_session(fn) {#obsidian-on-session-fn}

每当加载器开启或关闭本模组时调用 `fn(on)`，让模组可以撤销其他玩家会看到的更改。

## 文本和日志 {#text-and-logs}

### obsidian.text(key, ...) {#obsidian-text-key}

从模组 `locales` 文件夹中按游戏语言取文本 `key`，缺失时使用英文。额外参数用于填充 `%s` 和 `%d` 占位符。

### obsidian.log(...) {#obsidian-log}

向本模组的日志写一行，位于 `%APPDATA%\Arrowhead\Helldivers\ObsidianModManager\logs`。

## 其他模组和更多文件 {#other-mods}

### obsidian.module(name) {#obsidian-module-name}

加载本模组的另一个 Lua 文件并返回其返回值。该文件必须列在 `obsidian.json` 的 `modules` 中，`name` 是不带 `.lua` 的路径，例如 `"lib/parts"`。

### obsidian.import(id) {#obsidian-import-id}

另一个模组的 `entry` 脚本返回的值；该模组未运行时为 `nil`。请把该模组写进 `requires`，使其先启动（[Obsidian 运行时 API](wiki:obsidian-api)）。

### obsidian.scope() {#obsidian-scope}

模组在本次部署中的运行方式：`"client"` 或 `"session"`。库会得到其中之一（[编写库](wiki:session-mods#libraries)）。

### obsidian.caller() {#obsidian-caller}

正在运行其代码的模组 id，便于共享模组记录请求者。

## 游戏内菜单 {#menu}

### obsidian.menu.add(title, build, opts) {#obsidian-menu-add-title-build-opts}

在游戏内菜单左侧的列表中添加一个页面。`title` 是文本或返回文本的函数。每次打开页面时都会运行 `build(page)`。`opts.icon` 是 `obsidian.ui.glyphs()` 中的一个：

```lua
obsidian.menu.add("我的模组", function(page)
  page:header("调整")
  page:toggle("启用", { get = function() return on end, set = function(v) on = v end })
  page:number("强度", { min = 1, max = 10, step = 1, get = get_power, set = set_power })
  page:choice("模式", { choices = { { id = "a", label = "第一种" } }, get = get_mode, set = set_mode })
  page:action("执行", function() return "完成" end)
  page:input("名称", function(text) end)
  page:page("更多", function(sub) sub:label("更深一层") end)
end, { icon = "player" })
```

每一行都可以设置 `help`（选中时显示的文字）和 `enabled`（返回 `false` 和原因的函数）。操作返回的字符串会以提示显示。操作可以调用 `page:refresh()` 重新构建页面。出错的行会显示错误，且不会计入模组的错误次数。

`toggle`、`number` 和 `choice` 行可以设置 `default`（初始值）。这样行末会出现重置按钮，数值改变后亮起，按 R 效果相同。如果写入数值无法还原，请提供 `reset`（执行撤销的函数）和 `changed`（判断是否有内容可撤销的函数）。

大厅未处于许可状态时，会话模组的页面不会列出，已打开的也会关闭。

### obsidian.menu.open(), close(), toggle(), is_open() {#obsidian-menu-open}

菜单不会自行打开，因此请绑定按键：

```lua
obsidian.keybind("open", function() obsidian.menu.toggle() end)
```

### obsidian.menu.overlay(fn) {#obsidian-menu-overlay-fn}

每帧调用 `fn(draw, width, height)`，用于在菜单之外显示内容。`draw.text(text, x, y, color)` 和 `draw.rect(x, y, w, h, color, alpha)` 以屏幕左下角为原点，单位为像素。颜色为 `{ r, g, b }`。

### obsidian.toast(text, seconds) {#obsidian-toast-text-seconds}

在屏幕左上角显示一条简短消息；游戏内菜单打开时，则显示在菜单旁边。

### obsidian.notify(text, seconds) {#obsidian-notify-text-seconds}

在游戏自己的提示框中显示你的文字，带有它的图标和声音，即使游戏提示已关闭。省略 `seconds` 时为 7。在游戏没有提示框的地方（例如主菜单），它会改为显示 toast。

提示框一次只显示一条消息，因此请把它用于少见且重要的内容，快速反馈（例如按键）请用 `obsidian.toast`。已在等待的文字不会再次加入，每个模组最多有 3 条在等待：超过后 `notify` 返回 `false` 和 `"queue_full"`。Obsidian 自己的消息优先显示。

## 替换游戏界面 {#ui}

`obsidian.ui` 用于重绘游戏菜单和 HUD 的模组。任何作用范围的模组都能使用，`requires` 中无需填写任何内容，所以换肤模组仍是客户端模组，保留银河战争和匹配功能。在模组请求之前，这里的任何内容都不会运行。

思路是保留游戏的逻辑和输入，停止它的绘制，读取它的数据，然后在上面绘制。游戏的更新和绘制函数始终运行，因为按键、鼠标和网络代码都在这些函数里。

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

如果某个界面的代码出错，该界面会在本次会话中停止，游戏重新绘制自己的界面，错误写入 `loader.log`。

### obsidian.ui.current() {#obsidian-ui-current}

玩家所在的位置：`{ state, screen, sub }`。`screen` 是本节其余部分使用的 id：菜单用其名称（`"loadout"`、`"armory_profile"`、`"main"`），没有打开菜单时为 `"title"` 或 `"ship"`，任务中为 `"hud"`，其他情况为状态自身的名称，例如 `"mission_end"`。`sub` 是其中的步骤（如果有）。

### obsidian.ui.on(id, handlers), off(id) {#obsidian-ui-on-id-handlers-off-id}

在某个界面显示时运行代码。`id` 是 `screen` 的取值，`"*"` 表示所有界面。

| 处理器 | 时机 |
|---|---|
| `enter(screen)` | 界面出现。参数与 `current()` 的返回值相同 |
| `draw(kit, data, dt)` | 每帧。`kit` 是它专用的画布，`data` 是 `obsidian.ui.data` |
| `exit()` | 界面消失 |
| `mute` | 不是函数：`true` 表示此界面，或一个 id 列表，只在界面显示期间有效 |

大厅未处于许可状态时，会话模组的处理器会停止，其静音也会解除。`off(id)` 移除本模组的处理器。

### obsidian.ui.mute(id), unmute(id) {#obsidian-ui-mute-id-unmute-id}

让游戏的某个部分停止绘制，直到 `unmute`。其逻辑继续运行。

| Id | 隐藏的内容 |
|---|---|
| 菜单名称 | 该菜单：`"loadout"`、`"armory"`、`"options"`、`"mission_briefing"` 等 |
| `"hud"` | 下列所有部分 |
| `"hud.panels"` | 玩家面板 |
| `"hud.map"` | 地图 |
| `"hud.stratagems"` | 战略配备列表和冷却 |
| `"hud.names"` | 玩家头顶的名字 |
| `"hud.world"` | 世界中物体的标签 |
| `"hud.prompts"` | 可交互物体的提示 |
| `"hud.dialogue"` | 对话 |
| `"chat"`、`"hints"`、`"samples"`、`"news"`、`"save_icon"` | 对应的内容 |
| `"mission_end"` | 结算界面的各个步骤 |

各部分相互嵌套：静音 `"hud.panels"` 时地图仍会显示，除非也将其静音。飞船上的 3D 控制台、星系地图和开场影片不是这样绘制的，所以会保留。战役界面有几处框体是直接绘制的，也会保留。

### obsidian.ui.scope(id, target, method) {#obsidian-ui-scope-id-target-method}

添加你自己的部分。之后 `mute(id)` 会停止游戏这个函数绘制的所有内容：`target` 是表或全局表的名称。其他方面不受影响。

### obsidian.ui.data {#obsidian-ui-data}

游戏数据的副本，写入副本不会改变任何内容。不适用时各函数返回 `nil`。`player` 是 1 到 4 的位置，`nil` 表示使用本机键盘的玩家。

| 函数 | 返回 |
|---|---|
| `players()` | 每位玩家的 `name`、`title`、`rank_icon`、`xp`、`color`、`index`、`slot`、`is_local`、`pad` |
| `loadout(player)` | 在配装界面：`items`、`available`、`selection`、`submenu`、`slot`、`ready`、`random`、`loadout`、`presets`、`new_unlocks`、`upgrades` |
| `hud(player)` | `state`、`health`、`shield`、`inventory`、`character`、`stratagems`、`cooldowns`、`respawn`。其中的表使用游戏自己的字段名 |
| `map(player)` | `zoom`、`offset`、`size`、`pings`、`markers` |
| `mission()` | `objectives`、`enemy`、`difficulty`、`environment`、`type`、`seed`、`game_over`、`exit` |
| `campaign()` | `status`、`statistics`、`season`。数据量大，请只请求一次 |
| `mission_end()` | `sub`，以及包含状态所有简单字段的 `values` |

### obsidian.ui.canvas(id, opts) {#obsidian-ui-canvas-id-opts}

绘图工具，用于 `draw` 处理器之外的代码。`opts.layer` 取值 0 到 875，`opts.over_fades` 让它在游戏淡出到黑屏时保持完全显示。默认情况下，画布随游戏一起淡出。

坐标是从左下角起算的像素，调用 `obsidian.ui.frame` 后则是你的设计空间。颜色为 `{ r, g, b }`、`{ r, g, b, a }`、`"#rrggbb"` 或 `"#rrggbbaa"`。

| 方法 | 绘制 |
|---|---|
| `kit:rect(x, y, w, h, color)` | 矩形 |
| `kit:poly(points, color)` | 凸多边形，由 `{ x1, y1, x2, y2, ... }` 定义 |
| `kit:outline(points, color, width)` | 形状的内描边 |
| `kit:panel{ x, y, w, h, cut, corners, fill, line, line_width }` | 切去 45 度角的面板。`corners` 取 `"tl tr br bl"` 中的任意项，默认 `"tr bl"` |
| `kit:hex(x, y, r, color, icon)` | 以一点为中心的六边形，可在其中放一个图标 |
| `kit:glyph(name, x, y, size, color)` | 由形状绘制的图标之一，以其左下角为起点。`obsidian.ui.glyphs()` 列出名称 |
| `kit:text(text, x, y, opts)` | 文本。`y` 是基线，换行符开始新的一行。`opts`：`font`、`size`、`tracking`、`align`、`color`、`leading` |
| `kit:measure(text, opts)` | 文本的宽度和高度，包含字距。不绘制 |
| `kit:point(x, y)` | 把一个像素点（例如鼠标位置）转换为此画布的坐标 |
| `kit:icon(name, x, y, w, h, color, uv)` | 游戏的一张图片。`uv` 为 `{ u0, v0, u1, v1 }`。`obsidian.ui.icons()` 列出名称 |
| `kit:art(name, x, y, w, h, color, uv)` | 你的模组自带材质中的图像，用 `obsidian.ui.art` 添加。`uv` 为 `{ u0, v0, u1, v1 }`，`u0` 大于 `u1` 时左右镜像 |
| `kit:band(x, y, w, h, color)` | 游戏的玩家色带：染色后的渐变 |
| `kit:bar(x, y, w, h, value, color, back)` | 在 `back` 之上从 0 到 1 填充的条 |
| `kit:clip(x, y, w, h, fn)` | 运行 `fn(kit)`，绘制限制在矩形内 |
| `kit:layer(n)`、`kit:opacity(a)` | 之后内容的图层和强度 |

图标（glyph）不需要美术资源，因此在 `kit:icon` 不可用的地方仍然有效。游戏内菜单就是用这些相同的调用构建的，所以菜单能画的内容，模组也能画。

字体：`"body"`、`"body_large"`、`"heading"`、`"heading_large"`、`"ui"`、`"ui_heading"`、`"chat"`、`"debug"`、`"debug_medium"`。`"ui"` 和 `"ui_heading"` 是 Barlow 和 Saira，即游戏内菜单的字体，包含带音标的拉丁字母。`"heading"` 是 Insignia；`"chat"` 含中文、日文和韩文，字体缺字时文本会改用它。`"debug"` 无需加载任何内容，其他字体在游戏加载完成之前都会改用它。

裁剪能精确切割形状和图标，文本则以完整字符为单位。配装界面的菜单美术不可用：其材质在状态切换之间会卸载，而列出未加载材质的 gui 可能使游戏停止。你自己的美术也需要同样的检查，所以画布目前只接受游戏的图片。

### obsidian.ui.font(id, opts) {#obsidian-ui-font-id-opts}

添加你自己的字体，供 `kit:text` 使用。Obsidian 仓库中的 `tools/bake_font.py` 会把 TrueType 文件转换为游戏能读取的格式：

```
pip install pillow uharfbuzz
python tools/bake_font.py --out res --base mymod/fonts title=MyFont.ttf
```

然后把每个文件列入清单的 `resources`，放在启动包（boot bundle）中：

```json
"resources": [
  { "type": "font", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.font" },
  { "type": "texture", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.texture" },
  { "type": "material", "name": "mymod/fonts/fonts", "bundle": "9cf71054ea26dddf", "data": "res/fonts.material" }
]
```

在模组启动时添加，并通过其 id 使用：

```lua
obsidian.ui.font("title", { path = "mymod/fonts/title", size = 32 })
kit:text("HELLDIVERS", 100, 200, { font = "title" })
```

`size` 是文本未指定大小时使用的字号。如果部署中没有该字体，返回 `false`，指定它的文本会改用调试字体。它必须放在启动包中，因为列出未加载材质的 gui 可能使游戏停止。文本在你烘焙的大小以内（默认 48）保持清晰，字体中缺少的字符来自 `"chat"`。请检查字体的许可：烘焙得到的是修改版，因此带保留名称的字体不能沿用该名称。

### obsidian.ui.art(name, opts) {#obsidian-ui-art-name-opts}

为 `kit:art` 添加模组放在启动包中的材质的子材质。`opts.path` 是材质的资源名称。部署中没有该材质时返回 `false`。

### obsidian.ui.frame(w, h), safe() {#obsidian-ui-frame-w-h-safe}

`frame(1920, 1080)` 让本模组的坐标成为设计空间，按玩家的窗口和界面缩放，以游戏的方式缩放并居中。`frame()` 恢复为像素。调用一次即可，模组启动时也可以调用。 界面缩放小于 1 时，游戏会把该空间留在窗口底部；`frame(1920, 1080, { centered = true })` 则让它保持在中间。

`safe()` 返回 `{ x, y, w, h, scale }`，即游戏放置自己菜单的区域，使用你的坐标。

### obsidian.ui.remap_hits(id, fn) {#obsidian-ui-remap-hits-id-fn}

游戏按绘制位置检测鼠标。当你的布局移动了某个内容时，`fn(x, y, w, h)` 会收到每个矩形的像素值，并返回它现在的位置，`nil` 表示不变，`false` 表示不可点击。`remap_hits(id)` 停止映射。

### obsidian.ui.prompt(name, pad) {#obsidian-ui-prompt-name-pad}

玩家设备对应的按键，作为可绘制的文本。`name` 是动作（如 `"select"`、`"back"`、`"fire"`、`"reload"`、`"map"`）、标记（如 `"#mod_map"`）或游戏的文本键，文本键返回时已填入按键。`pad` 是 `data.players()` 中的手柄编号，`nil` 表示主玩家。

## 保存的值 {#storage}

### obsidian.storage.get(key, default), set(key, value), all() {#obsidian-storage}

本模组在多次启动之间保留的字符串、数字和布尔值。`set` 传入 `nil` 会删除键。文件会立即写入，因此不要每帧调用。

## 版本和功能 {#version-and-features}

`obsidian.version` 是 API 版本，目前为 `1`。需要更新 API 的模组在 `obsidian.json` 中设置 `loader_api`，较旧的加载器会拒绝启动它。

`obsidian.capabilities` 是一个只读表，列出当前加载器支持的功能，每项一个条目：`hook`、`on_require`、`on_frame`、`on_session`、`after_startup`、`settings`、`keybind`、`session`、`text`、`log`、`module`、`import`、`caller`、`menu`、`toast`、`notify`、`storage`、`ui`。使用较晚加入的功能前，先按名称检查：

```lua
if obsidian.capabilities.on_frame then
  obsidian.on_frame(update)
end
```

## 模组出错时 {#when-a-mod-fails}

- 启动时出错会在本次会话中关闭该模组，并把错误写入 `loader.log`。
- 挂钩和回调中的错误在每个位置最多记录三次，因此出错的每帧函数不会刷屏日志。
- 持续出错的模组会在本次会话剩余时间内被关闭。
- 如果游戏在某个模组启动时崩溃，应用会在下次启动时指出该模组。
