---
title: 选项和按键
group: 制作模组
summary: 玩家在应用中更改的设置和可以重新绑定的按键，以及如何在 Lua 中读取。
---

玩家选中模组时，选项会显示在右侧面板中。Obsidian 会按照声明检查每个值，所以模组永远不会收到超出声明范围的值。选项值属于配置，因此两个配置可以用不同设置运行同一个模组。

## 声明选项 {#declaring-options}

每个选项都有 `id`（小写字母、数字和 `_`）、`type`、`default` 和 `label`。`description` 添加玩家悬停在选项上时看到的文字。

| 类型 | 值 | 额外字段 |
|---|---|---|
| `toggle` | `true` 或 `false` | |
| `number` | 数字 | `min`、`max`、`step` |
| `choice` | 列出的 id 之一 | `choices`：id 列表，或 `{ "id": "…", "label": { … } }` 的列表 |
| `key` | 按键名称，例如 `f8` | |
| `color` | `#rrggbb` | |

```json
"options": [
  { "id": "enabled", "type": "toggle", "default": true, "label": { "en": "Show the compass" } },
  { "id": "style", "type": "choice", "default": "small",
    "choices": [ { "id": "small", "label": { "en": "Small" } }, { "id": "large", "label": { "en": "Large" } } ],
    "label": { "en": "Size" }, "visible_when": { "enabled": true } },
  { "id": "tint", "type": "color", "default": "#f0b429", "label": { "en": "Color" }, "scope": "client" }
]
```

### 只在需要时显示选项 {#showing-options-only-when-they-matter}

`visible_when` 会隐藏某个选项，直到其他选项取得列出的值。在上面的例子中，只有 **Show the compass** 开启时才会显示 **Size**。

### 会话模组中的个人选项 {#personal-options-in-session-mods}

Obsidian 比较两名玩家的模组时，会话模组或扩展的选项值会计入，因为平衡滑块会改变共享游戏。如果某个值只改变本玩家看到的内容（例如颜色或镜头距离），请把该选项标记为 `"scope": "client"`。这样玩家可以选择不同的值，仍然算作匹配（[联机游戏](wiki:multiplayer)）。

### 次要作用范围中的选项 {#options-in-a-secondary-scope}

在使用[混合作用范围](wiki:manifest#hybrid-scopes)的模组中，`"secondary_scope": "<id>"` 会把选项放进该次要作用范围。客户端次要作用范围中的选项视为 `"scope": "client"`，已关闭的次要作用范围中的选项完全不计入。在会话模组中，未标记的选项遵循已开启的最严格的次要作用范围。次要作用范围在 `option` 中指定的开关负责开关它，所以更改该开关需要部署。

## 选项分组 {#option-groups}

`option_groups` 把选项归到可折叠的标题下。每个选项用 `group` 指定所属标题：

```json
"option_groups": [
  { "id": "idle", "label": { "en": "Idle crosshair" }, "description": { "en": "While not aiming." } },
  { "id": "aim", "label": { "en": "Aiming" }, "collapsed": true }
],
"options": [
  { "id": "idle_shape", "type": "choice", "default": "cross", "choices": ["cross", "dot"], "group": "idle",
    "label": { "en": "Shape" } }
]
```

没有分组的选项排在最前，然后按声明顺序排列各组。`collapsed` 让分组默认折叠。应用会按模组记住玩家展开了哪些分组。`visible_when` 在分组内照常有效。指向不存在分组的 `group` 会在导入时被拒绝。较旧版本的 Obsidian 会忽略这两个字段，按原样列出选项。

## 选项预览 {#option-previews}

`option_previews` 会显示选项值的预览图，它根据模组文件夹中的 SVG 文件绘制，并在玩家更改值时重新绘制：

```json
"option_previews": [
  { "id": "idle_preview", "group": "idle", "template": "preview/idle.svg", "width": 240, "height": 140,
    "label": { "en": "Preview" } }
]
```

预览显示在其指定分组的顶部，或选项标签页的顶部。该文件必须是 SVG，最大 64 KB，并在导入时检查。在文件中：

- 属性值或文字中的 `{{expr}}` 会替换为表达式的值。
- 任意元素上的 `data-if="expr"` 会在表达式为假时移除该元素。

表达式可以使用数字、`'文字'`、选项 id（数字、`true` 或 `false`、选择项 id 或 `#rrggbb` 颜色）、`+ - * / %`、`== != < > <= >=`、`&& || !`、`a ? b : c`、括号，以及 `min`、`max`、`abs`、`round`、`floor`、`ceil` 和 `clamp(x, lo, hi)`。`+` 用于连接文字。有错误的表达式不产生任何结果，`data-if` 中有错误的元素会被移除。

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="240" height="140">
  <rect width="240" height="140" fill="#1b1d1f"/>
  <g data-if="idle_shape == 'cross'" stroke="{{idle_color}}" stroke-width="{{idle_thickness}}">
    <line x1="{{120 - idle_size}}" y1="70" x2="{{120 + idle_size}}" y2="70"/>
    <line x1="120" y1="{{70 - idle_size}}" x2="120" y2="{{70 + idle_size}}"/>
  </g>
  <circle data-if="idle_shape == 'dot'" cx="120" cy="70" r="{{idle_size / 2}}" fill="{{idle_color}}"/>
</svg>
```

预览以图片形式显示：脚本、`foreignObject`、`on...` 属性以及指向文件外部的链接都会被移除。

## 在 Lua 中读取选项 {#reading-options-in-lua}

```lua
local get = obsidian.settings.get

if get("enabled") then
  local size = get("style")   -- "small" or "large"
end
```

`obsidian.settings.get` 返回玩家选择的值，或默认值。

更改后的选项或按键会在下次启动时进入游戏：Obsidian 把它们写入加载器读取的设置文件，无需部署。以下选项需要部署，其提示中会显示**部署后生效。**：

- 通过 `when` 选择文件的选项（见下文）；
- 开关[次要作用范围](#options-in-a-secondary-scope)的开关选项；
- 会话模组或扩展中在比较组合时计入的选项，即所有未标记 `"scope": "client"` 且不在客户端次要作用范围中的选项；
- 没有 `entry` 脚本的模组的所有选项。

## 暂停菜单中的选项 {#options-in-the-pause-menu}

需要 Obsidian API 的模组还可以通过 `api.mod_settings` 在游戏中显示设置，位于暂停菜单 **Mods** 下的一个页面中。用 `option = true` 创建的行就是 `obsidian.json` 中同一 id 的选项：它以应用中设置的值开始，在游戏中更改的值会在游戏关闭期间回到当前配置中。[Obsidian API 参考](https://github.com/Alomare/ObsidianAPI/blob/main/REFERENCE.md#your-settings-in-the-pause-menu-apimod_settings)列出了可用的行。

## 选择文件的选项 {#options-that-pick-files}

资源可以通过 `when` 依赖某个选项。只有当选项取这些值时，资源才会进入游戏：

```json
"resources": [
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_small.lua", "when": { "style": "small" } },
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_large.lua", "when": { "style": "large" } }
]
```

更改这样的选项需要重新部署。

## 按键 {#keys}

声明模组监听的每个按键，并给出默认值：

```json
"keybinds": [ { "id": "toggle", "default": "f8", "label": { "en": "Turn the compass on or off" } } ]
```

然后在 Lua 中响应它：

```lua
obsidian.keybind("toggle", function()
  obsidian.log("toggle pressed")
end)
```

每次绑定触发时，函数运行一次。玩家在应用中重新绑定按键，模组之间的冲突也在那里解决。按键名称与游戏使用的名称相同，小写：`f8`、`k`、`insert`。玩家也可以绑定左键以外的鼠标按钮：加载器会从鼠标读取 `mouse_middle`、`mouse_right`、`mouse_extra_1` 和 `mouse_extra_2`。

### 输入方式 {#input-modes}

玩家还可以在按键标签页的**更改绑定**中选择每个绑定的触发方式：

| 方式 | 触发时机 |
|---|---|
| 按下（默认） | 按键按下的那一帧 |
| 松开 | 按键松开的那一帧 |
| 轻按 | 按键在四分之一秒内松开时 |
| 双击 | 在第一次按下后 0.3 秒内第二次按下时 |
| 长按 | 按住半秒后触发一次 |

输入方式由加载器处理，模组无需为此编写代码。

### 手柄输入 {#controller-input}

按键标签页有一列用于鼠标和键盘，一列用于手柄。如果某个操作也有手柄输入，请把该输入声明为 `choice` 选项，并在按键绑定的 `pad` 中指定该选项：

```json
"options": [ { "id": "pad_toggle", "type": "choice", "default": "thumbs",
               "choices": [ { "id": "thumbs", "label": { "en": "Both sticks pressed in" } }, { "id": "off", "label": { "en": "Off" } } ] } ],
"keybinds": [ { "id": "toggle", "default": "f8", "pad": "pad_toggle", "label": { "en": "Turn the compass on or off" } } ]
```

应用随后会在该绑定行的手柄列中显示此选项，并且不在选项标签页中显示。模组像读取其他选项一样读取它。要让输入方式也作用于手柄，请给 `obsidian.keybind` 传入一个函数作为 `opts.pad`，用来判断手柄输入在本帧是否被按住；这样加载器会在触发时以 `"pad"` 调用 `fn`（支持此功能的加载器中 `obsidian.capabilities.input_modes` 为 `true`）。否则，模组需要自行监视手柄。

玩家可以在**更改绑定**中为任意绑定设置手柄输入，无论是否有 `pad` 选项：同时按住一到四个按钮。这样加载器会为该绑定读取手柄，并以 `"pad"` 调用 `fn`，取代 `opts.pad`（支持此功能的加载器中 `obsidian.capabilities.pad_binds` 为 `true`）。模组无需为此编写代码。没有 `pad` 的绑定在该列中显示一条横线。没有手柄列的 Obsidian 版本会忽略 `pad`，并把该选项列在选项标签页中。
