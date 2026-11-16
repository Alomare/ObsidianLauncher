---
title: Loader API
group: Criar mods
summary: Cada função da tabela obsidian, versão 1.
---

O Obsidian Loader inicia o script `entry` de cada mod ligado na ordem de carregamento, uma vez, quando o jogo abre. Todo script vê uma tabela global chamada `obsidian`. Tudo o que um mod registra por ela roda protegido: um erro é registrado no log e nunca para o jogo.

## Ganchos {#hooks}

### obsidian.hook(target, method, fn, opts)

Envolve uma função do jogo. `target` é uma tabela, ou o nome de uma tabela global como string. `fn` recebe primeiro a função original, depois os argumentos da chamada:

```lua
obsidian.hook("PlayerUnitMovement", "update", function(original, self, dt)
  -- before the game code
  local result = original(self, dt)
  -- after the game code
  return result
end)
```

Quando a global ainda não existe, o gancho espera e entra assim que o jogo a define. Se `fn` falha, a original roda no lugar.

Num mod de sessão, o gancho chama só a original enquanto o lobby não está liberado ([Escrever mods de sessão](wiki:session-mods)). Numa expansão, chama só a original até o conteúdo começar ([Escrever uma expansão](wiki:session-mods#expansions)). `opts.secondary_scope` põe o gancho num [escopo secundário](#hybrid-scopes) do mod.

### obsidian.wrap(target, method, fn)

O mesmo que `obsidian.hook`, com a mesma segurança se `fn` falhar, exceto que continua rodando enquanto o lobby não está liberado, e fora do conteúdo de uma expansão. Serve para o caminho de entrada de uma expansão, e para um mod cujos arquivos substituídos também não podem ser desligados, onde um gancho desligando deixaria um jogo pela metade. Todo o resto deve usar `obsidian.hook`. Confira `obsidian.capabilities.wrap` num carregador mais antigo.

### obsidian.on_require(name, fn)

Chama `fn(module)` quando o jogo carrega o script `name`, como `"lua/settings/perk_settings"`. Use para mudar uma tabela de configurações logo depois que o jogo a monta. Quando o script já carregou, `fn` roda na hora.

### obsidian.on_frame(fn, opts)

Chama `fn` uma vez por quadro. Retorne `true` para parar as chamadas. As funções de quadro de um mod de sessão só rodam num lobby liberado, e as de uma expansão só enquanto o conteúdo roda. `opts.secondary_scope` põe a função num [escopo secundário](#hybrid-scopes) do mod. `opts.always` a roda em qualquer lobby, como `obsidian.wrap` faz com um gancho, para trabalho que só este jogador vê; confira `obsidian.capabilities.secondary_scopes` antes, já que um carregador mais antigo a controla como antes.

### obsidian.after_startup(fn)

Chama `fn` quando todos os mods já começaram, para que os mods trabalhem juntos sem depender da ordem de carregamento.

## Configurações e teclas {#settings-and-keys}

### obsidian.settings.get(id, mod_id)

O valor da opção `id` deste mod: o que o jogador escolheu no app, ou o padrão ([Opções e teclas](wiki:options-and-keys)). Um script do jogo que o mod substitui roda fora do mod, então ali passe o id do mod como `mod_id`.

### obsidian.keybind(id, fn, opts)

Chama `fn(device)` a cada vez que o atalho `id` dispara, pelo [modo de entrada](wiki:options-and-keys#input-modes) que o jogador escolheu (pressionar quando nenhum). A tecla vem do app, ou do padrão em `obsidian.json`. `device` é `"key"`, ou `"pad"` quando foi a entrada de controle que disparou: a que o jogador definiu no app, senão `opts.pad`, uma função que retorna se a entrada de controle do atalho está pressionada neste quadro. As teclas de um mod de sessão esperam um lobby liberado, e as de uma expansão esperam o conteúdo, como os ganchos do mod. `opts.always` deixa a tecla em paz, como `obsidian.wrap` faz com um gancho, para uma tecla que só age na tela deste jogador. `opts.secondary_scope` põe a tecla num [escopo secundário](#hybrid-scopes) do mod.

## Multijogador {#multiplayer}

### obsidian.session.cleared(id)

`true` enquanto o mod que chama pode rodar: a sessão está liberada, e, para um mod marcado `every_player`, todos os outros jogadores também o rodam ([Mods de que todos precisam](wiki:session-mods#every-player)). Para uma expansão, `true` enquanto o conteúdo roda. Mods de cliente raramente precisam disso. Com o id de um [escopo secundário](#hybrid-scopes), a resposta para aquele escopo: sempre `true` para um escopo secundário de cliente, e `false` para um desligado.

### obsidian.session.guard(t, free)

Uma visão da tabela `t` para entregar a outros mods. As funções citadas na lista `free` funcionam normalmente. Todas as outras respondem `false, "needs_session"` quando o mod que chama roda como mod de cliente, e `false, "not_cleared"` quando a sessão não está liberada ([Escrever uma biblioteca](wiki:session-mods#libraries)). Uma expansão que as chama recebe `false, "not_started"` fora do conteúdo, e uso completo das bibliotecas de que precisa enquanto o conteúdo roda, seja quem for que esteja no lobby.

### obsidian.session.members()

Uma lista dos outros jogadores no lobby. Cada entrada tem `peer`, `obsidian` (se aquele jogador roda o Obsidian), `modset` e `consent` (verdadeiro, a não ser que aquele jogador bloqueie mods de sessão de outros jogadores num lobby público).

Para rodar só quando todos os jogadores têm o mod, defina [`every_player`](wiki:session-mods#every-player) no manifesto em vez de comparar isso por conta própria.

### obsidian.session.forfeit_progress(), release_progress(key) and hold_progress()

Estas não fazem nada. O save pertence ao jogador, então o Obsidian não segura nenhum progresso de uma missão, façam os mods o que fizerem. Elas continuam existindo para que os mods que as chamam sigam funcionando. Só os relatórios da Guerra Galáctica e dos placares continuam bloqueados, enquanto há conteúdo de sessão aplicado e para missões em que o conteúdo de uma expansão rodou ([A Guerra Galáctica](wiki:galactic-war)). Para jogar sem uma missão contar, o jogador liga **Congelar a progressão da conta** nas [Proteções](wiki:guardrails).

### obsidian.guardrails()

O que o jogador escolheu nas **Proteções** para este início do jogo, como `{ consent, block_achievements, freeze_progress }`, em que `consent` é falso enquanto o jogador bloqueia mods de sessão de outros jogadores num lobby público. Um mod pode lê-los, por exemplo para avisar o jogador de que a conta dele está congelada. Só o jogador pode mudá-los, e o carregador os impõe a todos os mods ([Proteções](wiki:guardrails)).

### obsidian.war()

A guerra personalizada do [Commander](wiki:commander) que este início do jogo joga, como `{ id, model }`, em que `model` é `"vanilla"` ou `"enhanced"`; `nil` na guerra oficial. O mod complementar de uma guerra Enhanced o lê para saber quando agir. Confira `obsidian.capabilities.war` antes: um carregador mais antigo não tem essa chamada.

### obsidian.on_session(fn, opts)

Chama `fn(on)` sempre que o carregador liga ou desliga este mod, para que o mod possa desfazer o que os outros jogadores veriam. Um mod marcado `every_player` também fica sabendo quando um jogador que não o roda chega ou sai. Uma expansão recebe `true` quando o conteúdo começa e `false` quando termina, em todos os jogos do lobby, e o mesmo vale para uma biblioteca de que só expansões precisam. Com `opts.secondary_scope`, `fn` só recebe a troca daquele [escopo secundário](#hybrid-scopes): um de cliente nunca desliga.

### Escopos Híbridos {#hybrid-scopes}

Um mod com [`secondary_scopes`](wiki:manifest#hybrid-scopes) marca cada registro com o escopo secundário a que pertence, como último argumento de `obsidian.hook`, `obsidian.on_frame`, `obsidian.keybind` e `obsidian.on_session`:

```lua
local view = { secondary_scope = "view" }
obsidian.hook("CameraManager", "update", on_camera, view)
obsidian.on_frame(tick_view, view)
obsidian.keybind("toggle_view", toggle, { secondary_scope = "view", always = true })
obsidian.on_session(function(on) if not on then undo_range() end end, { secondary_scope = "weapon_range" })
```

O código de um escopo secundário de cliente roda em qualquer lobby, e o de um de sessão só num lobby liberado. Numa expansão, o código de um escopo secundário de sessão roda enquanto o conteúdo roda, e fora dele num lobby liberado. Num mod de sessão, código sem marcação segue o escopo secundário mais restrito que está ligado; numa expansão, fica com a expansão. Um escopo secundário desligado não registra nada. Código que roda uma vez, como o script de entrada e `on_require`, não é marcado. Um único estado Lua serve todos os escopos secundários, então código num de cliente ainda pode chamar código de sessão diretamente: só os registros são controlados.

`obsidian.scope()` responde `"hybrid"` enquanto um mod de sessão tem um escopo secundário de cliente e um de sessão ligados. As chamadas que uma biblioteca protege tratam um mod híbrido como mod de sessão, de qualquer escopo secundário. Um carregador mais antigo ignora `opts.secondary_scope` e controla tudo com o escopo principal, então confira `obsidian.capabilities.secondary_scopes` antes de dizer ao jogador que o código pessoal continua rodando. Carregadores de antes de os Escopos Híbridos terem esse nome leem `opts.part`, e este carregador ainda lê.

## Expansões {#expansions}

Para um mod com `"scope": "expansion"` ([Escrever uma expansão](wiki:session-mods#expansions)). Confira `obsidian.capabilities.expansion` antes: um carregador mais antigo não tem estas funções.

### obsidian.expansion.start()

Inicia o conteúdo da expansão que chama, no jogo do anfitrião. Todos os outros jogadores do lobby precisam rodar a mesma expansão, na mesma versão e com os mesmos valores de opção compartilhados. Responde `true`, ou `false` e um motivo: `"players_missing"` com uma lista dos nomes dos jogadores que não a rodam como terceiro valor, `"not_host"`, `"busy"` enquanto o conteúdo de outra expansão roda, ou `"not_expansion"`. Um início recusado também mostra ao anfitrião um aviso com os nomes.

### obsidian.expansion.stop()

Encerra o conteúdo, no jogo do anfitrião. Responde `true`, ou `false` e `"not_running"` ou `"not_host"`. Sair do lobby também encerra o conteúdo.

### obsidian.expansion.running(id)

`true` enquanto o conteúdo da expansão `id` roda neste jogo. Sem `id`, o mod que chama.

## Texto e logs {#text-and-logs}

### obsidian.text(key, ...)

O texto `key` da pasta `locales` do mod, no idioma do jogo, com o inglês como reserva. Argumentos extras preenchem marcadores `%s` e `%d`. Um catálogo tem o nome do idioma: `en.json`, `fr.json`, `es.json`, `pt-BR.json`, `de.json`, `it.json`, `ru.json`, `pl.json`, `ko.json`, `ja.json`, `zh-CN.json`, `zh-TW.json`, `sv.json`, `da.json`, `nl.json`, `fi.json` ou `no.json`.

### obsidian.language()

O idioma em que o jogo roda, pelos mesmos nomes, como `"ru"`. `"en"` para um idioma que o Obsidian não conhece. Confira `obsidian.capabilities.language` num carregador mais antigo.

### obsidian.game_text(key, words)

Faz o jogo mostrar `words` para uma das chaves de texto do jogo, ou para uma chave que o mod inventa, como o título de uma nova patente. `words` é uma string, ou uma tabela por idioma como `{ en = "Warden", ru = "Страж" }`, que recorre ao inglês. `nil` devolve a chave ao jogo. Responde `false` para qualquer outra coisa. Um texto é só o que este jogador vê, então continua funcionando enquanto o lobby não está liberado. Confira `obsidian.capabilities.game_text` num carregador mais antigo.

### obsidian.log(...)

Escreve uma linha no log deste mod, em `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`.

### obsidian.profiler.start(), stop(), running() and read()

Mede quanto de cada quadro os scripts de cada mod tomam. `start()` liga a medição para o mod que chama até ele chamar `stop()`; a medição fica ligada enquanto algum mod a pediu, e custa um pouco de cada chamada a um mod. `read()` responde o último resultado, gerado cerca de duas vezes por segundo, ou `nil` antes do primeiro: `{ frame_ms, frames, total_ms, clock, mods = { { id, ms, peak_ms, calls } } }`, o mod mais custoso primeiro. `ms` é o tempo médio por quadro, `peak_ms` o máximo num quadro, e `calls` o número médio de chamadas por quadro. O tempo que um gancho passa na função original do jogo conta para o jogo, não para o mod. `obsidian.loader` representa os ganchos do próprio carregador. Com a opção Script Profiler do Obsidian API ligada no Obsidian, a página Script Profiler da janela F6 mostra esses números sobre o jogo. Confira `obsidian.capabilities.profiler` num carregador mais antigo.

## Outros mods e mais arquivos {#other-mods}

### obsidian.module(name)

Carrega outro arquivo Lua deste mod e retorna o que ele retorna. O arquivo precisa estar listado em `modules` no `obsidian.json`, e `name` é o caminho do arquivo sem `.lua`, como `"lib/parts"`.

### obsidian.import(id)

O que o script `entry` de outro mod retornou, ou `nil` quando esse mod não está rodando. Liste o mod em `requires`, para que ele comece antes ([Obsidian API](wiki:obsidian-api)).

### obsidian.scope()

Como o mod que chama roda nesta aplicação: `"client"`, `"session"`, `"hybrid"` (um mod de sessão com um escopo secundário de cliente e um de sessão ligados, [Escopos Híbridos](#hybrid-scopes)) ou `"expansion"`. Uma biblioteca recebe `"client"` ou `"session"`, e `"session"` quando uma expansão precisa dela, para que a biblioteca prepare o que a expansão pode usar ([Escrever uma biblioteca](wiki:session-mods#libraries)).

### obsidian.caller()

O id do mod cujo código está rodando, para que um mod compartilhado possa registrar quem pediu algo.

### obsidian.info(id)

O que um mod em execução é: `{ id, name, version, scope }`. `name` vem do `obsidian.json` do mod, no idioma do jogo, senão em inglês, e `scope` é o que `obsidian.scope()` responde naquele mod. Sem `id`, o mod que chama. `nil` para um mod que não está rodando.

## Menu dentro do jogo {#menu}

A janela dentro do jogo pertence ao Obsidian API, então muda com as releases do API, e mods novos usam `api.menu` ([Obsidian API](wiki:obsidian-api)). **F6** a abre, uma tecla que pertence ao Obsidian API e que os jogadores mudam na aba Teclas do Obsidian API, e as páginas de cada mod ficam numa pasta com o nome do mod. `obsidian.menu` continua existindo para mods escritos antes, e repassa toda chamada à janela do Obsidian API, como o mod que a fez. Liste `obsidian.api` em `requires`: sem o Obsidian API rodando não há janela, e `open()` responde `false`.

### obsidian.menu.add(title, build, opts)

Adiciona uma página à lista à esquerda do menu dentro do jogo, na pasta do mod. `title` é um texto, ou uma função que o retorna. `build(page)` roda cada vez que a página abre. `opts.icon` é um de `obsidian.ui.glyphs()`:

```lua
obsidian.menu.add("Example Mod", function(page)
  page:header("Tuning")
  page:toggle("Enabled", { get = function() return on end, set = function(v) on = v end })
  page:number("Power", { min = 1, max = 10, step = 1, get = get_power, set = set_power })
  page:choice("Mode", { choices = { { id = "a", label = "First" } }, get = get_mode, set = set_mode })
  page:action("Do it", function() return "Done" end)
  page:input("Name", function(text) end)
  page:page("More", function(sub) sub:label("Deeper") end)
end, { icon = "player" })
```

Qualquer linha aceita `help`, um texto mostrado enquanto ela está selecionada, e `enabled`, uma função que retorna `false` e um motivo. Uma string retornada por uma ação aparece como aviso rápido. Uma ação pode chamar `page:refresh()` para montar a página de novo. Uma linha que falha mostra o erro e não conta contra o mod.

Uma linha `toggle`, `number` ou `choice` aceita `default`, o valor inicial. Ela então ganha um botão de restaurar no fim da linha, aceso quando o valor mudou, e R faz o mesmo pelo teclado. Quando escrever um valor não é o caminho de volta, passe `reset`, uma função que o desfaz, e `changed`, uma que diz se há algo para desfazer.

As páginas de um mod de sessão não são listadas, e fecham, enquanto o lobby não está liberado. As páginas de uma expansão continuam listadas, para que o conteúdo possa ser iniciado por uma delas.

### obsidian.menu.open(), close(), toggle(), is_open()

A tecla do Obsidian API abre a janela quando há uma página listada, então o mod não precisa de tecla para isso. Estas funções são para um mod que abre a janela num momento que ele escolhe.

### obsidian.menu.overlay(fn)

Chama `fn(draw, width, height)` a cada quadro, para pôr algo na tela fora do menu. `draw.text(text, x, y, color)` e `draw.rect(x, y, w, h, color, alpha)` trabalham em pixels a partir do canto inferior esquerdo. Cores são `{ r, g, b }`.

### obsidian.toast(text, seconds)

Mostra uma mensagem curta no canto superior esquerdo da tela, ou ao lado do menu dentro do jogo enquanto ele está aberto. Com o Obsidian API rodando, a janela do API a desenha.

### obsidian.notify(text, seconds) {#obsidian-notify-text-seconds}

Mostra `text` na caixa de dicas do jogo, com o ícone e o som de dica, mesmo com as dicas do jogo desligadas. `seconds` é 7 se omitido. Onde o jogo não tem caixa de dicas, como no menu principal, aparece como aviso rápido.

A caixa mostra uma mensagem por vez, então use-a para o que é raro e importa, e `obsidian.toast` para retornos rápidos, como uma tecla pressionada. Um texto que já está esperando não é adicionado de novo, e um mod pode ter 3 esperando: além disso, `notify` retorna `false` e `"queue_full"`. Mensagens do Obsidian vêm primeiro.

## Substituir telas do jogo {#ui}

`obsidian.ui` é para um mod que redesenha os menus e o HUD do jogo. Qualquer mod pode usá-lo, de qualquer escopo, sem nada em `requires`, então um reskin continua sendo mod de cliente e mantém a Guerra Galáctica e o matchmaking. Nada aqui roda até um mod pedir.

A ideia é manter a lógica e a entrada do jogo, parar o desenho do jogo, ler os dados do jogo e desenhar por cima. As funções de atualização e desenho do jogo sempre rodam, porque teclas, mouse e código de rede vivem dentro delas.

```lua
local ui = obsidian.ui
ui.frame(1920, 1080)

ui.on("loadout", {
  mute = true,
  draw = function(kit, data, dt)
    kit:panel{ 120, 140, 1680, 800, cut = 24, corners = "tr bl", fill = "#38383ee0", line = "#ffc433" }
    kit:text("LOADOUT", 960, 880, { font = "heading", size = 48, tracking = 4, align = "center" })

    for i, player in ipairs(data.players()) do
      local panel = data.loadout(player.slot)
      local x = 140 + (i - 1) * 420
      kit:band(x, 780, 400, 40, player.color)
      kit:text(player.name, x + 12, 792, { font = "body_large" })
      if panel then kit:text(panel.ready and "READY" or "PICKING", x + 12, 740) end
    end

    kit:text(ui.prompt("menu_loadout_select"), 140, 150)
  end,
})
```

Se o código de uma tela falha, aquela tela para pelo resto da sessão, o jogo volta a desenhar a original, e o erro vai para `loader.log`.

### obsidian.ui.current()

Onde o jogador está: `{ state, screen, sub }`. `screen` é o id que o resto desta seção usa: um menu pelo nome (`"loadout"`, `"armory_profile"`, `"main"`), `"title"` e `"ship"` sem menu aberto, `"hud"` numa missão, ou o nome do estado, como `"mission_end"`. `sub` é a etapa dentro dele, quando há uma.

### obsidian.ui.on(id, handlers), off(id)

Roda código enquanto uma tela está aberta. `id` é um valor de `screen`, ou `"*"` para todas.

| Handler | Quando |
|---|---|
| `enter(screen)` | A tela apareceu. Recebe o que `current()` dá |
| `draw(kit, data, dt)` | A cada quadro. `kit` é uma tela de desenho própria e `data` é `obsidian.ui.data` |
| `exit()` | A tela saiu |
| `mute` | Não é função: `true` para esta tela, ou uma lista de ids, mantida só enquanto ela está aberta |

Os handlers de um mod de sessão param, e os silenciamentos daquele mod são desfeitos, enquanto o lobby não está liberado. Os de uma expansão continuam rodando fora do conteúdo. `off(id)` remove os handlers do mod que chama.

### obsidian.ui.mute(id), unmute(id)

Faz uma parte do jogo parar de desenhar, até `unmute`. A lógica daquela parte continua rodando.

| Id | O que some |
|---|---|
| O nome de um menu | Aquele menu: `"loadout"`, `"armory"`, `"options"`, `"mission_briefing"` e assim por diante |
| `"hud"` | Todas as partes abaixo |
| `"hud.panels"` | Os painéis dos jogadores |
| `"hud.map"` | O mapa |
| `"hud.stratagems"` | A lista de estratagemas e as recargas |
| `"hud.names"` | Nomes sobre os jogadores |
| `"hud.world"` | Etiquetas em coisas do mundo |
| `"hud.prompts"` | Avisos em coisas para usar |
| `"hud.dialogue"` | Diálogos |
| `"chat"`, `"hints"`, `"samples"`, `"news"`, `"save_icon"` | Cada um desses |
| `"mission_end"` | As etapas das telas finais |

As partes se aninham: silenciar `"hud.panels"` mantém o mapa, até ele ser silenciado também. Os consoles 3D da nave, o mapa da galáxia e o filme de introdução não são desenhados assim, então ficam. A tela de campanha desenha alguns quadros diretamente, e esses também ficam.

### obsidian.ui.scope(id, target, method)

Adiciona uma parte personalizada. `mute(id)` então para o que aquela função do jogo desenha: `target` é uma tabela ou o nome de uma global. Não muda mais nada.

### obsidian.ui.data

Cópias do que o jogo sabe, então escrever numa delas não muda nada. Cada uma responde `nil` onde não se aplica. `player` é uma posição de 1 a 4, e `nil` quer dizer o jogador deste teclado.

| Função | Dá |
|---|---|
| `players()` | `name`, `title`, `rank_icon`, `xp`, `color`, `index`, `slot`, `is_local`, `pad`, para cada jogador |
| `loadout(player)` | Na tela de Equipamento: `items`, `available`, `selection`, `submenu`, `slot`, `ready`, `random`, `loadout`, `presets`, `new_unlocks`, `upgrades` |
| `hud(player)` | `state`, `health`, `shield`, `inventory`, `character`, `stratagems`, `cooldowns`, `respawn`. As tabelas usam os nomes de campo do jogo |
| `map(player)` | `zoom`, `offset`, `size`, `pings`, `markers` |
| `mission()` | `objectives`, `enemy`, `difficulty`, `environment`, `type`, `seed`, `game_over`, `exit` |
| `campaign()` | `status`, `statistics`, `season`. É grande, então peça uma vez |
| `mission_end()` | `sub`, e `values` com todos os campos simples do estado |

### obsidian.ui.canvas(id, opts)

Um kit de desenho, para código que não é um handler `draw`. `opts.layer` vai de 0 a 875, e `opts.over_fades` o mantém com força total enquanto o jogo escurece. Por padrão um kit escurece junto com o jogo.

As coordenadas são pixels a partir do canto inferior esquerdo, ou o espaço de design do mod depois de `obsidian.ui.frame`. Cores são `{ r, g, b }`, `{ r, g, b, a }`, `"#rrggbb"` ou `"#rrggbbaa"`.

| Método | Desenha |
|---|---|
| `kit:rect(x, y, w, h, color)` | Um retângulo |
| `kit:poly(points, color)` | Uma forma convexa, a partir de `{ x1, y1, x2, y2, ... }` |
| `kit:outline(points, color, width)` | A borda de uma forma, por dentro |
| `kit:panel{ x, y, w, h, cut, corners, fill, line, line_width }` | Um painel com cantos cortados a 45 graus. `corners` é qualquer um de `"tl tr br bl"`, por padrão `"tr bl"` |
| `kit:hex(x, y, r, color, icon)` | Um hexágono em volta de um ponto, com um ícone dentro se passado |
| `kit:glyph(name, x, y, size, color)` | Um dos ícones desenhados com formas, a partir do canto inferior esquerdo. `obsidian.ui.glyphs()` lista os nomes |
| `kit:text(text, x, y, opts)` | Texto. `y` é a linha de base e uma quebra de linha começa uma nova linha. `opts`: `font`, `size`, `tracking`, `align`, `color`, `leading` |
| `kit:measure(text, opts)` | Largura e altura de um texto, espaçamento incluído. Não desenha nada |
| `kit:point(x, y)` | Um ponto em pixels, como a posição do mouse, nas coordenadas deste kit |
| `kit:icon(name, x, y, w, h, color, uv)` | Uma das imagens do jogo. `uv` é `{ u0, v0, u1, v1 }`. `obsidian.ui.icons()` lista os nomes |
| `kit:art(name, x, y, w, h, color, uv)` | Uma imagem de um material que o mod traz, adicionada com `obsidian.ui.art`. `uv` é `{ u0, v0, u1, v1 }`, e `u0` acima de `u1` espelha a imagem |
| `kit:band(x, y, w, h, color)` | A faixa de jogador do jogo, com o degradê da faixa, tingida |
| `kit:bar(x, y, w, h, value, color, back)` | Uma barra preenchida de 0 a 1, sobre `back` |
| `kit:clip(x, y, w, h, fn)` | Roda `fn(kit)` com o desenho mantido dentro do retângulo |
| `kit:layer(n)`, `kit:opacity(a)` | A camada e a força do que vem depois |

Os glifos não precisam de arte, então continuam funcionando onde `kit:icon` não funciona. O menu dentro do jogo é feito dessas mesmas chamadas, então qualquer coisa que ele desenha, um mod também pode desenhar.

Fontes: `"body"`, `"body_large"`, `"heading"`, `"heading_large"`, `"ui"`, `"ui_heading"`, `"chat"`, `"debug"`, `"debug_medium"`. `"ui"` e `"ui_heading"` são Barlow e Saira, as fontes do menu dentro do jogo, e têm letras latinas com acentos. `"heading"` é Insignia, e `"chat"` é a que tem chinês, japonês e coreano, para a qual um texto recorre quando a fonte escolhida não tem uma letra. `"debug"` não precisa de nada carregado, e todas as outras fontes recorrem a ela até o jogo carregá-las.

Um recorte corta formas e ícones exatamente, e texto em letras inteiras. A arte de menu da tela de Equipamento não está disponível: o material é descarregado entre estados, e uma gui que lista um que não está carregado pode parar o jogo. Arte personalizada precisa da mesma verificação, então uma tela de desenho aceita só as imagens do jogo por enquanto.

### obsidian.ui.font(id, opts)

Adiciona uma fonte personalizada, para `kit:text`. `tools/bake_font.py` no repositório do Obsidian transforma um arquivo TrueType no que o jogo lê:

```
pip install pillow uharfbuzz
python tools/bake_font.py --out res --base mymod/fonts title=MyFont.ttf
```

Depois liste cada arquivo nos `resources` do manifesto, no bundle de boot:

```json
"resources": [
  { "type": "font", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.font" },
  { "type": "texture", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.texture" },
  { "type": "material", "name": "mymod/fonts/fonts", "bundle": "9cf71054ea26dddf", "data": "res/fonts.material" }
]
```

Adicione-a quando o mod começa, e use-a pelo id:

```lua
obsidian.ui.font("title", { path = "mymod/fonts/title", size = 32 })
kit:text("HELLDIVERS", 100, 200, { font = "title" })
```

`size` é o tamanho que um texto recebe quando não pede um. A resposta é `false` para uma fonte que a aplicação não tem, e um texto que a cita recebe a fonte de depuração. Ela precisa estar no bundle de boot, porque uma gui que lista um material não carregado pode parar o jogo. O texto fica nítido até o tamanho em que foi gerado, 48 por padrão, e uma letra que falta na fonte vem de `"chat"`. Confira a licença da fonte: uma fonte gerada assim é uma versão modificada, então uma fonte com nome reservado não pode manter esse nome.

### obsidian.ui.art(name, opts)

Adiciona um submaterial de um material que o mod traz no bundle de boot, para `kit:art`. `opts.path` é o nome de recurso do material. A resposta é `false` para um material que a aplicação não tem, ou quando um novo material é pedido depois que o jogo começou a desenhar, então peça enquanto o mod começa.

### obsidian.ui.list_with(path, with)

Faz toda gui que o jogo cria e que lista o material `with` listar também `path`, para que o jogo possa desenhar um submaterial de `path` onde desenha os de `with`. `with` é `"hud/hud"` se omitido, o material que tem os ícones de patente e de estratagema. `path` é um material que o mod traz no bundle de boot, e a resposta é `false` para um que a aplicação não tem. Chame enquanto o mod começa, antes de o jogo criar as guis. Ponha muitos ícones num só material e liste esse.

### obsidian.ui.frame(w, h), safe()

`frame(1920, 1080)` transforma as coordenadas deste mod num espaço de design, escalado e centralizado como o jogo faz, a partir da janela e da escala de interface do jogador. `frame()` volta para pixels. Chame uma vez, mesmo enquanto o mod começa. O jogo deixa o espaço no fundo da janela quando a escala da interface é menor que 1; `frame(1920, 1080, { centered = true })` o mantém no meio.

`safe()` dá `{ x, y, w, h, scale }`, a área em que o jogo mantém os menus do jogo, nas coordenadas do mod.

### obsidian.ui.remap_hits(id, fn)

O jogo testa o mouse contra onde desenha. Quando o layout do mod move algo, `fn(x, y, w, h)` recebe cada retângulo em pixels e responde onde ele está agora, `nil` para deixá-lo, ou `false` para lugar nenhum. `remap_hits(id)` para isso.

### obsidian.ui.prompt(name, pad)

O botão no dispositivo de um jogador, como texto para desenhar. `name` é uma ação como `"select"`, `"back"`, `"fire"`, `"reload"` ou `"map"`, um token como `"#mod_map"`, ou uma das chaves de texto do jogo, que volta com os botões preenchidos. `pad` é o número do controle vindo de `data.players()`, e `nil` quer dizer o jogador principal.

## Valores salvos {#storage}

### obsidian.storage.get(key, default), set(key, value), all()

Strings, números e booleanos que este mod guarda entre inícios do jogo. `set` com `nil` remove uma chave. O arquivo é escrito na hora, então não chame isso a cada quadro.

### obsidian.save_data.get(key, default), set(key, value), sandbox() {#save-data}

Strings, números e booleanos que este mod guarda com o save atual, num arquivo ao lado do save. Cada [Sandbox](wiki:saves) tem um arquivo separado, e o arquivo Oficial acompanha o save quando o Obsidian troca saves Oficiais. Use para o que pertence a um personagem, como um desbloqueio manual, e `obsidian.storage` para o que pertence à instalação. `set` com `nil` remove uma chave, e o arquivo é escrito na hora. `sandbox()` é o id do Sandbox deste início do jogo, ou `nil` num save Oficial. Confira `obsidian.capabilities.save_data` antes: um carregador mais antigo não tem isso.

Valores que um jogador deve poder editar no editor de saves vão na [chave `save_editor`](wiki:manifest#save-editor) do manifesto.

## Versão e recursos {#version-and-features}

`obsidian.version` é a versão da API, `1` hoje. Um mod que precisa de uma API mais nova define `loader_api` no `obsidian.json`, e carregadores mais antigos se recusam a iniciá-lo.

`obsidian.capabilities` é uma tabela somente leitura do que o carregador em execução suporta, uma entrada por recurso: `hook`, `wrap`, `on_require`, `on_frame`, `on_session`, `after_startup`, `settings`, `keybind`, `input_modes`, `pad_binds`, `session`, `guardrails`, `war`, `scope`, `text`, `log`, `profiler`, `module`, `import`, `caller`, `info`, `menu`, `toast`, `notify`, `storage`, `save_data`, `ui`, `expansion`, `language`, `game_text`, `secondary_scopes` (também `parts`, o nome antigo). Teste um recurso pelo nome antes de usar um que veio depois:

```lua
if obsidian.capabilities.on_frame then
  obsidian.on_frame(update)
end
```

## Quando um mod falha {#when-a-mod-fails}

- Um erro na inicialização desliga o mod por esta sessão e escreve o erro em `loader.log`.
- Erros em ganchos e callbacks são registrados no máximo três vezes por lugar, então uma função de quadro quebrada não inunda o log.
- Um mod que falha repetidamente é desligado pelo resto da sessão.
- Se o jogo trava enquanto um mod começa, o app cita esse mod no próximo início.
