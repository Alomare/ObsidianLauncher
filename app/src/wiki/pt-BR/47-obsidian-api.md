---
title: API Obsidian (Obsidian API)
group: Criar mods
summary: "Uma biblioteca para mods: os dados do jogo por nome, mudanças verificadas e ações prontas."
---

O Obsidian API é um mod sobre o qual outros mods são construídos. Ele sabe onde as coisas ficam no jogo, faz mudanças do jeito que o jogo aceita e as desfaz quando precisa. É assim que o Obsidian Trainer funciona, e o trainer não usa nada que você não possa usar.

## Como usar {#using-it}

Coloque-o em `requires`. O Obsidian liga o API junto com o seu mod e o inicia primeiro:

```json
"scope": "session",
"entry": "main.lua",
"requires": ["obsidian.api"]
```

O Obsidian API é uma [biblioteca](wiki:session-mods#libraries). Um mod de cliente pode exigi-lo e continuar sendo de cliente, sem efeito na busca de partidas nem na Guerra Galáctica. Nesse caso, ele pode ler os dados do jogo e usar `api.ui`, `api.audio` e `api.events`, e tudo que muda o jogo compartilhado responde `false, "needs_session"`. Para usar essas funções, faça do seu mod um mod de sessão.

Depois peça por ele:

```lua
local api = obsidian.import("obsidian.api")

obsidian.keybind("panic", function()
  api.player.heal()
  api.enemies.kill_near(20)
end)
```

As ações respondem `true`, ou `false` e um motivo curto, como `"host_only"`, `"no_avatar"` ou `"not_in_mission"`.

## Mudar dados do jogo {#edits}

`api.edits` muda tabelas de configuração e registra o que mudou, quem mudou e como era antes.

```lua
-- Dobra o dano do fuzil de assalto, com melhorias.
local ok, why = api.edits.set_unit("equipment/wep_assaultrifle_01", "weapon_info.power", 150)

-- Um valor em qualquer tabela que o jogo lê.
local s = api.catalog.stratagem("stratagem_resupply")
api.edits.set_table("Resupply", s, "cooldown_time", 30)
```

- Tipos e limites são verificados, e `opts` aceita `min` e `max`.
- As tabelas continuam as mesmas, porque unidades vivas guardam referências a elas.
- Uma arma tem uma cópia das configurações para cada melhoria, e a edição alcança todas.
- `api.edits.list()`, `revert(id)` e `revert_all()` mostram e desfazem mudanças.
- `opts.also` em `set_table` lista mais caminhos que recebem o mesmo valor, como uma só mudança.
- Quando o lobby deixa de estar liberado, cada edição é desfeita. Quando libera de novo, elas voltam.

Os níveis de melhoria de um estratagema podem definir um campo por conta própria, então mudar a base para nesses níveis. `api.stratagems.edit` muda todos os níveis de uma vez, com os limites do campo:

```lua
api.stratagems.edit("stratagem_railcannon_strike", "cooldown_time", 30)
```

Os campos editáveis, com limites e quando cada um vale, estão em `api.catalog.fields(family)`. `api.catalog.families_of(settings)` diz quais famílias uma tabela de configuração tem. Alguns números valem na hora e outros no próximo surgimento. `api.gear.refresh()` leva os guardados para as armas em jogo.

## Efeitos {#effects}

Um efeito continua ligado entre reaparições e missões. O jogo reconstrói boa parte do estado nesses momentos, então a biblioteca aplica de novo os efeitos ativos.

```lua
api.effects.set("god", true)
api.effects.set("speed", 2)     -- um valor, false para desligar
api.effects.is_on("god")
```

Os seus:

```lua
api.effects.define({
  id = "mymod.low_gravity",
  apply = function(ctx, value) end,            -- ao ligar, e de novo após reaparecer ou nova missão
  frame = function(ctx, value, dt) end,        -- a cada quadro enquanto ligado
  remove = function(ctx, value) end,           -- ao desligar
})
```

`ctx` tem `state`, `name`, `record` e `unit`. Todos podem ser nil. Dê um `remove` a cada efeito, para ele sair quando o lobby deixar de estar liberado.

## Eventos {#events}

`api.events.on(name, fn)` chama a sua função quando o jogo avisa que algo aconteceu, então você não precisa olhar a cada quadro.

```lua
local kills = 0

api.events.on("on_enemy_killed", function(unit, attacker, owner)
  kills = kills + 1
  if kills == 50 then api.ui.hint("Fifty down") end
end)
```

| Evento | Sua função recebe | Quando |
|---|---|---|
| `on_enemy_killed` | `unit`, `attacker`, `owner`, `damage_type` | Este jogador abateu um inimigo. `damage_type` só vem quando este jogo calculou o golpe |
| `on_enemy_spawned` | `unit`, `enemy_name` | Um inimigo surgiu, no jogo de cada jogador |
| `on_player_death` | `unit` | Um jogador morreu, no jogo de cada jogador |
| `on_objective_completed` | nada | Um objetivo foi concluído |
| `on_stratagem_activated` | `peer`, `stratagem`, `position`, `unit` | A contagem de um estratagema terminou, no jogo de cada jogador |
| `on_avatar_extract` | `unit`, `shuttle` | Este jogador embarcou na nave de extração |
| `on_killed` | `unit`, `attacker`, `owner` | Uma unidade com vida foi morta |
| `on_dead` | `unit` | A morte de uma unidade terminou |
| `on_health_downed` | `unit`, `state` | Um jogador caiu |
| `on_damage_taken` | `unit`, `hit` | Uma unidade perdeu vida. `hit` tem `damage`, `hit_zone`, `attacker_unit` e `owner_unit` |
| `on_health_received` | `unit`, `amount` | Uma unidade foi curada. Uma cura total não passa nenhum dos dois |
| `on_stratagem_pre_activated` | `stratagem`, `position`, `owner`, `flare` | A contagem de um sinalizador começou |
| `on_stratagem_used` | `unit`, `data` | Um jogador usou um estratagema |
| `on_stratagem_wielded` | `unit`, `flare`, `info` | Um jogador pegou um sinalizador na mão |
| `on_extract_shuttle_landed` | `unit` | A nave de extração pousou |
| `on_add_to_stat` | `unit`, `stat`, `amount` | Uma estatística subiu, como `"stratagems_used"` ou `"hits"`. Este vem com frequência |
| `on_pause` | `paused` | O menu de pausa abriu ou fechou |
| `on_mission_end` | `stats`, `extra` | As telas finais têm os resultados |

- Uma função que falha conta contra o seu mod, e as outras continuam rodando.
- Nada chega enquanto o lobby não estiver liberado.
- `position` e `hit` duram um quadro. Copie o que precisar deles.
- `api.events.off(name, fn)` cancela uma.

## O que há na API {#contents}

| Tabela | Para |
|---|---|
| `api.game` | O estado atual, o jogador local, `is_host()` e escritas seguras |
| `api.catalog` | Armas, estratagemas, vantagens, inimigos e grupos por nome: o jogo base, mais as DLCs que o jogador tem. `usable(name)` verifica isso, `unowned_path(path)` verifica o caminho de uma arma, e `unlocked(name)` diz se o jogador desbloqueou um item jogando. Também `backpacks`, `crates`, `pickups`, `vehicles`, `by_subtype(name)` e `max_upgrade(path)` |
| `api.player` | `heal`, `heal_by(amount)`, `set_health(n)`, `respawn`, `teleport`, `set_cosmetic`, `set_outline(r, g, b)`, `clear_status(name)`, `immune(name, on)`, `knockdown`, `drop_carried` e mais |
| `api.gear` | `refill`, `give_weapon(slot, path, upgrade)`, `give_backpack(path, upgrade)`, `give_perk`, `remove_perk`, `refresh`, `weapons_of(owner)`, `tune(unit, component, changes)`, `add_drone(path, unit, upgrade)` |
| `api.stratagems` | `reset_cooldowns`, `refill_uses`, `give`, `drop`, `call(name, pos, delay)`, `edit(name, field, value)` |
| `api.enemies` | `spawn`, `spawn_group`, `spawn_entity(path, pos)`, `kill_all`, `kill(unit)`, `list`, `info(unit)`, `nearest(pos)`, `apply_status(unit, template)`, `stun_near(radius)`, `near(radius, pos)`, `damage(unit, power, statuses)`, `set_hitpoints(unit, n)`, `despawn(unit)`, `load_faction(name)`, `set_pressure(t)`, `waves()`, `force_wave(name, pos)`, `carried()`, os inimigos no mapa com cadáveres e a fila de surgimento, e `room()`, quantos mais cabem abaixo de `MAX_SAFE` |
| `api.mission` | `finish_objectives`, `objectives()`, `mutator(name)`, `win`, `abort`, `practice` |
| `api.vehicles` | `current()`, `driving()`, `repair()`, `refill()` |
| `api.combat` | `scale{ players, enemies }` |
| `api.events` | `on(name, fn)`, `off(name, fn)` |
| `api.ui` | `hint(text, seconds)`, `ping(pos)`, `open(name, opts)`, `close()`, `menus()`, `with_texts(texts, fn, ...)` |
| `api.map` | `is_open(unit)`, `open()`, `close()`, `zoom()`, `set_zoom(z)`, `view()`, `cursor()`, `look_at(pos)`, `pings()`, `to_map(pos)`, `to_world(x, y)`, `marker(pos, opts)`, `clear_markers()` |
| `api.bots` | `add(opts)`, `all()`, `remove_all()` |
| `api.lobby` | `players()`, `share(name, value)`, `shared(name)` |
| `api.world` | `cursor`, `ahead`, `to_screen(pos)`, `prop(unit, pos, opts)`, `explode`, `camera`, `shading(name, kind, value)`, `flash(seconds)` |
| `api.audio` | `bus(name, db)`, `play(event, pos)`, `stinger(name)` |
| `api.progress` | `set_samples`, `unlock_items`, `upgrade_all`, `loadout()`, `set_loadout(t)`, `upgrades()`, `set_upgrade(name, value)`, `upgrade_limit(name)` |
| `api.console` | `run(line)`, `commands()` |

## Jogador {#player}

Estes efeitos continuam depois de um respawn. A maioria devolve o que havia antes, então o valor de uma vantagem volta quando eles saem.

| Efeito | Valor |
|---|---|
| `armor` | Armadura em todos os ângulos |
| `sprint_speed` | Velocidade máxima de corrida |
| `throw_range` | Um multiplicador nos arremessos |
| `acceleration`, `deceleration` | A rapidez com que o jogador arranca e para |
| `ignore_slow` | `true`: o multiplicador de lentidão do movimento não vale, então nada que age por ele deixa o jogador lento |
| `no_screen_leash` | `true`: a câmera para de prender o jogador na tela |
| `hide_model` | `true`: o modelo do jogador fica escondido |
| `heavy_armor_look` | `true`: o visual da vantagem, sem a armadura dela |

- `api.player.set_outline(r, g, b)` muda o contorno que o jogador mostra através de paredes, e `set_outline(nil)` devolve.
- `api.player.clear_status(name)` tira um modelo de status, ou todos os status de um elemento como `"fire"`. `api.player.immune(name, on)` mantém um deles fora depois de um respawn, até `on` ser `false`.
- `set_cosmetic("gender", index)` muda o personagem, com um índice de `api.catalog.cosmetics("gender")`.

## Equipamento {#gear}

`upgrade` em `give_weapon` e `give_backpack` vai de 0 a 15. Deixe de fora para usar o que o jogador tem para aquele item. `api.catalog.max_upgrade(path)` dá o maior valor.

`give_backpack` só aceita caminhos de `api.catalog.backpacks()`, porque o jogo lê configurações que outros caminhos não têm, num ponto em que nada captura o erro. A mochila antiga cai no chão, como a arma antiga.

`give_perk` troca vantagens do jeito que o console do jogo troca. A mira laser responde `"alone_only"` com outros jogadores no lobby, porque o jogo deles a rodaria sem preparo.

### Estatísticas de uma arma {#one-weapon-s-stats}

Edições de `api.data` mudam o tipo de uma arma para todos naquela máquina. `api.gear.weapons_of(owner)` lista as armas que um jogador tem, como `{ unit, component, slot, info }`, e `api.gear.tune(unit, component, changes)` muda só uma delas. `changes` guarda um fator para um número (`firerate = 1.2`) e `true` ou `false` para um atributo (`armorpiercing = true`). Cada chamada substitui a anterior, e `{}` tira tudo. Só alcança as armas que esta máquina controla, então o jogo de cada jogador ajusta as suas.

O componente `"melee"` ajusta o ataque corpo a corpo da mesma arma: `power`, `duration` (quanto tempo o golpe prende o jogador, então um valor menor golpeia mais rápido) e `delay` (quão cedo o golpe acerta).

```lua
for _, w in ipairs(api.gear.weapons_of()) do
  api.gear.tune(w.unit, w.component, { firerate = 1.2 })
end
```

### Drones sem mochila {#drones-without-a-backpack}

`api.gear.add_drone(path, unit, upgrade)` dá a um jogador os drones de uma mochila de drone e deixa o espaço de mochila livre. `path` é uma mochila de drone de `api.catalog.backpacks()`, como `equipment/equipment_drone_defensive_backpack_01` ou `equipment/equipment_drone_heal_backpack_01`. Os drones precisam da estação da mochila para pousar, recarregar e voar em volta, então a estação vai no jogador como uma mochila vestida, e todo jogo que roda o Obsidian API a esconde. Chame de novo para ter mais drones. Responde um drone com `:alive()` e `:remove()`. Os drones pousam enquanto o jogador está num veículo e somem quando ele morre, como uma mochila que cai, então dê de novo depois de um respawn.

```lua
local dog = api.gear.add_drone("equipment/equipment_drone_defensive_backpack_01")
```

## Estratagemas {#stratagems}

`api.stratagems.call` coloca um sinalizador real, como o jogo faz nas próprias entregas programadas. Tem som e contagem, as patrulhas ouvem, e funciona para Reforço. Não gasta recarga nem uso, e chama no nível que o jogador tem. Um quarto argumento, uma unidade, chama para outro jogador desta máquina: a chamada e as mortes que ela causar são dele.

```lua
api.stratagems.call("stratagem_resupply", api.world.cursor(), 15)   -- 15 s além da própria chamada
```

`api.stratagems.drop` é o rápido: sem sinalizador, sem espera e sem Reforço.

O efeito `ignore_jammers` deixa estratagemas funcionarem dentro de zonas sem estratagemas.

## Inimigos {#enemies}

```lua
local bug, metres = api.enemies.nearest()
if bug then api.enemies.apply_status(bug, "flashbang_stun") end

-- Lutas maiores na mesma missão. false devolve um número ao jogo.
api.enemies.set_pressure({ cap = 24, difficulty = 15, patrols = 20, interval = 4 })
api.enemies.set_pressure({ difficulty = false })
```

- `apply_status` funciona em qualquer inimigo, no jogo de qualquer jogador. Os nomes de status estão em `api.catalog.status_templates()`.
- `info(unit)` dá `key`, `name`, `faction`, `boss`, `hitpoints`, `max_hitpoints` e `owned`.
- `set_pressure` é do host. `cap` vai até 32 e levanta os dois limites em que as ondas param. Cadáveres contam.
- `difficulty` é quanto cada encontro pode gastar. A semente, as ondas disponíveis e os objetivos continuam iguais.
- `force_wave` cria uma onda de `api.enemies.waves()` de uma vez, ignorando os atrasos dela.
- `spawn(key, count, where)` aceita `where` como `"cursor"`, uma posição (um vetor ou `{ x, y, z }`), ou nada para a frente do jogador.
- `spawn_entity` cria uma caixa ou um item no chão. Use caminhos de `api.catalog.crates()` e `pickups()`, que deixam de fora itens pagos.
- `near(radius, pos)` lista inimigos vivos, do mais perto ao mais longe, em volta de uma posição ou do jogador.
- `damage(unit, power, statuses)` é um golpe deste jogador que funciona no jogo de qualquer jogador, e a armadura conta. `statuses` é uma lista opcional de nomes de status.
- `set_hitpoints(unit, n)` e `despawn(unit)` passam pelo jogo do jogador dono do inimigo.
- `load_faction(name)` carrega `"bugs"`, `"cyborgs"` ou `"illuminates"` durante uma missão, para qualquer um deles aparecer em qualquer planeta. Responde `"alone_only"` com mais alguém no lobby, porque todos os jogos precisam do pacote. O jogo para por um instante enquanto carrega, e o pacote sai quando a missão termina.

## Missões {#missions}

`api.mission.objectives()` lista `name`, `text`, `type`, `state`, `group`, `done`, `total` e `position`. `state` é `pending`, `complete` ou `failed`.

O efeito `protect_objectives` mantém comboios e pessoas escoltadas vivos. Ele alcança o que roda no jogo deste jogador, então use no host.

`api.mission.mutator(name)` liga um dos modificadores do campo de provas do jogo em qualquer missão, e `nil` desliga. É um por vez, e um campo de provas real mantém o dele.

| Nome | O que faz |
|---|---|
| `super_boots` | 40% mais rápido a pé |
| `what_goes_around` | Dano em um colega volta para quem atirou |
| `orbital_support` | Estratagemas de apoio caem perto de um jogador a cada 20 s |
| `orbital_bombardments` | Ataques caem perto de um jogador a cada 15 s, mais a chamada deles |

## Veículos {#vehicles}

Estes efeitos alcançam o veículo que o jogador está dirigindo e o soltam quando ele sai. O jogo de um passageiro não consegue mudá-lo.

| Efeito | Valor |
|---|---|
| `vehicle_speed` | Um multiplicador |
| `vehicle_turn` | Um multiplicador |
| `vehicle_armor` | Armadura em todos os ângulos |
| `vehicle_god` | `true`: sem dano |
| `vehicle_rockets` | `true`: os foguetes do exotraje nunca acabam |

Os campos da família `vehicle` mudam um veículo de vez. A velocidade máxima vale quando o motorista sai e entra de novo.

## Dano {#damage}

```lua
api.combat.scale({ players = 0.5, enemies = 2 })
```

Um multiplicador de 0 a 100, depois da armadura, então a armadura que para um tiro continua parando. 1 ou `false` desliga um lado. O jogo não passa quem atacou para as contas, então a escala vale por quem leva o golpe. O jogo de cada jogador calcula os próprios golpes, então todos no lobby devem usar os mesmos números.

## Câmera e visual {#view}

Tudo isto é só deste jogador: ninguém mais vê. A Obsidian API é um mod de sessão, então para enquanto o lobby não estiver liberado.

```lua
api.world.camera.fov(90)                    -- graus, nil devolve
api.world.camera.spin(30)                   -- graus em volta do jogador
api.world.camera.spectate(friend_unit)      -- nil volta
api.world.camera.photo(true)                -- congela a câmera e limpa a tela
api.world.camera.place(position, rotation)  -- move a câmera, no modo foto

api.world.shading("fog_color", "vector3", { 0.5, 0.45, 0.3 })
api.world.shading("fog_color", "vector3")   -- volta ao do planeta
api.world.flash()                           -- relâmpago
```

- Um giro ou ângulo baixo pode deixar a mira estranha: o mouse mira no espaço da tela.
- `spectate` recebe a unidade de outro jogador, de `api.game.all_avatars()`.
- `shading` aceita os nomes em `api.world.SHADING`, por tipo: `scalar` é um número, `vector2` e `vector3` são listas. Um valor volta em cada nova missão. O jogo só traz o código para `fog_color`, `sun_color` e os outros nomes `vector3`, sem usá-los, então confira na primeira vez.
- `flash` precisa de um planeta de floresta, e não dos illuminates. Os outros respondem `"wrong_environment"`.

## Compartilhar entre máquinas {#lobby}

`api.lobby.share(name, value)` coloca um texto de até 200 caracteres onde todas as outras máquinas do lobby podem lê-lo, sob o id do seu mod. `api.lobby.shared(name)` responde o que cada outra máquina compartilha, por peer. A Steam repassa uma mudança em cerca de um segundo, então serve para um estado que muda de vez em quando, como uma pontuação, não a cada quadro. Sem lobby, `share` responde `false, "not_available"`.

```lua
api.lobby.share("score", tostring(my_score))
for peer, text in pairs(api.lobby.shared("score")) do total = total + tonumber(text) end
```

`api.world.to_screen(pos)` responde onde um ponto do mapa fica na tela, em pixels a partir do canto inferior esquerdo, e se está à vista, para desenhar por cima do mundo com um kit.

## Props {#props}

`api.world.prop(unit, pos, opts)` coloca uma unidade do jogo no mapa só nesta máquina. Nada vai pela rede e nenhum sistema do jogo sabe dela, então ela só serve para ser vista, com o brilho e o destaque que a unidade tiver. `opts.scale` define o tamanho, 1 como foi feita. Responde um prop com `move(pos)`, `scale(size)`, `flow(event)`, `alive()` e `remove()`, ou `nil` e um motivo. `flow` roda um dos eventos de flow da própria unidade, do jeito que o jogo faz uma unidade tocar seus sons e efeitos: `on_picked_up` numa amostra de pesquisa toca a coleta.

```lua
local sample = api.world.prop("gameobjects_spec/gameobject_bug_research_object", pos, { scale = 2 })
sample:move(new_pos)
sample:remove()
```

A unidade precisa estar num pacote que o mapa carregou: o jogo para quando não encontra uma. As amostras de pesquisa de uma missão, `gameobjects/flare_01` e os inimigos de uma facção carregada são seguros.

`api.ui.with_texts(texts, fn, ...)` roda `fn` com algumas chaves de texto do jogo mostrando suas palavras: `texts` é `{ key = words }`. Só vale enquanto `fn` roda, então você pode envolver uma das telas do jogo e reusá-la como sua, enquanto todo o resto do texto continua sendo o do jogo.

```lua
api.ui.with_texts({ menu_campaign = "SURVIVORS" }, original, self, ...)
```

## Som {#sound}

O som também é local.

```lua
api.audio.bus("combat_enemy", -30)   -- decibéis; 0 é como mixado, -60 é silêncio
api.audio.bus("combat_enemy", false) -- volta à mixagem do jogo

api.audio.play("ui_select")          -- em todo lugar
api.audio.play("ui_select", position)
api.audio.stinger("music_objective_found")
```

- `api.audio.BUSES` lista os barramentos. Um volume se repete a cada quadro, porque os estados de mixagem do jogo os escrevem enquanto fazem a transição.
- `play` e `stinger` respondem `"unknown_event"` para um som que esta missão não carregou.
- `stinger` passa pela recarga de 5 segundos do próprio jogo.

## Equipamentos salvos {#loadouts}

```lua
api.progress.set_loadout({ primary_weapon = "weapon_railgun", stratagems = { [1] = "stratagem_resupply" } })
api.progress.set_upgrade("weapon_railgun", 3)
```

Os dois escrevem no save real, na nave, com os menus fechados, e respondem `"close_screen"` fora disso.

- `set_loadout` aceita `perk`, `primary_weapon`, `sidearm_weapon` e `stratagems`, que liga os quatro espaços livres a nomes. Vale no próximo respawn, e entra também na primeira predefinição, porque a tela de equipamento começa por ela.
- Itens pagos e nomes desconhecidos voltam ao padrão, como quando o jogo lê um save. Um item que o jogador não desbloqueou não passa pela tela de equipamento.
- Um valor em `set_upgrade` é o que o jogo guarda: um bit por melhoria nas armas, um nível nos estratagemas. `upgrade_limit(name)` dá o maior.

## Mensagens {#messages}

`api.ui.hint(text, seconds)` mostra a caixa de dicas do próprio jogo, mesmo com as dicas desligadas. `text` são as suas palavras ou o nome de uma dica, como `"hint_alarm"`. As suas palavras passam por [`obsidian.notify`](wiki:loader-api#obsidian-notify-text-seconds), então pode responder `false, "queue_full"`. `api.ui.ping(pos)` coloca a marca deste jogador no mapa em uma posição, e `api.ui.ping(nil)` tira. Todos os jogadores veem.

`api.ui.open(name, opts)` abre um menu da nave pelo nome, como um terminal faz, e `api.ui.close()` fecha o de cima. `api.ui.menus()` lista os nomes. `opts.seed` dá uma missão ao Loadout e ao briefing, e `opts.pad` abre para um controle que não seja o do jogador principal.

`api.lobby.players()` lista `peer`, `name`, `ping` em milissegundos, `host` e `me`. Só lê. Um colega tem nome durante uma missão.

## O mapa {#map}

`api.map` lê e controla o mapa da missão do jogador local. Tudo aqui muda só o que esse jogador vê, então um mod de cliente pode usar.

- `is_open(unit)` diz se um mapa está aberto, o do jogador local quando `unit` é nil. `open()` e `close()` fazem o que a tecla do mapa faz. Um mapa aberto assim fica aberto até o jogador apertar e soltar a tecla do mapa, fazer outra coisa como esquivar ou recarregar, ou até `close()` ser chamado.
- `zoom()` e `set_zoom(z)` leem e definem o zoom, de 0.5 a 4. `view()` é o ponto do mundo sob a mira, onde um ping cairia, e `cursor()` é outro nome para ele. `look_at(pos)` move a visão para deixar um ponto sob a mira.
- `pings()` lista o ping de cada jogador como `{ record, owned, position }`. `api.ui.ping(pos)` define o do jogador local.
- `to_map(pos)` e `to_world(x, y)` convertem entre posições do mundo e as unidades do mapa no zoom 1.

`marker(pos, opts)` coloca um marcador do mod no mapa desse jogador, e ele some quando a missão termina. `opts` aceita `icon` (um material que o mapa carregou, `icon_map_pickup_generic` por padrão), `size`, `color` como `{ a, r, g, b }` e `unit` para seguir uma unidade em vez de ficar em `pos`.

```lua
local m = api.map.marker(pos, { icon = "icon_map_objective_bunker_01", size = 32, color = { 255, 40, 255, 40 } })
m:move(new_pos)
m:remove()
```

## Bots {#bots}

`api.bots.add(opts)` cria um Helldiver que esta máquina controla. Precisa ser o host ou um jogador sozinho, numa missão, e responde `nil, "host_only"` ou `nil, "not_in_mission"` nos outros casos. `opts` aceita `name`, `bare` e `position`.

Um bot é primeiro um jogador local num controle que ninguém usa, então tem espaço no HUD, nome, revives e reforços. Os outros jogadores veem um jogador comum de co-op local. Com os quatro registros de jogador ocupados, o bot é um Helldiver avulso: uma unidade com o nome sobre a cabeça, sem espaço no HUD nem respawn. `bare = true` pede um desses, e `bare = false` responde `nil, "no_slot"` em vez disso. Um bot avulso precisa de um número de controle do jogo que ninguém use para ser controlado, e fica parado quando não sobra nenhum.

```lua
local bot = api.bots.add({ name = "Dummy" })
bot:follow(nil, 6)   -- fica a até 6 metros do jogador
bot:fight(25)        -- mira e atira no inimigo mais próximo a até 25 metros
```

Para controlar você mesmo: `move(dir)`, `aim(dir)`, `fire(on)`, `sprint(on)`, `reload()`, `dodge()`, `melee()`, `interact()`, `throw()` e `switch()`. Direções são direções do mundo como `{ x, y }`. `unit()`, `alive()` e `remove()` fazem o que dizem. Bots andam em linha reta até o objetivo, sem busca de caminho, e o jogo mantém jogadores locais na tela, então um bot jogador para na borda da visão.

Os bots saem antes dos resultados da missão, então nunca chegam às telas finais nem ao save de ninguém.

## Regras importantes {#rules}

> [!WARNING]
> O jogo guarda cópias com hash de valores protegidos, como `hitpoints_real`, `rounds`, `cooldown_timer`, `uses_left`, `num_grenades` e `extra_lives`. Escrever um deles diretamente dispara a verificação anti-adulteração, que corta a conexão do jogador até reiniciar. Use `api.game.protected` ou um comando na fila. `api.edits` recusa essas chaves.

- `api.game.command(unit, component, command, data)` põe na fila um comando de uma lista conhecida, numa unidade deste jogador. Um comando desconhecido direto no jogo o para com um erro que nenhum script captura.
- Um temporizador que o jogo só roda acima de zero, como o `drop_time` de um hellpod, nunca termina em 0. Use 0.01.
- Criar uma unidade cujo pacote não está carregado faz o jogo parar. `api.enemies.available(enemy)` verifica.
- `api.console.run` recusa comandos que alcançam o backend compartilhado, apagam saves ou liberam conteúdo pago. `spawn_enemy` só roda sem mais ninguém no lobby, porque carrega um pacote apenas no jogo deste jogador.
- Pense bem antes de mudar o XP: o total vai junto com os próximos resultados normais, mesmo depois de uma limpeza, e chega aos placares compartilhados.
