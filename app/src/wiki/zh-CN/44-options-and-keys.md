---
title: 选项和按键
group: 制作模组
summary: 玩家在应用中更改的设置和可重新绑定的按键，以及如何在 Lua 中读取。
---

玩家选中你的模组时，选项会显示在右侧面板中。Obsidian 会按你的声明检查每个值，所以模组永远不会收到超出你设定范围的值。选项值属于配置，因此两个配置可以用不同设置运行同一个模组。

## 声明选项 {#declaring-options}

每个选项都有 `id`（小写字母、数字和 `_`）、`type`、`default` 和 `label`。`description` 会在玩家悬停时显示说明文字。

| 类型 | 值 | 额外字段 |
|---|---|---|
| `toggle` | `true` 或 `false` | |
| `number` | 数字 | `min`、`max`、`step` |
| `choice` | 所列 id 之一 | `choices`：id 列表，或 `{ "id": "…", "label": { … } }` 列表 |
| `key` | 按键名，例如 `f8` | |
| `color` | `#rrggbb` | |

```json
"options": [
  { "id": "enabled", "type": "toggle", "default": true, "label": { "en": "Show the compass", "zh-CN": "显示指南针" } },
  { "id": "style", "type": "choice", "default": "small",
    "choices": [ { "id": "small", "label": { "en": "Small", "zh-CN": "小" } }, { "id": "large", "label": { "en": "Large", "zh-CN": "大" } } ],
    "label": { "en": "Size", "zh-CN": "大小" }, "visible_when": { "enabled": true } },
  { "id": "tint", "type": "color", "default": "#f0b429", "label": { "en": "Color", "zh-CN": "颜色" }, "scope": "client" }
]
```

### 只在需要时显示选项 {#showing-options-only-when-they-matter}

`visible_when` 会隐藏一个选项，直到其他选项取得所列的值。上例中，只有 **显示指南针** 打开时才会显示 **大小**。

### 会话模组中的个人选项 {#personal-options-in-session-mods}

比较两位玩家的模组时，会话模组和扩展模组的选项值会计入，因为平衡滑块会改变共享游戏。当某个值只影响该玩家自己看到的内容（例如颜色或镜头距离）时，给选项加上 `"scope": "client"`。这样玩家可以选择不同的值，仍然彼此匹配（[联机游戏](wiki:multiplayer)）。

## 在 Lua 中读取选项 {#reading-options-in-lua}

```lua
local get = obsidian.settings.get

if get("enabled") then
  local size = get("style")   -- "small" 或 "large"
end
```

`obsidian.settings.get` 返回玩家所选的值，或默认值。在应用中更改后，下次启动游戏即生效，无需重新部署。

## 决定文件的选项 {#options-that-pick-files}

资源可以通过 `when` 依赖某个选项。只有选项取得这些值时，该资源才会进入游戏：

```json
"resources": [
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_small.lua", "when": { "style": "small" } },
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_large.lua", "when": { "style": "large" } }
]
```

更改这类选项需要重新部署，应用会把部署按钮变为绿色提示。

## 按键 {#keys}

声明模组监听的每个按键及其默认值：

```json
"keybinds": [ { "id": "toggle", "default": "f8", "label": { "en": "Turn the compass on or off", "zh-CN": "开关指南针" } } ]
```

然后在 Lua 中响应：

```lua
obsidian.keybind("toggle", function()
  obsidian.log("按下了 toggle")
end)
```

每次按键按下时函数运行一次。玩家在应用中重新绑定按键，模组之间的按键冲突也在那里解决。按键名使用游戏中的名称，小写：`f8`、`k`、`insert`。

### 手柄输入 {#controller-input}

“按键”标签页有一列用于鼠标和键盘，另一列用于手柄。如果某个操作也有手柄输入，请把该输入声明为 `choice` 选项，并在按键的 `pad` 中写上该选项：

```json
"options": [ { "id": "pad_toggle", "type": "choice", "default": "thumbs",
               "choices": [ { "id": "thumbs", "label": { "en": "Both sticks pressed in", "zh-CN": "同时按下两个摇杆" } }, { "id": "off", "label": { "en": "Off", "zh-CN": "关闭" } } ] } ],
"keybinds": [ { "id": "toggle", "default": "f8", "pad": "pad_toggle", "label": { "en": "Turn the compass on or off", "zh-CN": "开关指南针" } } ]
```

应用随后会在该按键的行中、手柄列下显示这个选项，并且不在“选项”标签页中列出它。模组像读取其他选项一样读取它，并自行监听手柄。没有 `pad` 的按键在该列显示一个短横线。没有手柄列的 Obsidian 版本会忽略 `pad`，并在“选项”中列出该选项。
