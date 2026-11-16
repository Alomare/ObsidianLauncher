---
title: Obsidian 的工作方式
group: 入门
summary: Obsidian 对游戏做了什么，哪些保持不变，以及每个标签页的用途。
---

Obsidian 是 HELLDIVERS（2015）的启动器和模组管理器。模组放在库中，配置决定启用哪些模组，Obsidian 再把它们放进游戏。Obsidian 从不修改任何游戏文件，所以多个模组可以同时运行，游戏也随时可以恢复原版。

## 用补丁代替替换文件 {#patches-instead-of-replaced-files}

游戏把内容存放在安装目录 `contents` 文件夹中的资源包（bundle）里。引擎还会加载每个资源包旁边名为 `<bundle>.patch_0`、`.patch_1` 等的文件，补丁中的资源会取代资源包中的同名资源。

手工制作的模组通常替换整个资源包，因此两个这样的模组即使改动不同的内容也会互相覆盖。Obsidian 会读取每个已启用的模组，找出每个模组真正改动的资源，并为每个资源包写入一个补丁。原版资源包保持原样。

> [!NOTE]
> **部署**写入补丁，[清除](wiki:deploy-and-purge#purge)删除补丁。清除之后，安装目录与原版游戏逐字节相同。

## Obsidian Loader {#obsidian-loader}

运行代码的模组（例如镜头模组）需要一种在游戏内启动的方式。每次部署时，Obsidian 都会向游戏添加一个小脚本，即 Obsidian Loader。游戏启动时，加载器读取已部署模组的列表，逐个启动，并监督它们可以做什么。

加载器还执行联机规则。在公开房间中，改变共享游戏的模组会等待所有玩家都允许后才运行，使用模组的任务也不会计入银河战争。具体方式见[作用范围](wiki:scopes)。

## 标签页 {#the-tabs}

| 标签页 | 用途 |
|---|---|
| 主页 | 前往商店标签页、Nexus Mods、Discord 服务器、Ko-fi 和 GitHub 源代码的按钮 |
| [商店](app:store) | 一键安装的模组（[详情](wiki:tab-store)） |
| [模组](app:mods) | 模组库：启用和停用模组、加载顺序、选项、按键、**部署**和**清除**（[详情](wiki:tab-mods)） |
| [存档](app:saves) | 官方存档、沙盒存档和存档编辑器（[详情](wiki:tab-saves)） |
| [战争](app:commander) | 自定义银河战争，详见本百科的 Commander 子标签页（[详情](wiki:tab-wars)） |
| 百科 | 这些指南（[详情](wiki:tab-wiki)） |
| [账户](app:account) | GitHub 登录、问题报告和多台电脑之间的同步（[详情](wiki:tab-account)） |
| [设置](app:settings) | 游戏文件夹、语言、更新、手柄和外观（[详情](wiki:tab-settings)） |

窗口顶部的栏从左到右依次是：

- 标签页：主页、商店、模组、存档和战争。
- [防护](wiki:guardrails)按钮，一个盾牌图标。
- **开始游戏**：通过 Steam 启动游戏，使用上次部署的模组。当有多场战争或多个存档可选时，**开始游戏**旁边的菜单按钮用来选择**开始游戏**要启动的战争和存档。
- 三个图标：百科（书）、账户（人像）和设置（齿轮）。齿轮上出现圆点表示 Obsidian 有新版本。

**部署**和**清除**位于[模组](app:mods)标签页的最右端（[部署与清除](wiki:deploy-and-purge)）。

底部的状态栏显示 Obsidian 正在做什么、版本号、通知以及“关于”窗口。

## 更新 Obsidian {#updating-obsidian}

[设置](app:settings)中的**更新**标签页显示已安装的版本，检查是否有新版本，并列出每个版本的更改。系统标签页中的**有新版本时**决定启动时的行为：**在此显示**、**启动时询问**或**自动安装**。列表先显示已安装的版本和更新的版本，较旧的版本点击后展开。

下载新版本时，应用前方的窗口会显示进度。然后 Obsidian 关闭，安装程序运行片刻，Obsidian 以新版本重新打开。便携版不会安装新版本：请从发布页面下载，并解压覆盖旧的副本。

## 文件位置 {#where-your-files-are}

| 内容 | Windows | Linux |
|---|---|---|
| 模组库、配置、应用设置 | `%LOCALAPPDATA%\ObsidianLauncher`，便携版则为应用旁边的 `data` 文件夹 | `~/.local/share/ObsidianLauncher` |
| 游戏读取的选项和按键、日志 | `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher` | 游戏 Proton 前缀中的同一文件夹：`drive_c/users/steamuser/AppData/Roaming/Arrowhead/Helldivers/ObsidianLauncher` |
| 存档备份 | 应用数据文件夹中的 `backups` 文件夹 | 相同 |

[设置](app:settings)系统标签页中的**数据文件夹**会打开应用数据文件夹，**打开日志**会打开日志。
