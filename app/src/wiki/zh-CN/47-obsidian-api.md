---
title: Obsidian 模组 API
group: 制作模组
summary: "模组库：按名称访问游戏数据、受检查的修改和现成操作。"
---

Obsidian API 是供其他模组使用的模组。它知道游戏中各项内容的位置，以游戏接受的方式进行修改，并在需要时撤销。Obsidian Trainer 就是这样工作的，它没有使用任何你用不到的功能。

## 使用方法 {#using-it}

把它写进 `requires`，Obsidian 会随你的模组一同启用它，并先启动它：

```json
"scope": "session",
"entry": "main.lua",
"requires": ["obsidian.api"]
```

Obsidian API 是一个[库](wiki:session-mods#libraries)。客户端模组可以依赖它并保持客户端身份，不影响匹配和银河战争。这种情况下，它可以读取游戏数据并使用 `api.ui`、`api.audio` 和 `api.events`，而任何会改变共享游戏的操作都返回 `false, "needs_session"`。要使用这些操作，请把你的模组设为会话模组。

然后获取它：

```lua
local api = obsidian.import("obsidian.api")

obsidian.keybind("panic", function()
  api.player.heal()
  api.enemies.kill_near(20)
end)
```

操作返回 `true`，或者返回 `false` 和简短的原因，例如 `"host_only"`、`"no_avatar"` 或 `"not_in_mission"`。

## 修改游戏数据 {#edits}

`api.edits` 修改设置表，并记录改了什么、谁改的以及原来的值。

```lua
-- 将突击步枪的伤害翻倍，包括升级版本。
local ok, why = api.edits.set_unit("equipment/wep_assaultrifle_01", "weapon_info.power", 150)

-- 修改游戏读取的任意表中的值。
local s = api.catalog.stratagem("stratagem_resupply")
api.edits.set_table("Resupply", s, "cooldown_time", 30)
```

- 会检查类型和范围，`opts` 可以提供 `min` 和 `max`。
- 表保持为同一张表，因为游戏中的单位持有对它们的引用。
- 武器的每种升级都有一份设置副本，修改会覆盖所有副本。
- `api.edits.list()`、`revert(id)` 和 `revert_all()` 用于查看和撤销更改。
- `set_table` 的 `opts.also` 可以列出更多路径，它们取同一个值，算作一次更改。
- 当大厅不再处于许可状态时，所有修改都会撤销；重新许可后会恢复。

战略配备的升级等级可以自己设置某个字段，所以只改基础值在这些等级上不起作用。`api.stratagems.edit` 会一次修改所有等级，并使用该字段的范围：

```lua
api.stratagems.edit("stratagem_railcannon_strike", "cooldown_time", 30)
```

可编辑的字段、范围和生效时机见 `api.catalog.fields(family)`。`api.catalog.families_of(settings)` 说明设置表包含哪些类别。有些数值立即生效，有些在下次生成时生效。`api.gear.refresh()` 会把缓存的数值应用到正在使用的武器上。

## 效果 {#effects}

效果在重生和任务之间保持开启。游戏这时会重建大部分状态，所以它会重新应用已启用的效果。

```lua
api.effects.set("god", true)
api.effects.set("speed", 2)     -- 一个数值，false 表示关闭
api.effects.is_on("god")
```

自定义效果：

```lua
api.effects.define({
  id = "mymod.low_gravity",
  apply = function(ctx, value) end,            -- 开启时，以及重生或新任务后再次调用
  frame = function(ctx, value, dt) end,        -- 开启期间每帧调用
  remove = function(ctx, value) end,           -- 关闭时
})
```

`ctx` 包含 `state`、`name`、`record` 和 `unit`，它们都可能为 nil。请为每个效果提供 `remove`，这样大厅不再处于许可状态时它才能撤销。

## 事件 {#events}

`api.events.on(name, fn)` 会在游戏通知某事发生时调用你的函数，无需每帧检查。

```lua
local kills = 0

api.events.on("on_enemy_killed", function(unit, attacker, owner)
  kills = kills + 1
  if kills == 50 then api.ui.hint("已消灭五十个") end
end)
```

| 事件 | 函数收到的参数 | 时机 |
|---|---|---|
| `on_enemy_killed` | `unit`、`attacker`、`owner`、`damage_type` | 本玩家击杀了敌人。只有本机计算这次命中时才有 `damage_type` |
| `on_enemy_spawned` | `unit`、`enemy_name` | 敌人出现，每位玩家的游戏都会收到 |
| `on_player_death` | `unit` | 玩家死亡，每位玩家的游戏都会收到 |
| `on_objective_completed` | 无 | 目标完成 |
| `on_stratagem_activated` | `peer`、`stratagem`、`position`、`unit` | 战略配备倒计时结束，每位玩家的游戏都会收到 |
| `on_avatar_extract` | `unit`、`shuttle` | 本玩家登上撤离飞船 |
| `on_killed` | `unit`、`attacker`、`owner` | 有生命值的单位被杀死 |
| `on_dead` | `unit` | 单位的死亡过程结束 |
| `on_health_downed` | `unit`、`state` | 玩家倒地 |
| `on_damage_taken` | `unit`、`hit` | 单位损失生命值。`hit` 包含 `damage`、`hit_zone`、`attacker_unit` 和 `owner_unit` |
| `on_health_received` | `unit`、`amount` | 单位得到治疗。完全治疗时两者都不传 |
| `on_stratagem_pre_activated` | `stratagem`、`position`、`owner`、`flare` | 信标开始倒计时 |
| `on_stratagem_used` | `unit`、`data` | 玩家使用战略配备 |
| `on_stratagem_wielded` | `unit`、`flare`、`info` | 玩家手持信标 |
| `on_extract_shuttle_landed` | `unit` | 撤离飞船着陆 |
| `on_add_to_stat` | `unit`、`stat`、`amount` | 统计数据增加，例如 `"stratagems_used"` 或 `"hits"`。此事件很频繁 |
| `on_pause` | `paused` | 暂停菜单打开或关闭 |
| `on_mission_end` | `stats`、`extra` | 结算界面已有结果 |

- 函数出错时会计入你的模组，其他函数继续运行。
- 大厅未处于许可状态时不会收到任何事件。
- `position` 和 `hit` 只在当前帧有效。请复制需要的内容。
- `api.events.off(name, fn)` 用于取消。

## API 内容 {#contents}

| 表 | 用途 |
|---|---|
| `api.game` | 当前状态、本地玩家、`is_host()` 和安全写入 |
| `api.catalog` | 按名称列出武器、战略配备、技能、敌人和敌群：基础游戏，加上玩家拥有的 DLC。`usable(name)` 检查这一点，`unowned_path(path)` 检查武器的路径，`unlocked(name)` 说明玩家是否通过游玩解锁了某个物品。还有 `backpacks`、`crates`、`pickups`、`vehicles`、`by_subtype(name)` 和 `max_upgrade(path)` |
| `api.player` | `heal`、`heal_by(amount)`、`set_health(n)`、`respawn`、`teleport`、`set_cosmetic`、`set_outline(r, g, b)`、`clear_status(name)`、`immune(name, on)`、`knockdown`、`drop_carried` 等 |
| `api.gear` | `refill`、`give_weapon(slot, path, upgrade)`、`give_backpack(path, upgrade)`、`give_perk`、`remove_perk`、`add_perk(name, unit)`、`remove_extra_perk(name, unit)`、`extra_perks(unit)`、`refresh`、`weapons_of(owner)`、`tune(unit, component, changes)`、`add_drone(path, unit, upgrade)` |
| `api.stratagems` | `reset_cooldowns`、`refill_uses`、`give`、`drop`、`call(name, pos, delay)`、`edit(name, field, value)` |
| `api.enemies` | `spawn`、`spawn_group`、`spawn_entity(path, pos)`、`kill_all`、`kill(unit)`、`list`、`info(unit)`、`nearest(pos)`、`apply_status(unit, template)`、`stun_near(radius)`、`near(radius, pos)`、`damage(unit, power, statuses)`、`set_hitpoints(unit, n)`、`despawn(unit)`、`load_faction(name)`、`set_pressure(t)`、`waves()`、`force_wave(name, pos)`、`carried()`（地图上的敌人，含尸体和生成队列）、`room()`（低于 `MAX_SAFE` 还能容纳的数量） |
| `api.mission` | `finish_objectives`、`objectives()`、`mutator(name)`、`win`、`abort`、`practice` |
| `api.vehicles` | `current()`、`driving()`、`repair()`、`refill()` |
| `api.combat` | `scale{ players, enemies }` |
| `api.events` | `on(name, fn)`、`off(name, fn)` |
| `api.ui` | `hint(text, seconds)`、`ping(pos)`、`open(name, opts)`、`close()`、`menus()`、`with_texts(texts, fn, ...)` |
| `api.map` | `is_open(unit)`, `open()`, `close()`, `zoom()`, `set_zoom(z)`, `view()`, `cursor()`, `look_at(pos)`, `pings()`, `to_map(pos)`, `to_world(x, y)`, `marker(pos, opts)`, `clear_markers()` |
| `api.bots` | `add(opts)`, `all()`, `remove_all()` |
| `api.lobby` | `players()`、`share(name, value)`、`shared(name)` |
| `api.world` | `cursor`、`ahead`、`to_screen(pos)`、`prop(unit, pos, opts)`、`explode`、`camera`、`shading(name, kind, value)`、`flash(seconds)` |
| `api.audio` | `bus(name, db)`、`play(event, pos)`、`stinger(name)` |
| `api.progress` | `set_samples`、`unlock_items`、`upgrade_all`、`loadout()`、`set_loadout(t)`、`upgrades()`、`set_upgrade(name, value)`、`upgrade_limit(name)` |
| `api.console` | `run(line)`、`commands()` |

## 玩家 {#player}

这些效果在重生后仍然有效。大多数效果关闭时会恢复之前的值，所以技能自身的数值会回来。

| 效果 | 数值 |
|---|---|
| `armor` | 所有角度的护甲 |
| `sprint_speed` | 最高冲刺速度 |
| `throw_range` | 投掷距离倍率 |
| `acceleration`、`deceleration` | 玩家起步和停下的快慢 |
| `ignore_slow` | `true`：移动器的减速倍率不再生效，因此任何通过它起作用的东西都无法使玩家减速 |
| `no_screen_leash` | `true`：镜头不再把玩家限制在屏幕内 |
| `hide_model` | `true`：隐藏玩家模型 |
| `heavy_armor_look` | `true`：技能的外观，但没有其护甲 |

- `api.player.set_outline(r, g, b)` 更改玩家透过墙壁显示的轮廓，`set_outline(nil)` 恢复。
- `api.player.clear_status(name)` 移除一个状态模板，或某元素（如 `"fire"`）的所有状态。`api.player.immune(name, on)` 让其在重生后仍保持免疫，直到 `on` 为 `false`。
- `set_cosmetic("gender", index)` 更改角色，索引来自 `api.catalog.cosmetics("gender")`。

## 装备 {#gear}

`give_weapon` 和 `give_backpack` 的 `upgrade` 取值 0 到 15。省略时使用玩家为该物品拥有的升级。`api.catalog.max_upgrade(path)` 给出最高值。

`give_backpack` 只接受 `api.catalog.backpacks()` 中的路径，因为游戏会读取其他路径没有的设置，而且那里的错误无法捕获。旧背包会掉到地上，旧武器也一样。

`add_perk(name, unit)` 在主技能之外为玩家增加技能，配套的有 `remove_extra_perk(name, unit)` 和 `extra_perks(unit)`。死亡后的新身体不带额外技能。带每帧代码的技能（如激光瞄准）返回 `"not_available"`，因为游戏只为主技能运行这些代码。

`give_perk` 以游戏控制台的方式更换技能。大厅里有其他玩家时，激光瞄准器会返回 `"alone_only"`，因为他们的游戏会在没有初始化的情况下运行它。

### 单把武器的属性 {#one-weapon-s-stats}

`api.data` 的编辑会改变这台机器上所有人的某类武器。`api.gear.weapons_of(owner)` 列出玩家持有的武器，形式为 `{ unit, component, slot, info }`，`api.gear.tune(unit, component, changes)` 只改变其中一把。`changes` 中，数值用倍数（`firerate = 1.2`），属性用 `true` 或 `false`（`armorpiercing = true`）。每次调用都会替换上一次，`{}` 会全部取消。它只作用于这台机器拥有的武器，所以每位玩家的游戏调整自己的武器。

组件 `"melee"` 则调整同一把武器的近战攻击：`power`、`duration`（挥击让玩家停留多久，数值越小挥得越快）和 `delay`（多快命中）。

```lua
for _, w in ipairs(api.gear.weapons_of()) do
  api.gear.tune(w.unit, w.component, { firerate = 1.2 })
end
```

### 无需背包的无人机 {#drones-without-a-backpack}

`api.gear.add_drone(path, unit, upgrade)` 给玩家一个无人机背包的无人机，同时不占用背包栏位。`path` 是 `api.catalog.backpacks()` 中的无人机背包，例如 `equipment/equipment_drone_defensive_backpack_01` 或 `equipment/equipment_drone_heal_backpack_01`。无人机需要背包的基座来停靠、装弹和环绕飞行，所以基座会像背着的背包一样跟随玩家，而每个运行 Obsidian API 的游戏都会把它隐藏起来。再次调用可获得更多无人机。返回一个带有 `:alive()` 和 `:remove()` 的无人机。玩家坐在载具中时无人机会停靠，玩家死亡时它们会消失，就像背着的背包会掉落一样，所以重生后请再给一次。

```lua
local dog = api.gear.add_drone("equipment/equipment_drone_defensive_backpack_01")
```

## 战略配备 {#stratagems}

`api.stratagems.call` 会放下一枚真实的信标，和游戏自己的定时投放一样。它有声音和倒计时，巡逻队能听到，增援也可以使用。它不消耗冷却或次数，并按玩家拥有的等级呼叫。第四个参数是一个单位，可以为这台机器上的另一位玩家呼叫：呼叫及其击杀都归该玩家。

```lua
api.stratagems.call("stratagem_resupply", api.world.cursor(), 15)   -- 在原有呼叫时间上再加 15 秒
```

`api.stratagems.drop` 是快速方式：没有信标，没有等待，不能用于增援。

效果 `ignore_jammers` 让战略配备可以在禁用区域内使用。

## 敌人 {#enemies}

```lua
local bug, metres = api.enemies.nearest()
if bug then api.enemies.apply_status(bug, "flashbang_stun") end

-- 在同一任务中打更大的仗。false 把数值交还给游戏。
api.enemies.set_pressure({ cap = 24, difficulty = 15, patrols = 20, interval = 4 })
api.enemies.set_pressure({ difficulty = false })
```

- `apply_status` 可以从任何玩家的游戏作用于任何敌人。状态名称见 `api.catalog.status_templates()`。
- `info(unit)` 返回 `key`、`name`、`faction`、`boss`、`hitpoints`、`max_hitpoints` 和 `owned`。
- `set_pressure` 只属于主机。`cap` 最大为 32，并同时提高波次受限的两个上限。尸体也计入。
- `difficulty` 是每次遭遇可花费的预算。种子、可选波次和目标都不变。
- `force_wave` 一次生成 `api.enemies.waves()` 中的一个波次，忽略其延迟。
- `spawn(key, count, where)` 的 `where` 可以是 `"cursor"`、一个位置（向量或 `{ x, y, z }`），省略则在玩家前方。
- `spawn_entity` 生成补给箱或拾取物。请使用 `api.catalog.crates()` 和 `pickups()` 中的路径，它们不含付费物品。
- `near(radius, pos)` 列出某位置或玩家周围的存活敌人，由近到远。
- `damage(unit, power, statuses)` 是来自本玩家的一次命中，在任何玩家的游戏中都有效，护甲也会计入。`statuses` 是可选的状态名称列表。
- `set_hitpoints(unit, n)` 和 `despawn(unit)` 通过拥有该敌人的玩家的游戏执行。
- `load_faction(name)` 在任务中加载 `"bugs"`、`"cyborgs"` 或 `"illuminates"`，让它们可以在任何星球上生成。大厅中还有其他人时返回 `"alone_only"`，因为每个游戏都需要该资源包。加载时游戏会短暂停顿，任务结束时卸载。

## 任务 {#missions}

`api.mission.objectives()` 列出 `name`、`text`、`type`、`state`、`group`、`done`、`total` 和 `position`。`state` 为 `pending`、`complete` 或 `failed`。

效果 `protect_objectives` 让车队和护送对象不会死亡。它只作用于本玩家游戏运行的单位，所以请在主机上使用。

`api.mission.mutator(name)` 在任意任务中开启游戏的一个试炼场修改项，`nil` 表示关闭。同一时间只有一个，真实的试炼场保留自己的修改项。

| 名称 | 作用 |
|---|---|
| `super_boots` | 步行速度提高 40% |
| `what_goes_around` | 对队友造成的伤害返还给射击者 |
| `orbital_support` | 每 20 秒在玩家附近投下支援战略配备 |
| `orbital_bombardments` | 每 15 秒加上呼叫时间，在玩家附近落下打击 |

## 载具 {#vehicles}

这些效果作用于玩家正在驾驶的载具，玩家离开时解除。乘客的游戏无法更改它。

| 效果 | 数值 |
|---|---|
| `vehicle_speed` | 倍率 |
| `vehicle_turn` | 倍率 |
| `vehicle_armor` | 所有角度的护甲 |
| `vehicle_god` | `true`：不受伤害 |
| `vehicle_rockets` | `true`：机甲火箭弹永不耗尽 |

`vehicle` 类别的字段会持久修改载具。最高速度在驾驶员下车再上车后生效。

## 伤害 {#damage}

```lua
api.combat.scale({ players = 0.5, enemies = 2 })
```

倍率范围为 0 到 100，在护甲之后计算，所以能挡住射击的护甲仍然能挡住。1 或 `false` 关闭这一方。游戏的计算不提供攻击者，所以倍率按受击方区分。每位玩家的游戏各自计算命中，所以大厅中的每个人都应使用相同的数值。

## 镜头和外观 {#view}

这些内容只属于本玩家，其他人看不到。Obsidian API 是会话模组，所以大厅未处于许可状态时会停止。

```lua
api.world.camera.fov(90)                    -- 角度，nil 恢复
api.world.camera.spin(30)                   -- 绕玩家旋转的角度
api.world.camera.spectate(friend_unit)      -- nil 返回
api.world.camera.photo(true)                -- 冻结镜头并清空屏幕
api.world.camera.place(position, rotation)  -- 在照片模式中移动镜头

api.world.shading("fog_color", "vector3", { 0.5, 0.45, 0.3 })
api.world.shading("fog_color", "vector3")   -- 恢复星球的设置
api.world.flash()                           -- 闪电
```

- 较低的旋转或角度可能让瞄准感觉异常：鼠标在屏幕空间中瞄准。
- `spectate` 接受其他玩家的单位，来自 `api.game.all_avatars()`。
- `shading` 接受 `api.world.SHADING` 中按类型列出的名称：`scalar` 是数字，`vector2` 和 `vector3` 是列表。数值会在每个新任务中重新应用。对于 `fog_color`、`sun_color` 和其他 `vector3` 名称，游戏只附带代码而自身没有使用，所以请在首次运行时检查。
- `flash` 需要森林星球，且不能是 illuminates 的星球。其他情况返回 `"wrong_environment"`。

## 在机器之间共享 {#lobby}

`api.lobby.share(name, value)` 会把一段最多 200 个字符的文本放在大厅中其他每台机器都能读取的地方，归在你的模组 id 下。`api.lobby.shared(name)` 按 peer 返回其他每台机器共享的内容。Steam 大约一秒内传递一次更改，因此适合偶尔变化的状态，例如分数，而不是每一帧。没有大厅时，`share` 返回 `false, "not_available"`。

```lua
api.lobby.share("score", tostring(my_score))
for peer, text in pairs(api.lobby.shared("score")) do total = total + tonumber(text) end
```

`api.world.to_screen(pos)` 返回关卡中的一个点在屏幕上的位置（以左下角为原点的像素）以及它是否在视野内，用于用 kit 在世界上方绘制。

## 道具 {#props}

`api.world.prop(unit, pos, opts)` 只在这台机器上把游戏中的一个单位放进关卡。它不会通过网络发送，游戏系统也不知道它的存在，所以它只用来被看见，带有该单位本身的发光和高亮。`opts.scale` 设置大小，1 为原始大小。返回一个带有 `move(pos)`、`scale(size)`、`flow(event)`、`alive()` 和 `remove()` 的道具，或返回 `nil` 和原因。`flow` 运行该单位自己的一个 flow 事件，就像游戏让单位播放声音和特效那样：在研究样本上调用 `on_picked_up` 会播放拾取效果。

```lua
local sample = api.world.prop("gameobjects_spec/gameobject_bug_research_object", pos, { scale = 2 })
sample:move(new_pos)
sample:remove()
```

该单位必须位于关卡已加载的资源包中：找不到时游戏会停止。任务的研究样本、`gameobjects/flare_01` 和已加载阵营的敌人都是安全的。

`api.ui.with_texts(texts, fn, ...)` 在运行 `fn` 时让游戏的某些文本键显示你的文字：`texts` 是 `{ key = words }`。它只在 `fn` 运行期间有效，所以你可以包装游戏的某个界面并当作自己的界面重用，而其他所有文本仍是游戏原有的。

```lua
api.ui.with_texts({ menu_campaign = "SURVIVORS" }, original, self, ...)
```

## 声音 {#sound}

声音同样只在本地。

```lua
api.audio.bus("combat_enemy", -30)   -- 分贝；0 为原始混音，-60 为静音
api.audio.bus("combat_enemy", false) -- 恢复游戏的混音

api.audio.play("ui_select")          -- 全局播放
api.audio.play("ui_select", position)
api.audio.stinger("music_objective_found")
```

- `api.audio.BUSES` 列出所有总线。音量会每帧重复设置，因为游戏的混音状态在过渡时会写入它们。
- 对本任务未加载的声音，`play` 和 `stinger` 返回 `"unknown_event"`。
- `stinger` 受游戏自身 5 秒冷却的限制。

## 配装 {#loadouts}

```lua
api.progress.set_loadout({ primary_weapon = "weapon_railgun", stratagems = { [1] = "stratagem_resupply" } })
api.progress.set_upgrade("weapon_railgun", 3)
```

两者都会写入真实存档，需要在飞船上且菜单已关闭，否则返回 `"close_screen"`。

- `set_loadout` 接受 `perk`、`primary_weapon`、`sidearm_weapon` 和 `stratagems`，后者把四个空闲栏位映射到名称。它在下次重生时生效，同时也写入第一个预设，因为配装界面从它开始。
- 付费物品和未知名称会恢复为默认值，与游戏读取存档时相同。玩家未解锁的物品无法通过配装界面保留。
- `set_upgrade` 的数值就是游戏保存的值：武器每项升级一位，战略配备为等级。`upgrade_limit(name)` 给出最高值。

## 消息 {#messages}

`api.ui.hint(text, seconds)` 显示游戏自己的提示框，即使提示已关闭。`text` 是你的文字，或提示名称，例如 `"hint_alarm"`。你的文字会通过 [`obsidian.notify`](wiki:loader-api#obsidian-notify-text-seconds) 显示，因此可能返回 `false, "queue_full"`。`api.ui.ping(pos)` 在地图上的某个位置放下本玩家的标记，`api.ui.ping(nil)` 将其移除。所有玩家都能看到。

`api.ui.open(name, opts)` 按名称打开飞船上的菜单，效果与终端相同；`api.ui.close()` 关闭最上层的菜单。`api.ui.menus()` 列出所有名称。`opts.seed` 为配装和简报界面指定任务，`opts.pad` 为主玩家以外的手柄打开菜单。

`api.lobby.players()` 列出 `peer`、`name`、以毫秒为单位的 `ping`、`host` 和 `me`。它只读取数据。队友只有在任务中才有名字。

## 地图 {#map}

`api.map` 读取并控制本地玩家的任务地图。这里的一切只改变该玩家看到的内容，因此客户端模组也可以使用。

- `is_open(unit)` 返回地图是否打开，`unit` 为 nil 时指本地玩家。`open()` 和 `close()` 的效果与地图键相同。这样打开的地图会保持打开，直到玩家按下并松开地图键、执行翻滚或换弹等其他操作，或调用 `close()`。
- `zoom()` 和 `set_zoom(z)` 读取和设置缩放，范围 0.5 到 4。`view()` 是准星下的世界坐标，也就是信标会落下的位置，`cursor()` 是它的别名。`look_at(pos)` 平移视图，让某一点位于准星下。
- `pings()` 以 `{ record, owned, position }` 列出每位玩家的信标。`api.ui.ping(pos)` 设置本地玩家的信标。
- `to_map(pos)` 和 `to_world(x, y)` 在世界坐标和缩放为 1 时的地图单位之间转换。

`marker(pos, opts)` 在该玩家的地图上放置模组自己的标记，任务结束时标记消失。`opts` 接受 `icon`（地图已加载的材质，默认 `icon_map_pickup_generic`）、`size`、`{ a, r, g, b }` 形式的 `color`，以及用于跟随单位而不是停在 `pos` 的 `unit`。

```lua
local m = api.map.marker(pos, { icon = "icon_map_objective_bunker_01", size = 32, color = { 255, 40, 255, 40 } })
m:move(new_pos)
m:remove()
```

## 机器人 {#bots}

`api.bots.add(opts)` 创建一名由这台机器操控的绝地潜兵。必须在任务中，且是主机或独自一人的玩家，否则返回 `nil, "host_only"` 或 `nil, "not_in_mission"`。`opts` 接受 `name`、`bare` 和 `position`。

机器人首先是使用无人手柄的本地玩家，因此有 HUD 栏位、名字、复活和增援。其他玩家看到的是普通的本地合作玩家。四个玩家记录都被占用时，机器人是独立的绝地潜兵：头顶有名字，但没有 HUD 栏位，也不会重生。`bare = true` 直接要求这种机器人，`bare = false` 则返回 `nil, "no_slot"`。独立机器人需要一个无人使用的游戏手柄编号才能被操控，没有可用编号时会站着不动。

```lua
local bot = api.bots.add({ name = "Dummy" })
bot:follow(nil, 6)   -- 与玩家保持 6 米以内
bot:fight(25)        -- 瞄准并射击 25 米内最近的敌人
```

自行操控时可用：`move(dir)`、`aim(dir)`、`fire(on)`、`sprint(on)`、`reload()`、`dodge()`、`melee()`、`interact()`、`throw()` 和 `switch()`。方向是 `{ x, y }` 形式的世界方向。`unit()`、`alive()` 和 `remove()` 如其名。机器人直线走向目标，没有寻路，而且游戏会把本地玩家留在屏幕内，因此玩家机器人会停在视野边缘。

机器人会在任务结算前离开，因此不会进入结算画面，也不会影响任何人的存档。

## 重要规则 {#rules}

> [!WARNING]
> 游戏会为受保护的数值保留哈希副本，例如 `hitpoints_real`、`rounds`、`cooldown_timer`、`uses_left`、`num_grenades` 和 `extra_lives`。直接写入这些值会触发防篡改检查，使玩家断开网络直到重启。请使用 `api.game.protected` 或排队命令。`api.edits` 会拒绝这些键。

- `api.game.command(unit, component, command, data)` 从已知列表中向本玩家拥有的单位排队命令。直接向游戏排队未知命令会使游戏因脚本无法捕获的错误而停止。
- 游戏只在大于零时运行的计时器（例如空投舱的 `drop_time`）设为 0 时永远不会结束。请使用 0.01。
- 生成资源包未加载的单位会使游戏停止。`api.enemies.available(enemy)` 可以检查。
- `api.console.run` 会拒绝访问共享后端、清除存档或解锁付费内容的命令。`spawn_enemy` 只在大厅没有其他人时运行，因为它只在本玩家的游戏中加载资源包。
- 修改经验值之前请三思：总经验值会随之后的普通结果增加，即使清除之后也是如此，并且会进入共享排行榜。
