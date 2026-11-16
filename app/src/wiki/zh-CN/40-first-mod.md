---
title: 第一个模组
group: 制作模组
summary: 用两个文件做出一个可用的模组，无需安装编译器或其他工具。
---

Obsidian 模组是一个文件夹，其中有一个描述模组的 `obsidian.json` 文件，以及模组使用的文件。脚本模组只需要一个 Lua 文件。Obsidian 会在部署时编译 Lua，所以模组就是纯文本，完全不需要游戏的开发工具。

## 1. 创建文件夹 {#1-make-the-folder}

在任意位置创建一个文件夹，例如 `hello-helldiver`，其中有两个文件：

```
hello-helldiver/
  obsidian.json
  main.lua
```

## 2. 描述模组 {#2-describe-the-mod}

`obsidian.json`：

```json
{
  "obsidian": 1,
  "id": "author.hello-helldiver",
  "version": "1.0.0",
  "name": { "en": "Hello Helldiver" },
  "description": { "en": "Writes a line to the log when the game starts." },
  "authors": ["Author Name"],
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1
}
```

- `id` 是永久的，且必须唯一。以作者名开头，这样两位作者永远不会选到同一个。
- `scope` 为 `client`，因为这个模组不会改变其他玩家能注意到的任何东西（[作用范围](wiki:scopes)）。
- `entry` 是加载器启动的脚本。

## 3. 编写脚本 {#3-write-the-script}

`main.lua`：

```lua
obsidian.log("Hello from a first mod")

obsidian.after_startup(function()
  obsidian.log("Every mod has started")
end)
```

加载器会给每个模组一个名为 `obsidian` 的全局表，其中包含 [Loader API](wiki:loader-api) 的函数。

## 4. 试一试 {#4-try-it}

1. 把文件夹拖到[模组](app:mods)标签页的模组列表上。
2. 启用该模组并点击**部署**。
3. 启动游戏，然后关闭。
4. 打开 `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`。`authorhello-helldiver.log` 中有那两行，`loader.log` 会显示模组启动。模组日志以模组 id 命名，只保留字母、数字、`_` 和 `-`。

要修改模组，编辑文件后重新导入文件夹即可：相同 id 的模组会替换旧副本。每次分享新版本时都要提高 `version`。

> [!TIP]
> Obsidian 自带的模组也是普通模组。Obsidian Patches 是带选项的客户端模组的好例子，其文件位于 [ObsidianPatches 仓库](https://github.com/Alomare/ObsidianPatches)。

## 下一步 {#next-steps}

- [obsidian.json 文件](wiki:manifest)列出了所有字段。
- [选项和按键](wiki:options-and-keys)让玩家在应用中设置模组。
- [挂钩](wiki:loader-api#hooks)是模组改变游戏行为的方式。
- 一旦模组改变共享游戏，就需要了解[编写会话模组](wiki:session-mods)。
