---
title: 你的第一个模组
group: 制作模组
summary: 两个文件就能完成一个可用的模组，无需安装编译器或其他工具。
---

Obsidian 模组是一个文件夹，里面有描述模组的 `obsidian.json` 文件和模组用到的文件。脚本模组只需要一个 Lua 文件。Obsidian 在部署时编译 Lua，所以你写的是纯文本，从不需要接触游戏本身的工具。

## 1. 创建文件夹 {#1-make-the-folder}

在任意位置创建一个文件夹，例如 `hello-helldiver`，放入两个文件：

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
  "id": "yourname.hello-helldiver",
  "version": "1.0.0",
  "name": { "en": "Hello Helldiver", "zh-CN": "你好，地狱潜兵" },
  "description": { "en": "Writes a line to the log when the game starts.", "zh-CN": "游戏启动时向日志写一行。" },
  "authors": ["你的名字"],
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1
}
```

- `id` 是永久的，必须唯一。以你的名字开头，避免两位作者选到同一个。
- `scope` 为 `client`，因为这个模组不会改变其他玩家能察觉的任何东西（[作用范围](wiki:scopes)）。
- `entry` 是加载器启动的脚本。
- 英文名称（`en`）是必需的，其他语言可选。

## 3. 编写脚本 {#3-write-the-script}

`main.lua`：

```lua
obsidian.log("来自我的第一个模组的问候")

obsidian.after_startup(function()
  obsidian.log("所有模组都已启动")
end)
```

加载器为每个模组提供一个名为 `obsidian` 的全局表，包含[加载器 API](wiki:loader-api) 的所有函数。

## 4. 试运行 {#4-try-it}

1. 把文件夹拖到[模组](app:mods)标签页的模组列表上。
2. 启用模组并点击 **部署**。
3. 启动游戏，然后关闭。
4. 打开 `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`。`yournamehello-helldiver.log` 中有你写的两行，`loader.log` 显示模组正在启动。模组日志以模组 id 命名，只保留字母、数字、`_` 和 `-`。

要修改模组，编辑文件后重新导入文件夹即可：相同 id 的模组会替换旧副本。分享新版本时请提高 `version`。

> [!TIP]
> Obsidian 自带的模组也是普通模组。Obsidian Patches 是带选项的客户端模组的好例子；文件位于[它的仓库](https://github.com/Alomare/ObsidianPatches)中。

## 下一步 {#next-steps}

- [obsidian.json 文件](wiki:manifest)列出了每个字段。
- [选项和按键](wiki:options-and-keys)让玩家在应用中配置你的模组。
- [挂钩](wiki:loader-api#hooks)是模组改变游戏行为的方式。
- 一旦你的模组会改变共享游戏，请阅读[编写会话模组](wiki:session-mods)。
