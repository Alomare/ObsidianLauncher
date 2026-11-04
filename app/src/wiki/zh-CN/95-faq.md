---
title: 常见问题
group: 常见问题
summary: 简短的回答，每条都附有完整说明页面的链接。
---

## 游戏 {#playing}

### 使用模组会被封号吗？ {#will-a-mod-get-me-banned}

HELLDIVERS 没有反作弊，Obsidian 也从不修改游戏文件。会话模组被隔离在银河战争和无模组大厅之外，其他玩家永远看不到它们的效果（[作用范围](wiki:scopes)）。

### 为什么我找不到任何大厅？ {#why-can-t-i-find-any-lobbies}

部署了会话模组时，你只能看到其他运行会话模组的 Obsidian 玩家。在不含会话模组的情况下部署，即可回到普通匹配：你能看到所有人，你自己的大厅也对所有人显示（[联机游戏](wiki:multiplayer#matchmaking)）。

### 我和朋友都用 Obsidian，为什么我的会话模组是关闭的？ {#my-friend-and-i-both-use-obsidian-why-are-my-session-mods-off}

在公开大厅中，只有其他每位玩家都使用相同组合或没有阻止他人的会话模组，会话模组才会运行。把大厅设为仅限好友，它们就能和任何人一起运行。导出你的配置并发给朋友，是最快的匹配方式（[配置](wiki:profiles#sharing-a-profile)）。

### 我的任务会计入银河战争吗？ {#do-my-missions-count-for-the-galactic-war}

部署了会话内容时不会，运行过扩展模组内容的任务也不会。只使用客户端模组和扩展模组时照常计入（[银河战争](wiki:galactic-war)）。

### 我的存档安全吗？ {#is-my-save-safe}

Obsidian 在每次部署前备份存档文件夹，并保留最近 10 份（[部署与清除](wiki:deploy-and-purge#saves)）。

## 模组 {#mods}

### 能使用 Obsidian 之前制作的模组吗？ {#can-i-use-mods-made-before-obsidian}

可以。像平常一样把模组拖到列表上。Obsidian 只保留与游戏不同的部分，因此旧式模组可以一起运行（[添加和更新模组](wiki:adding-mods#older-mods)）。

### 两个模组修改了同一内容，哪个生效？ {#two-mods-change-the-same-thing-which-one-wins}

列表中靠下的那个（[加载顺序](wiki:load-order)）。

### 我需要超宽屏模组吗？ {#do-i-need-an-ultrawide-mod}

不需要。Obsidian Patches 会去掉宽屏上的黑边，这个选项默认开启。导入的超宽屏补丁效果相同。

### Obsidian Patches 是什么？ {#what-is-obsidian-patches}

Obsidian 自带的客户端模组：跳过开场、登录界面、战争简报和行星扫描，去掉宽屏上的黑边，还可以加快或跳过任务结束后的结算画面，并修正游戏在投掷战略配备和扑倒之后的两处动画问题。每项改进都是一个选项。它默认启用，并开启大部分修正；位于列表顶部的 Obsidian Kit 文件夹中。

## 制作模组 {#making-mods}

### 需要什么工具吗？ {#do-i-need-any-tools}

一个文本编辑器就够了。Obsidian 在部署时编译 Lua（[你的第一个模组](wiki:first-mod)）。

### 客户端还是会话？ {#client-or-session}

如果其他玩家能察觉到更改，就选会话。如果所有更改都发生在独立于普通游戏的内容中（例如一种新模式），就选扩展。拿不准时选会话（[作用范围](wiki:scopes)）。
