---
title: 故障排除
group: 帮助
summary: 日志、启动时崩溃、无法加载的模组，以及恢复原版。
---

## 先恢复原版 {#back-to-vanilla-first}

游戏出现异常时，**清除**会从游戏中移除所有 Obsidian 文件。清除后仍存在的问题不是 Obsidian 造成的，下一步是在 Steam 中验证游戏文件完整性。

## 游戏崩溃了 {#the-game-crashed}

加载器在启动每个模组之前都会写入一行日志。如果游戏在那一刻停止，下次打开 Obsidian 时，模组标签页上的横幅会指出当时正在启动的模组。横幅上的**关闭**会停用该模组：然后部署，并告知作者。

如果游戏在之后才停止，Obsidian 只会提示游戏未正常关闭。引擎内部的崩溃通常不会在日志中留下错误，所以请从最后添加的模组开始，一次停用一个，直到崩溃消失。

## 模组没有效果 {#a-mod-does-nothing}

- 检查模组是否已启用，以及启用后是否进行过部署。**部署**按钮为绿色表示有待处理的更改。
- 对于会话模组，检查所在的房间：在有原版玩家或有阻止会话模组的玩家的公开房间中，会话模组会关闭（[联机游戏](wiki:multiplayer)）。
- 查看下面的日志。启动时出错的模组会在那里留下一行错误信息。

## “游戏文件已更改” {#build-unknown}

部分游戏文件与 Obsidian 认识的版本不一致。这通常是因为有模组被直接复制到游戏的 `contents` 文件夹中，覆盖了原始文件。部署和开始游戏仍然可用，点击警告会列出被更改的文件。要恢复原始文件，请先清除，然后在 Steam 中验证游戏文件完整性，再在 Obsidian 中用**添加**导入该模组，这样部署时不会改动原始文件。如果游戏本身提示版本未知，说明游戏程序不是 Obsidian 所针对的版本，任何模组都不会运行。

## 日志 {#logs}

在 Windows 上，日志位于 `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`（Linux 上的文件夹见[文件位置](wiki:how-obsidian-works#where-your-files-are)）。[设置](app:settings)系统标签页中的**打开日志**会打开该文件夹。

| 文件 | 内容 |
|---|---|
| `loader.log` | 每个模组的启动、挂钩的挂上、联机状态、自定义战争相关的行，以及每个模组在加载器之外改动的内容 |
| `<模组 id>.log` | 该模组通过 `obsidian.log` 写入的行，没有其他内容 |

## 寻求帮助 {#asking-for-help}

[设置](app:settings)系统标签页**问题报告**下的**复制报告**会复制版本号、已启用的模组和 `loader.log` 的末尾，不包含 Windows 账户名称。把它连同问题描述一起粘贴到 [Discord](https://dsc.gg/hd1modding) 或 [GitHub issue](https://github.com/Alomare/ObsidianLauncher/issues) 中。自定义战争的问题在 Commander 百科中有单独的页面（[故障排除](wiki:commander-troubleshooting)）。
