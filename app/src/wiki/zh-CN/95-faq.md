---
title: 常见问题
group: 常见问题
summary: 简短的回答，每个都附有完整介绍该主题的页面链接。
---

## 游戏 {#playing}

### 模组会导致玩家被封禁吗？ {#will-a-mod-get-me-banned}

HELLDIVERS 没有反作弊系统，Obsidian 也从不覆盖游戏文件。会话模组不会影响银河战争，在公开房间中，只要有玩家不允许，它们就会关闭（[作用范围](wiki:scopes)）。

### 能和不用 Obsidian 的朋友一起玩吗？ {#can-i-play-with-friends-who-don-t-use-obsidian}

可以。会话模组不会改变玩家能找到谁、谁能加入。在仅限好友或私人房间中，它们可以和任何人一起运行；在公开房间中，只要有原版玩家在场就会关闭。会话模组替换的游戏文件无法关闭，所以使用此类模组开设的公开房间对原版玩家关闭。第一次进入舰船时会显示一条消息，说明适用哪种情况（[联机游戏](wiki:multiplayer#matchmaking)）。

### 两个朋友都用 Obsidian，为什么会话模组是关闭的？ {#my-friend-and-i-both-use-obsidian-why-are-my-session-mods-off}

在公开房间中，只有当其他每位玩家都有相同的组合，或没有阻止其他玩家的会话模组时，会话模组才会运行。把房间设为仅限好友，就可以和任何人一起运行。导出配置并把文件发给朋友，是让组合一致的最快方法（[配置](wiki:profiles#sharing-a-profile)）。

### 使用模组的任务会计入银河战争吗？ {#do-my-missions-count-for-the-galactic-war}

部署了会话内容时不会，运行过扩展内容的任务也不会。只有客户端模组和扩展时照常计入（[银河战争](wiki:galactic-war)）。Commander 的自定义战争不同：在那里每场任务都会计入，包括使用会话模组的任务。

### 存档安全吗？ {#is-my-save-safe}

部署之前，如果存档文件夹自上次备份后有变化，Obsidian 会备份它。会保留第一次备份和最新的 9 份，因此使用任何模组之前的存档始终保留（[部署与清除](wiki:deploy-and-purge#saves)）。

### 顶部栏显示“游戏文件已更改”，该怎么办？ {#the-bar-says-game-files-changed-what-now}

部分游戏文件与 Obsidian 认识的版本不同，通常是因为有模组被直接复制到了游戏的 `contents` 文件夹中。部署和开始游戏仍然可用。点击警告会显示哪些文件不同，以及如何恢复原始文件（[故障排除](wiki:troubleshooting#build-unknown)）。

### Commander 是什么？ {#what-is-commander}

[战争](app:commander)标签页中的 Obsidian Commander 用于托管由主机设定规则的自定义银河战争，可以在一台电脑上进行，也可以通过网络或互联网与朋友分享。本百科的 Commander 子标签页对其有完整介绍（[Commander 是什么](wiki:commander)）。

## 模组 {#mods}

### Obsidian 之前制作的模组能用吗？ {#can-i-use-mods-made-before-obsidian}

能。像往常一样把模组拖到列表上即可。Obsidian 只保留与游戏不同的部分，所以旧式模组可以一起运行（[添加和更新模组](wiki:adding-mods#older-mods)）。

### 两个模组改动同一内容，哪个生效？ {#two-mods-change-the-same-thing-which-one-wins}

列表中靠下的那个生效。上下拖动模组即可改变顺序（[加载顺序](wiki:load-order)）。

### 需要超宽屏模组吗？ {#do-i-need-an-ultrawide-mod}

不需要。Obsidian Patches 会去除宽屏上的黑边，该选项默认开启。导入的超宽屏补丁也能做到同样的事。

### Obsidian Patches 是什么？ {#what-is-obsidian-patches}

随 Obsidian 提供的客户端模组，由一些小修复组成，每个修复都是一个选项。默认开启：跳过开场、登录界面和星球扫描，去除宽屏黑边，任务后更快的结算界面，单人游戏中会冻结游戏的暂停，以及两项动画修复：投掷战略配备后的武器后坐，以及扑倒后的冲刺姿势。可以手动开启：跳过战争简报、始终可见的瞄准光标，以及保持打开的小地图。Obsidian Patches 位于列表顶部的 Obsidian Kit 文件夹中。

## 制作模组 {#making-mods}

### 制作模组需要什么工具？ {#do-i-need-any-tools}

一个文本编辑器。Obsidian 会在部署时编译 Lua（[第一个模组](wiki:first-mod)）。

### 客户端还是会话？ {#client-or-session}

如果其他玩家可能注意到这项改动，就是会话。如果所有改动都发生在独立于正常游戏的内容中（例如新模式），就是扩展。拿不准时，选会话（[作用范围](wiki:scopes)）。
