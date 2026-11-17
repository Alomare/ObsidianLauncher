---
title: obsidian.json 文件
group: 制作模组
summary: 描述模组的文件中的每个字段。
---

`obsidian.json` 位于模组文件夹的顶层，或 `.zip` 的顶层。Obsidian 会在导入时检查它，并在有不符合之处时指出哪个字段有误。

## 完整示例 {#a-full-example}

```json
{
  "obsidian": 1,
  "id": "yourname.better-camera",
  "version": "1.2.0",
  "name": { "en": "Better Camera", "pt-BR": "Câmera Melhor" },
  "description": { "en": "A closer camera with a toggle key." },
  "authors": ["Author Name"],
  "image": "image.png",
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1,
  "options": [
    { "id": "distance", "type": "number", "default": 5, "min": 1, "max": 12, "step": 0.5,
      "label": { "en": "Camera distance" } }
  ],
  "keybinds": [
    { "id": "toggle", "default": "f8", "label": { "en": "Toggle the camera" } }
  ],
  "resources": []
}
```

## 字段 {#fields}

| 字段 | 必需 | 说明 |
|---|---|---|
| `obsidian` | 是 | 格式版本。目前始终为 `1`。 |
| `id` | 是 | 永久 id：1 到 64 个字符，由 `a-z`、`0-9`、`.`、`_` 和 `-` 组成，以字母或数字开头。使用 `yourname.mod-name` 的形式。 |
| `version` | 是 | 版本号，例如 `1.2.0`（语义化版本）。导入时较高版本会替换较低版本。 |
| `name` | 是 | 按语言提供的模组名称。英文（`en`）是必需的。 |
| `description` | 否 | 按语言提供的一两句话，显示在名称下方。 |
| `authors` | 否 | 名字列表。 |
| `image` | 否 | 模组文件夹中的方形 PNG、JPG 或 WebP 图片，最大 4 MB。 |
| `source` | 否 | 模组的网址，显示在详情中。GitHub 仓库（例如 `https://github.com/author/mod`）还能让玩家在应用中更新模组（[分享模组](wiki:sharing#updates)）。 |
| `scope` | 否 | `client`、`session`、`expansion` 或 `library`（[作用范围](wiki:scopes)）。缺失或未知时模组为未指定，作为客户端模组运行，带 `every_player` 时作为会话模组运行。 |
| `every_player` | 否 | 设为 `true` 表示该模组只有在房间中所有玩家都运行它时才能工作（[所有玩家都需要的模组](wiki:session-mods#every-player)）。不适用于客户端模组和扩展。 |
| `secondary_scopes` | 否 | 以不同于 `scope` 的作用范围运行的代码、选项和文件（[混合作用范围](#hybrid-scopes)）。不适用于库。 |
| `entry` | 否 | 加载器启动的 Lua 脚本。只替换游戏文件的模组省略此项。 |
| `modules` | 否 | `entry` 之外的其他 `.lua` 文件，模组通过 `obsidian.module` 加载。路径位于模组文件夹内。 |
| `requires` | 否 | 此模组需要的模组 id。启用此模组会同时启用它们，并且它们先启动。玩家可以在模组的依赖标签页中看到它们。缺少任何一个时，部署会停止并给出提示。 |
| `load` | 否 | 模组请求在加载顺序中的位置：`"first"`、`"last"`，或包含更多请求的对象（[加载顺序请求](#load-order-requests)）。 |
| `load_after` | 否 | 此模组请求在其之后加载的模组 id。等同于 `load` 中的 `after`，为旧模组保留。 |
| `loader_api` | 否 | 模组需要的 [Loader API](wiki:loader-api) 版本。省略时为 `1`。 |
| `min_obsidian` | 否 | 模组支持的最旧 Obsidian 版本，例如 `"1.1.0"`。更旧的版本会拒绝该模组并要求玩家更新。1.1 之前的 Obsidian 不读取此项。 |
| `options` | 否 | 玩家在应用中更改的设置（[选项和按键](wiki:options-and-keys)）。 |
| `option_groups` | 否 | 可折叠的标题，用于给选项分组（[选项分组](wiki:options-and-keys#option-groups)）。 |
| `option_previews` | 否 | 根据 SVG 模板绘制的选项值预览图（[选项预览](wiki:options-and-keys#option-previews)）。 |
| `keybinds` | 否 | 玩家可以在应用中重新绑定的按键。按键绑定的 `pad` 指定一个选择型选项，其中保存同一操作的手柄输入（[手柄输入](wiki:options-and-keys#controller-input)）。 |
| `resources` | 否 | 模组替换或添加的游戏文件（[替换游戏文件](wiki:resources)）。 |
| `save_editor` | 否 | 模组在[存档编辑器](wiki:saves)模组标签页中显示的内容（[存档编辑器](#save-editor)）。 |

## 加载顺序请求 {#load-order-requests}

`load` 可以单独接受 `"first"` 或 `"last"`，也可以接受包含以下任意字段的对象：

```json
"load": {
  "position": "last",
  "before": ["author.other-mod"],
  "after": ["author.base-mod"],
  "after_patches": true
}
```

| 字段 | 请求内容 |
|---|---|
| `position` | `"first"` 表示加载顺序的顶部，`"last"` 表示末尾。 |
| `before` | 此模组要在其之前加载的模组 id。 |
| `after` | 此模组要在其之后加载的模组 id，这样两者都替换的文件由此模组生效。 |
| `before_patches` | 设为 `true` 表示在所有替换游戏文件的模组（有 `resources` 的模组）之前加载。 |
| `after_patches` | 设为 `true` 表示在所有替换游戏文件的模组之后加载。 |

新模组加入配置时会放到它请求的位置。此后以玩家设置的顺序为准，直到玩家在模组标签页点击**排序模组**（[排序模组](wiki:load-order#sort-mods)），它只会移动需要移动的模组。玩家没有的模组 id 会被跳过，排序模组会提示这一点。

只请求模组真正需要的。两个都要求最后加载、或互相要求在对方之后加载的模组无法同时满足：排序模组会让它们保持原位，并提示玩家手动排序。

Obsidian 会在导入时检查 `load`：`first` 或 `last` 以外的位置、未知字段、模组自身的 id，以及在 `before` 和 `after` 中出现的同一 id 都会被拒绝。Obsidian 1.3.1 及更早版本只读取 `"last"`，会把带有其他 `load` 的模组列为无法读取。

## 混合作用范围 {#hybrid-scopes}

`scope` 是模组的主作用范围。`secondary_scopes` 列出部分代码、选项和文件改用的作用范围，每个为 `client` 或 `session`（[混合作用范围](wiki:scopes#hybrid-scopes)）。会话模组用它让个人代码在未获准的房间中继续运行；扩展用会话次要作用范围来放置在内容之外、在已获准的房间中也会运行的代码。

```json
"scope": "session",
"secondary_scopes": [
  { "id": "view", "scope": "client",
    "name": { "en": "Camera and view" }, "description": { "en": "Keeps working in any lobby." } },
  { "id": "weapon_range", "scope": "session", "option": "max_weapon_range",
    "name": { "en": "Max weapon range" } }
]
```

| 字段 | 说明 |
|---|---|
| `id` | 小写字母、数字和 `_`，与选项 id 相同。 |
| `scope` | `client` 或 `session`。 |
| `name`、`description` | 按语言提供的文字，显示在模组详情和游戏内通知中。 |
| `option` | 模组的一个开关选项。只有该选项开启时次要作用范围才开启，更改它需要部署。 |

代码、选项和文件会注明各自的次要作用范围：[Loader API](wiki:loader-api#hybrid-scopes) 中的 `opts.secondary_scope`，以及[选项](wiki:options-and-keys#options-in-a-secondary-scope)和[资源](wiki:resources#hybrid-scopes)上的 `secondary_scope`。在会话模组中，未标记的内容遵循已开启的最严格的次要作用范围；在扩展中，则归属扩展本身。关闭的次要作用范围不运行代码、不部署文件，也不计入玩家之间的比较。

较旧版本的 Obsidian 只读取主作用范围，并以它运行整个模组，所以请在那里声明最严格的作用范围：带会话次要作用范围的会话模组用 `session`，扩展用 `expansion`。Obsidian 会拒绝重复的 id、`client` 或 `session` 以外的作用范围、主作用范围不是 `session` 或 `expansion` 时的会话次要作用范围、不是开关的 `option`，以及没有指向任何次要作用范围的 `secondary_scope` 标记。Obsidian 2.0.0-beta.1 及更早版本以 `parts` 为名读取同一列表，并以 `part` 作为标记；Obsidian 仍然可以读取这些名称。

## 多语言文字 {#text-in-several-languages}

玩家会读到的每个字段都是一个对象，每种语言一个条目，例如 `en`、`pt-BR`、`ru` 和 `zh-CN`，即应用支持的语言。英文始终是必需的，并会在缺少某种语言时代替显示。

模组在游戏中显示的文字放在 `obsidian.json` 旁边的 `locales` 文件夹中，每种游戏语言一个文件（`locales/en.json`、`locales/fr.json` 等），每个文件都是由键和文字组成的简单对象。模组通过 `obsidian.text` 读取它们，该函数的说明中列出了文件名（[Loader API](wiki:loader-api#obsidian-text-key)）。

## 存档编辑器 {#save-editor}

`save_editor` 为模组在[存档编辑器](wiki:saves)的模组标签页中添加一个分区，用于沙盒存档。这些值保存在模组通过 [`obsidian.save_data`](wiki:loader-api#save-data) 读取的数据中，位于模组 id 之下。

```json
"save_editor": {
  "items": [
    { "type": "stratagem", "name": "my_strike", "label": { "en": "Orbital Strike" }, "unlock": "manual" },
    { "type": "weapon", "name": "my_rifle", "label": { "en": "Long Rifle" }, "unlock": { "rank": 10 } }
  ],
  "fields": [
    { "id": "difficulty", "type": "choice", "label": { "en": "Difficulty" }, "choices": ["easy", "hard"] },
    { "id": "boost", "type": "number", "label": { "en": "Boost" }, "min": 0, "max": 5, "when": { "difficulty": "hard" } },
    { "id": "enabled", "type": "toggle", "label": { "en": "Enabled" } }
  ]
}
```

`items` 列出模组添加到游戏中的内容。`type` 为 `stratagem`、`weapon` 或 `perk`，`unlock` 说明存档如何获得该物品：`"manual"` 显示一个开关，`"free"` 和 `"base"` 表示每个存档都有，`{ "rank": n }` 表示达到该军衔时获得。

`fields` 是玩家可以更改的值。`type` 为 `number`（带 `min` 和 `max`）、`toggle` 或 `choice`（带 `choices`，每项是一个 id 或带 `id` 和 `label` 的对象）。`when` 会让该字段显示为灰色，除非指定的字段为指定的值。

模组停用时，该分区显示为灰色，数据保留在存档中。如果模组离开了库，数据会以模组 id 列出，供玩家删除。`"type": "tree"` 为日后保留。

## Obsidian 不认识的字段 {#fields-obsidian-doesn-t-know}

未知字段会被保留并忽略，所以为更新版本的 Obsidian 编写的模组仍然可以导入。
