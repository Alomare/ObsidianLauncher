---
title: 常见问题
group: 常见问题
summary: 简短的回答，每条都附有完整说明页面的链接。
---

## 游戏 {#playing}

### 使用模组会被封号吗？ {#will-a-mod-get-me-banned}

HELLDIVERS 没有反作弊，Obsidian 也从不覆盖游戏文件。会话模组被隔离在银河战争之外；在公开大厅中，只要有玩家不允许它们，它们就会关闭（[作用范围](wiki:scopes)）。

### 能和不用 Obsidian 的朋友一起玩吗？ {#can-i-play-with-friends-who-don-t-use-obsidian}

可以。会话模组不会改变你能找到谁，也不会改变谁能加入你。在仅限好友或私人大厅中，它们和任何人一起都会运行；在公开大厅中，只要有原版玩家在场，它们就会关闭。数据替换和导入的旧式模组无法关闭，因此你带着它们主持公开大厅时，原版玩家无法加入。第一次进入飞船时，会有一条消息告诉你属于哪种情况（[联机游戏](wiki:multiplayer#matchmaking)）。

### 我和朋友都用 Obsidian，为什么我的会话模组是关闭的？ {#my-friend-and-i-both-use-obsidian-why-are-my-session-mods-off}

在公开大厅中，只有其他每位玩家都使用相同组合或没有阻止他人的会话模组，会话模组才会运行。把大厅设为仅限好友，它们就能和任何人一起运行。导出你的配置并发给朋友，是最快的匹配方式（[配置](wiki:profiles#sharing-a-profile)）。

### 我的任务会计入银河战争吗？ {#do-my-missions-count-for-the-galactic-war}

部署了会话内容时不会，运行过扩展模组内容的任务也不会。只使用客户端模组和扩展模组时照常计入（[银河战争](wiki:galactic-war)）。

### 我的存档安全吗？ {#is-my-save-safe}

部署前，如果存档文件夹自上次备份后有变化，Obsidian 会先备份。第一份备份和最新的 9 份会保留，因此安装任何模组之前的存档一直都在（[部署与清除](wiki:deploy-and-purge#saves)）。

### 顶部栏显示“游戏文件已更改”，怎么办？ {#the-bar-says-game-files-changed-what-now}

部分游戏文件与 Obsidian 认识的版本不同，通常是因为有模组被直接复制进了游戏的 `contents` 文件夹。部署和开始游戏仍然可用。点击该提示即可查看哪些文件不同，以及如何恢复原始文件（[出现问题时](wiki:troubleshooting#build-unknown)）。

## 模组 {#mods}

### 能使用 Obsidian 之前制作的模组吗？ {#can-i-use-mods-made-before-obsidian}

可以。像平常一样把模组拖到列表上。Obsidian 只保留与游戏不同的部分，因此旧式模组可以一起运行（[添加和更新模组](wiki:adding-mods#older-mods)）。

### 两个模组修改了同一内容，哪个生效？ {#two-mods-change-the-same-thing-which-one-wins}

列表中靠下的那个（[加载顺序](wiki:load-order)）。

### 我需要超宽屏模组吗？ {#do-i-need-an-ultrawide-mod}

不需要。Obsidian Patches 会去掉宽屏上的黑边，这个选项默认开启。导入的超宽屏补丁效果相同。

### Obsidian Patches 是什么？ {#what-is-obsidian-patches}

Obsidian 自带的客户端模组，由一些小修正组成，每项都是一个选项。默认开启：跳过开场、登录界面和行星扫描，去掉宽屏上的黑边，加快任务结束后的结算画面，单人时暂停会冻结游戏，以及两处动画修正：投掷战略配备后的武器后坐力，和扑倒后的冲刺姿势。可以手动开启：跳过战争简报、瞄准光标始终显示、小地图保持打开。它位于列表顶部的 Obsidian Kit 文件夹中。

## 制作模组 {#making-mods}

### 需要什么工具吗？ {#do-i-need-any-tools}

一个文本编辑器就够了。Obsidian 在部署时编译 Lua（[你的第一个模组](wiki:first-mod)）。

### 客户端还是会话？ {#client-or-session}

如果其他玩家能察觉到更改，就选会话。如果所有更改都发生在独立于普通游戏的内容中（例如一种新模式），就选扩展。拿不准时选会话（[作用范围](wiki:scopes)）。
