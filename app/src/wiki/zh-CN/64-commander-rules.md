---
title: 战争规则
group: 战争
summary: 战线、敌人压力、防御和母星进攻如何推进，以及每项设置及其默认值。
---

## 战线和区域 {#fronts-and-regions}

Vanilla 战争有三条战线，每个敌人一条。每条战线的点数从 0 到 **Points per Front**，由十个区域平均分配：区域 1 紧邻超级地球，区域 10 紧邻敌人母星。每个赛季开始时，每条活跃战线都从 **Start Points** 开始，默认位于区域 2。

在游戏中完成一颗星球，会为该星球所在的战线增加 **Planet Value** 乘以难度和任务成功程度的点数。战线上没有事件时，**Enemy Pressure** 每个战争小时扣除点数。

## 防御 {#defenses}

**War Director** 开启时，每个战争小时有 **Defense Chance** 的概率在随机一条活跃战线的当前区域发起防御。一次只能进行一场防御。每赢得一场防御任务，增加 **Defense Mission Value** 乘以难度的点数，在 **Defense Duration** 结束前达到 **Defense Goal** 即赢得防御，并为战线增加 **Defense Reward**。防御失败会让战线失去一个区域。

战线点数降到 0 时，会发起超级地球保卫战。守住后，战线回到区域 2 的起点；失守则赛季失败。

## 母星进攻 {#home-world-attacks}

战线到达最后一点时，会对敌人母星发起进攻。每赢得一场进攻任务，增加 **Attack Mission Value** 乘以难度的点数，在 **Attack Duration** 结束前达到 **Attack Goal** 即击败该敌人。进攻失败会让战线退回 **Points per Front** 的 80%，位于区域 9。

## 设置 {#settings}

用**新战争**打开的 **Create War** 表单中的三个预设决定这些数值。战争进行期间，战争界面上的 **War Settings** 可以更改其中任何一项，更改从此刻起生效。War Director 关闭时，只有游戏主持人能发起防御，战线仍会随任务和敌人压力推进。

| 设置 | Standard | Short Campaign | Hard |
|---|---|---|---|
| Points per Front | 100,000 | 30,000 | 100,000 |
| Start Points | 15,000 | 3,000 | 15,000 |
| Enemy Pressure（每小时） | 600 | 250 | 1,200 |
| Planet Value | 250 | 250 | 200 |
| War Director | 开 | 开 | 开 |
| Defense Chance（每小时） | 8% | 12% | 16% |
| Defense Duration | 4 小时 | 2 小时 | 3 小时 |
| Defense Goal | 3,000 | 1,500 | 4,500 |
| Defense Mission Value | 150 | 150 | 120 |
| Defense Reward | 3,000 | 1,500 | 2,000 |
| Attack Duration | 6 小时 | 3 小时 | 4 小时 |
| Attack Goal | 5,000 | 2,500 | 8,000 |
| Attack Mission Value | 150 | 150 | 120 |
| Own Save Only | 关 | 关 | 关 |
| Allow Achievements | 关 | 关 | 关 |

最后两项属于 **Players** 分组。**Own Save Only** 把战争限定在 Obsidian 为它创建的沙盒存档上；关闭时，任意沙盒存档都可以进行该战争。**Allow Achievements** 让玩家在进行该战争时可以解锁 Steam 成就，在[防护](wiki:guardrails)中阻止了成就的玩家除外。
