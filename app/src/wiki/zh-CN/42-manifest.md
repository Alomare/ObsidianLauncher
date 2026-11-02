---
title: obsidian.json 文件
group: 制作模组
summary: 描述模组的文件中的每个字段。
---

`obsidian.json` 位于模组文件夹的顶层，或 `.zip` 的顶层。Obsidian 在导入时检查该文件，如有不符会指出是哪个字段出错。

## 完整示例 {#a-full-example}

```json
{
  "obsidian": 1,
  "id": "yourname.better-camera",
  "version": "1.2.0",
  "name": { "en": "Better Camera", "zh-CN": "更好的镜头" },
  "description": { "en": "A closer camera with a toggle key.", "zh-CN": "更近的镜头，并可用按键切换。" },
  "authors": ["你的名字"],
  "image": "image.png",
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1,
  "options": [
    { "id": "distance", "type": "number", "default": 5, "min": 1, "max": 12, "step": 0.5,
      "label": { "en": "Camera distance", "zh-CN": "镜头距离" } }
  ],
  "keybinds": [
    { "id": "toggle", "default": "f8", "label": { "en": "Toggle the camera", "zh-CN": "切换镜头" } }
  ],
  "resources": []
}
```

## 字段 {#fields}

| 字段 | 必需 | 说明 |
|---|---|---|
| `obsidian` | 是 | 格式版本，目前始终为 `1`。 |
| `id` | 是 | 永久 id：1 到 64 个 `a-z`、`0-9`、`.`、`_`、`-` 字符，以字母或数字开头。建议使用 `yourname.mod-name`。 |
| `version` | 是 | 版本号，例如 `1.2.0`（语义化版本）。导入时较高版本会替换较低版本。 |
| `name` | 是 | 各语言的模组名称，英文（`en`）必需。 |
| `description` | 否 | 各语言的一两句说明，显示在名称下方。 |
| `authors` | 否 | 作者名单。 |
| `image` | 否 | 模组文件夹中的正方形 PNG、JPG 或 WebP 图片，最大 4 MB。 |
| `source` | 否 | 模组的地址，显示在模组详情中。如果是 GitHub 仓库（例如 `https://github.com/you/mod`），玩家还可以在应用中更新此模组（[分享模组](wiki:sharing#updates)）。 |
| `scope` | 否 | `client`、`session` 或 `library`（[作用范围](wiki:scopes)）。缺失或未知时模组为未指定，作为客户端模组运行；带 `every_player` 时作为会话模组运行。 |
| `every_player` | 否 | 设为 `true` 表示模组只有在大厅中每位玩家都运行时才能工作（[每位玩家都需要的模组](wiki:session-mods#every-player)）。不适用于客户端模组。 |
| `entry` | 否 | 加载器启动的 Lua 脚本。只替换游戏文件的模组可省略。 |
| `modules` | 否 | `entry` 之外的其他 `.lua` 文件，模组用 `obsidian.module` 加载。路径位于模组文件夹内。 |
| `requires` | 否 | 本模组需要的模组 id。启用本模组时会同时启用它们，并先启动它们。玩家可在模组的“依赖”标签页中看到它们。缺少时部署会停止并给出提示。 |
| `load` | 否 | `"last"` 表示模组加入配置时排到加载顺序末尾，之后的新模组会排在它之前。此后以玩家的顺序为准。 |
| `load_after` | 否 | 本模组希望排在其后加载的模组 id，在任一方加入配置时生效。此后以玩家的顺序为准。 |
| `loader_api` | 否 | 模组需要的[加载器 API](wiki:loader-api)版本，省略时为 `1`。 |
| `options` | 否 | 玩家在应用中更改的设置（[选项和按键](wiki:options-and-keys)）。 |
| `keybinds` | 否 | 玩家可在应用中重新绑定的按键。 |
| `resources` | 否 | 模组替换或添加的游戏文件（[替换游戏文件](wiki:resources)）。 |

## 多语言文本 {#text-in-several-languages}

所有玩家会读到的字段都是按语言分条目的对象：应用使用 `en`、`pt-BR`、`zh-CN`，游戏内文本使用 `en`、`fr`、`es`、`pt-BR`。英文始终必需，并会在你省略的语言中代为显示。

模组在游戏中显示的文本放在 `obsidian.json` 旁边的 `locales` 文件夹中，每种语言一个文件（`locales/en.json`、`locales/fr.json` 等），每个文件都是简单的键值对象。模组通过 `obsidian.text` 读取。

## Obsidian 不认识的字段 {#fields-obsidian-doesn-t-know}

未知字段会被保留并忽略，因此为未来版本 Obsidian 编写的模组仍然可以导入。
