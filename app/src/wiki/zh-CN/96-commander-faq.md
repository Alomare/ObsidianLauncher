---
title: 常见问题
group: 常见问题
summary: 关于自定义战争的简短回答，每个都附有完整介绍该主题的页面链接。
---

## 战争 {#wars}

### 自定义战争会改变官方战争或官方存档吗？ {#does-a-custom-war-change-the-official-war-or-the-official-save}

不会。自定义战争在沙盒存档上、在单独的匹配池中进行，任务结果只会发送到战争服务器（[Commander 是什么](wiki:commander#kept-apart-from-the-official-war)）。

### 模组在自定义战争中有效吗？ {#do-mods-work-in-a-custom-war}

有效，而且使用会话模组的任务在那里会计入，因为结果永远不会进入官方战争。在房间中，会话模组遵循通常的联机规则（[联机游戏](wiki:multiplayer)）。

### 在战争中获得的军衔和解锁去了哪里？ {#where-do-rank-and-unlocks-earned-in-a-war-go}

去了进行该战争所用的沙盒存档。它们在赛季之间保留在那里，不会转移到官方存档（[游玩战争](wiki:commander-playing#the-war-save)）。

### 主机电脑关机时战争还会继续吗？ {#does-a-war-keep-running-while-the-host-pc-is-off}

不会。战争服务器在主机电脑上的 Obsidian 中运行，所以那里的 Obsidian 关闭时，战争时钟会冻结，下次启动时战争从同一时刻继续（[托管战争](wiki:commander-hosting)）。

### 战争可以移到另一台电脑吗？ {#can-a-war-move-to-another-pc}

可以，以副本的形式。在旧主机上 **Export War**，在新主机的战争标签页上**导入**，然后玩家重新加入新主机（[创建战争](wiki:commander-wars#export-import-and-delete)）。

## 分享与加入 {#sharing-and-joining}

### 加入的玩家需要 Commander 软件包吗？ {#do-players-who-join-need-the-commander-package}

不需要。有 Obsidian 就够了：战争标签页上的**加入战争**无需软件包即可使用（[加入战争](wiki:commander-joining)）。

### 朋友可以通过互联网加入吗？ {#can-friends-over-the-internet-join}

可以。点击 **Share**，然后开启 **Internet**，并在路由器上转发 TCP 端口，或使用 playit.gg 等隧道。朋友在**加入战争**的**在其他网络上**下输入地址和邀请码。Tailscale 等 VPN 无需这两者，也无需邀请码（[分享战争](wiki:commander-sharing#internet)）。

### 把战争开放到互联网安全吗？ {#is-opening-a-war-to-the-internet-safe}

对朋友来说是安全的。连接中只传输战争数据，从不传输游戏文件或 Steam 登录凭证，没有邀请码的访问者得不到任何响应。不过战争服务器信任每个游戏上报的玩家 ID，所以只把邀请码交给不会外传的可信之人，邀请码泄露时请使用 **New Code**（[邀请码](wiki:commander-sharing#the-invite-code)）。

### 可以把地址公开发布，让战争对所有人开放吗？ {#can-a-war-be-open-to-anyone-with-the-address-posted-in-public}

暂时不行。公开的邀请码起不到任何保护作用，战争服务器也没有玩家登录、踢出或封禁功能，也没有加密（[目前能做到什么](wiki:commander-sharing#what-is-possible-today)）。

### 一场战争最多有多少玩家？ {#how-many-players-can-a-war-have}

最多 5000 名玩家。与官方战争一样，每个任务房间仍然最多四人。
