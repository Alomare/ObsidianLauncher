---
title: 替换游戏文件
group: 制作模组
summary: 按名称修改游戏脚本和其他资源，无需分发整个资源包。
---

除了运行脚本，模组还可以通过 `resources` 列表替换游戏资源或添加新资源。Obsidian 在部署时把它们放进补丁，并逐个资源地处理与其他模组的冲突。

## 修改后的游戏脚本 {#a-changed-game-script}

把要修改的游戏脚本复制到模组文件夹中，编辑后列出它：

```json
"resources": [
  { "type": "lua", "name": "lua/settings/perk_settings", "source": "res/perk_settings.lua" }
]
```

- `type` 和 `name` 是该资源在游戏中的路径。
- `source` 是模组文件夹中的普通 Lua。Obsidian 在部署时编译它。
- 对于已经是编译好的 LuaJIT 2.0 字节码的文件，可以用 `bytecode` 代替 `source`。Obsidian 会检查文件头，拒绝其他任何内容。

> [!TIP]
> 挂钩只修改一个函数，脚本的其余部分仍交给游戏，所以两个模组可以修改同一个脚本而不冲突。修改代码时优先使用[挂钩](wiki:loader-api#hooks)，只在必要时才替换整个脚本。

## 其他资源 {#other-resources}

纹理、单位、材质和其他类型使用 `data`，即游戏存储的原始资源：

```json
{ "type": "texture", "name": "units/foo/bar", "data": "res/bar.texture" }
```

导入时转换的旧式模组使用这种形式。从零制作这类文件需要的工具目前还没有面向 HD1 的版本。

## 音效库和视频 {#streamed-resources}

音效库（`timpani_bank`）和视频（`bik`）的大部分字节保存在资源包的 `.stream` 文件中。请把这部分作为 `stream` 与 `data` 一起提供：

```json
{ "type": "timpani_bank", "name": "#e709d0c5697fb286", "bundle": "0e2ec9da60a0f53a",
  "data": "res/menu.timpani_bank", "stream": "res/menu.stream" }
```

游戏中有多种语言版本的资源，在 `variants` 下列出每种语言，并在 `lang` 中写上游戏的语言标志。没有 `lang` 的变体为 0，供所有没有专属变体的语言使用：

```json
{ "type": "timpani_bank", "name": "#f0b3f997d587fb77", "bundle": "f0b3f997d587fb77",
  "variants": [
    { "data": "res/voices.timpani_bank", "stream": "res/voices.stream" },
    { "lang": 256, "data": "res/voices.fr.timpani_bank", "stream": "res/voices.fr.stream" }
  ] }
```

| 语言 | `lang` |
|---|---|
| 所有没有变体的语言 | 0 |
| 日语 | 1 |
| 法语 | 256 |
| 西班牙语（拉丁美洲） | 512 |
| 西班牙语（西班牙） | 1024 |
| 葡萄牙语（巴西） | 2048 |
| 俄语 | 4096 |
| 意大利语 | 8192 |
| 德语 | 16384 |

部署时，Obsidian 会把流式部分写入补丁旁的 `.stream` 文件（`<bundle>.patch_0.stream`），顺序与补丁列出它们的顺序一致，和游戏排列原始文件的方式相同。声音的改动只留在一台电脑上，所以音效库模组可以是[客户端模组](wiki:scopes)。

## 音效库中的单个片段 {#sound-bank-clips}

一个完整的音效库加上所有语言可能超过 1 GB。只改动少数声音的模组可以只在 `clips` 下提供这些片段。部署时，Obsidian 会让游戏的声音列表（启动资源包中的 `timpani_master`）为这些片段指向新的 id，并把带新 id 的片段放进它们所替换的音效库旁边的一个小型 Obsidian 音效库中。游戏自带的音效库保持不变：

```json
{ "type": "timpani_bank", "name": "#e0f28740d6c19769",
  "clips": [
    { "clip": "#1111111111111111", "data": "res/rock.clip", "stream": "res/rock.stream" },
    { "clip": "#1111111111111111", "lang": 256, "data": "res/rock.fr.clip", "stream": "res/rock.fr.stream" },
    { "clip": "#2222222222222222", "data": "res/short.clip" }
  ] }
```

- `clip` 是片段的 id，即 `#` 加 16 位十六进制数字。`lang` 是上表中的语言标志，省略时为 0。
- `data` 是放进音效库的内容：片段 68 字节的头部和音频的开头部分。`stream` 是其余部分（如有）。
- 带有 `clips` 的资源没有 `bundle`：该小型音效库会进入所有包含该游戏音效库的资源包，并以游戏设置的语言随之加载。未列出的片段和语言按游戏原样播放。
- 两行可以使用同一个文件。多个模组可以改动同一音效库中的不同片段，两个模组改动同一片段时，加载顺序中靠后的模组生效。如果在替换整个音效库的模组之上使用，该音效库仍会部署，未列出的语言会按那个模组的版本播放该片段。
- 部署会拒绝音效库中不存在的片段或语言，以及头部与文件大小不符的文件。

游戏更新会算作待处理的更改，新的部署会重新读取游戏。部署后的补丁包含声音列表（约 0.2 MB）和新片段，对于播放其他音频的语言，会省略该语言的原始片段：一个美式英语的脚步声位于两个音效库中，约占 4 MB 磁盘空间。

## 字段 {#fields}

| 字段 | 说明 |
|---|---|
| `type` | 资源类型，例如 `lua` 或 `texture`，对于没有已知名称的类型，为 `#` 加 16 位十六进制数字 |
| `name` | 资源名称，例如 `lua/settings/perk_settings`，或 `#` 加 16 位十六进制数字 |
| `source` | 要编译的 Lua 源代码 |
| `bytecode` | 编译好的 LuaJIT 2.0 字节码 |
| `data` | 任意其他类型的原始资源 |
| `stream` | 与 `data` 一起：游戏从 `.stream` 文件读取的那部分资源 |
| `variants` | 代替 `data`：每种语言一个条目，每个条目包含 `lang`、`data` 和可选的 `stream` |
| `clips` | 代替 `data`：要改动的音效库片段，每个包含 `clip`、`lang`、`data` 和可选的 `stream` |
| `bundle` | 资源所在的资源包，16 位十六进制数字。除带有 `clips` 的资源外，所有资源都需要 |
| `flags` | 资源包为每个资源保存的一个值；除非转换工具设置了它，否则请省略 |
| `when` | 使用该资源所需的选项值（[选项和按键](wiki:options-and-keys#options-that-pick-files)） |
| `secondary_scope` | 该资源所属的[次要作用范围](wiki:manifest#hybrid-scopes)（混合作用范围得名之前为 `part`） |

## 次要作用范围中的文件 {#hybrid-scopes}

在使用[混合作用范围](wiki:manifest#hybrid-scopes)的模组中，`"secondary_scope": "<id>"` 会把资源放进该次要作用范围。已关闭的次要作用范围中的资源不会部署，与 `when` 相同。客户端次要作用范围中的资源只属于运行它的玩家：它不会像会话模组的文件那样让原版玩家无法进入公开房间。在会话模组中，没有次要作用范围的资源遵循已开启的最严格的作用范围，所以在会话模组或混合模组中，它算作会话内容。

## 局限 {#limits}

被替换的资源是静态的：无法在游戏中途关闭。在会话模组中，请用资源处理游戏启动时读取的数据，用挂钩处理行为。混合作用范围不会改变这一点：会话次要作用范围的文件在每个房间中都留在游戏里，只有部署决定哪些文件进入游戏。
