---
title: 商店
group: 了解各标签页
summary: 一键安装的模组，以及如何让模组进入商店。
---

[商店](app:store)列出可以一键安装的模组，最新的排在最前。**Obsidian Kit** 位于顶部的一栏：Obsidian API、Obsidian Patches 和 Obsidian Trainer，Obsidian 正是围绕这几个模组构建的。

## 查找模组 {#finding-a-mod}

卡片上方的一栏可以按[作用范围](wiki:scopes)筛选、对列表排序，以及只显示已安装的模组。搜索框会匹配名称和描述。

每张卡片显示图片、名称、简短描述和标签：

| 标签 | 含义 |
|---|---|
| 银色方块 | **Obsidian 认证**：该模组由 Obsidian 团队或获得其信任的人制作和维护 |
| 彩色方块 | 模组的[作用范围](wiki:scopes)。使用[混合作用范围](wiki:manifest#hybrid-scopes)的模组，方块沿对角线分开：左上为主作用范围，右下为次要作用范围 |
| **EA**（抢先体验） | 低于 1.0 的版本，仍在开发中，之后的版本可能大幅变化。私有仓库中的模组也属于抢先体验，只对能读取该仓库的 GitHub 账号显示（[账户](wiki:tab-account)） |

绿色角标表示已安装的模组，带向上箭头的黄色角标表示商店中有更新版本的模组。

## 模组页面 {#the-page-of-a-mod}

点击卡片会打开模组页面：描述、各版本及其更改、选项、它依赖的模组和依赖它的模组，以及 GitHub 上未解决的问题。右侧的面板点击标题即可展开。

**安装**会从 GitHub 下载最新版本及其依赖的所有模组到库中。模组安装后处于停用状态：在[模组](app:mods)标签页中启用并部署。**更新**会用最新版本替换已安装的副本，**卸载**会从库中移除该模组。

登录 GitHub 后（[账户](wiki:tab-account)），在模组页面还可以：

- 在 GitHub 上给模组加星标；
- **报告问题**：一个表单，以该 GitHub 账号在模组的仓库中创建 issue，可选择附上 Obsidian 问题报告。回复会显示在[账户](app:account)的报告标签页中。

## 将模组加入商店 {#adding-a-mod-to-the-store}

商店读取一个公开列表，即 [Obsidian Store Index](https://github.com/Alomare/ObsidianStoreIndex)。仓库出现在该列表中时，模组就会进入商店：

1. 在公开的 GitHub 仓库中发布模组，根目录放置 `obsidian.json`（见[obsidian.json 文件](wiki:manifest)）。
2. 创建以 `v` 加版本号为标签的发布版本，例如 `v1.0.0`，并附上模组 zip。商店会安装最新的稳定版。
3. 可选：在仓库根目录添加 `thumbnail.jpg`（16:9），并在清单中用 `image` 指定图标。README 会成为模组页面上的描述。
4. 使用拥有该仓库的 GitHub 账号登录，在商店中点击**提交模组**。Obsidian 会检查仓库、列出所有问题，然后向 Store Index 提交 pull request 等待审核。手动在 Store Index 上提交 pull request 或 issue 也可以。

索引每小时根据发布版本重新生成一次，所以新发布的版本会在一小时内出现在商店中。Obsidian 认证由 Store Index 授予它信任的 GitHub 账号：标签跟随拥有仓库的账号，而不是清单中列出的作者。
