---
title: API do loader
group: Criar mods
summary: Cada função da tabela obsidian, versão 1.
---

O Obsidian Loader inicia o script `entry` de cada mod ligado, na ordem de carregamento, uma vez, quando o jogo abre. Todo script enxerga uma tabela global chamada `obsidian`. Tudo o que um mod registra por ela roda protegido: um erro vai para o log e nunca impede o jogo de abrir.

## Hooks {#hooks}

### obsidian.hook(target, method, fn) {#obsidian-hook-target-method-fn}

Envolve uma função do jogo. `target` é uma tabela, ou o nome de uma tabela global como texto. `fn` recebe primeiro a função original, depois os argumentos da chamada:

```lua
obsidian.hook("PlayerUnitMovement", "update", function(original, self, dt)
  -- antes do código do jogo
  local result = original(self, dt)
  -- depois do código do jogo
  return result
end)
```

Quando a global ainda não existe, o hook espera e entra assim que o jogo a define. Se `fn` falhar, a original roda no lugar.

Num mod de sessão, o hook só chama a original enquanto o lobby não estiver liberado ([Escrever mods de sessão](wiki:session-mods)). Numa expansão, só chama a original até o conteúdo começar ([Escrever uma expansão](wiki:session-mods#expansions)).

### obsidian.wrap(target, method, fn) {#obsidian-wrap-target-method-fn}

Igual a `obsidian.hook`, com a mesma segurança se `fn` falhar, só que continua rodando enquanto o lobby não está liberado e fora do conteúdo de uma expansão. Serve para o caminho de entrada de uma expansão, e para um mod cujos arquivos substituídos também não podem ser desligados, em que um hook desligado deixaria um jogo pela metade. Todo o resto deve usar `obsidian.hook`. Num loader mais antigo, confira `obsidian.capabilities.wrap`.

### obsidian.on_require(name, fn) {#obsidian-on-require-name-fn}

Chama `fn(module)` quando o jogo carrega o script `name`, como `"lua/settings/perk_settings"`. Use para mudar uma tabela de configuração logo depois que o jogo a monta.

### obsidian.on_frame(fn) {#obsidian-on-frame-fn}

Chama `fn` uma vez por quadro. Quando `fn` devolve `true`, as chamadas param. As funções por quadro de um mod de sessão só rodam num lobby liberado, e as de uma expansão só enquanto o conteúdo roda.

### obsidian.after_startup(fn) {#obsidian-after-startup-fn}

Chama `fn` uma vez, depois que todos os mods iniciaram, para que mods possam trabalhar juntos sem depender da ordem de carregamento.

## Configurações e teclas {#settings-and-keys}

### obsidian.settings.get(id) {#obsidian-settings-get-id}

O valor da opção `id` deste mod: o que o jogador escolheu no app, ou o padrão ([Opções e teclas](wiki:options-and-keys)).

### obsidian.keybind(id, fn, opts) {#obsidian-keybind-id-fn-opts}

Chama `fn` toda vez que a tecla ligada a `id` desce. A tecla vem do app, ou do padrão no `obsidian.json`. As teclas de um mod de sessão esperam um lobby liberado, e as de uma expansão esperam o conteúdo, como os hooks do mod. `opts.always` deixa a tecla sempre ativa, como `obsidian.wrap` faz com um hook, para uma tecla que só age na tela deste jogador.

## Multijogador {#multiplayer}

### obsidian.session.cleared() {#obsidian-session-cleared}

`true` enquanto o mod que pergunta pode rodar: a sessão está liberada e, para um mod marcado com `every_player`, todos os outros jogadores também o rodam ([Mods que todos os jogadores precisam ter](wiki:session-mods#every-player)). Para uma expansão, `true` enquanto o conteúdo roda. Mods de cliente quase nunca precisam.

### obsidian.session.guard(t, free) {#obsidian-session-guard}

Uma visão da tabela `t` para entregar a outros mods. As funções da lista `free` funcionam normalmente. Qualquer outra responde `false, "needs_session"` quando o mod roda como mod de cliente, e `false, "not_cleared"` quando a sessão não está liberada ([Escrever uma biblioteca](wiki:session-mods#libraries)). Uma expansão que as chama recebe `false, "not_started"` fora do conteúdo e, enquanto o conteúdo roda, uso completo das bibliotecas que exige, seja quem for que esteja no lobby.

### obsidian.session.members() {#obsidian-session-members}

Uma lista dos outros jogadores no lobby. Cada entrada tem `peer`, `obsidian` (se o jogador usa o Obsidian), `modset` e `consent` (verdadeiro a menos que o jogador bloqueie os mods de sessão dos outros num lobby público).

Para rodar só quando todos os jogadores têm o seu mod, defina [`every_player`](wiki:session-mods#every-player) no manifesto em vez de comparar estes dados você mesmo.

### obsidian.session.forfeit_progress(), release_progress(key) e hold_progress() {#obsidian-session-forfeit-progress-release-progress-key-and-hold-progress}

Estas não fazem nada. O save de um jogador é dele, então o Obsidian não retém nada do progresso de uma missão, seja o que for que os mods façam. Elas continuam existindo para que os mods que as chamam sigam funcionando. Só os relatórios da Guerra Galáctica e dos placares continuam bloqueados, enquanto houver conteúdo de sessão aplicado e nas missões em que o conteúdo de uma expansão rodou ([A Guerra Galáctica](wiki:galactic-war)). Para jogar sem que uma missão conte, o jogador liga **Congelar a progressão da minha conta** em [Proteções](wiki:guardrails).

### obsidian.guardrails() {#obsidian-guardrails}

O que o jogador escolheu em **Proteções** para esta abertura do jogo, como `{ consent, block_achievements, freeze_progress }`, em que `consent` é falso enquanto o jogador bloqueia os mods de sessão dos outros num lobby público. Um mod pode lê-las, por exemplo para avisar que a conta está congelada. Só o jogador pode mudá-las, e o loader as aplica por baixo de todos os mods ([Proteções](wiki:guardrails)).

### obsidian.on_session(fn) {#obsidian-on-session-fn}

Chama `fn(on)` sempre que o loader liga ou desliga este mod, para que o mod possa desfazer o que outros jogadores veriam. Um mod marcado com `every_player` também fica sabendo quando chega ou sai um jogador que não o roda. Uma expansão recebe `true` quando o conteúdo começa e `false` quando ele termina, em todos os jogos do lobby, e o mesmo vale para uma biblioteca que só expansões exigem.

## Expansões {#expansions}

Para um mod com `"scope": "expansion"` ([Escrever uma expansão](wiki:session-mods#expansions)). Confira `obsidian.capabilities.expansion` antes: um loader mais antigo não tem estas funções.

### obsidian.expansion.start() {#obsidian-expansion-start}

Inicia o conteúdo da expansão que chama, no jogo do host. Todos os outros jogadores do lobby precisam rodar a mesma expansão, na mesma versão e com os mesmos valores de opção compartilhados. Responde `true`, ou `false` e um motivo: `"players_missing"`, com a lista dos nomes dos jogadores que não a rodam como terceiro valor, `"not_host"`, `"busy"` enquanto roda o conteúdo de outra expansão, ou `"not_expansion"`. Um início recusado também mostra ao host um aviso com os nomes.

### obsidian.expansion.stop() {#obsidian-expansion-stop}

Termina o conteúdo, no jogo do host. Responde `true`, ou `false` e `"not_running"` ou `"not_host"`. Sair do lobby também termina o conteúdo.

### obsidian.expansion.running(id) {#obsidian-expansion-running-id}

`true` enquanto o conteúdo da expansão `id` roda neste jogo. Sem `id`, vale para o mod que chama.

## Texto e logs {#text-and-logs}

### obsidian.text(key, ...) {#obsidian-text-key}

O texto `key` da pasta `locales` do mod, no idioma do jogo, com inglês como reserva. Os argumentos extras preenchem os marcadores `%s` e `%d`.

### obsidian.log(...) {#obsidian-log}

Escreve uma linha no log deste mod, em `%APPDATA%\Arrowhead\Helldivers\ObsidianModManager\logs`.

## Outros mods e mais arquivos {#other-mods}

### obsidian.module(name) {#obsidian-module-name}

Carrega outro arquivo Lua deste mod e devolve o que ele devolve. O arquivo precisa estar em `modules` no `obsidian.json`, e `name` é o caminho sem `.lua`, como `"lib/parts"`.

### obsidian.import(id) {#obsidian-import-id}

O que o script `entry` de outro mod devolveu, ou `nil` quando esse mod não está rodando. Coloque o mod em `requires`, para ele iniciar primeiro ([API Obsidian](wiki:obsidian-api)).

### obsidian.scope() {#obsidian-scope}

Como o mod roda nesta aplicação: `"client"`, `"session"` ou `"expansion"`. Uma biblioteca recebe `"client"` ou `"session"`, e `"session"` quando uma expansão a exige, para que a biblioteca prepare o que a expansão pode usar ([Escrever uma biblioteca](wiki:session-mods#libraries)).

### obsidian.caller() {#obsidian-caller}

O id do mod cujo código está rodando, para um mod compartilhado registrar quem pediu algo.

### obsidian.info(id) {#obsidian-info-id}

O que é um mod em execução: `{ id, name, version, scope }`. `name` vem do `obsidian.json` do mod, no idioma do jogo, ou em inglês, e `scope` é o que `obsidian.scope()` responde nesse mod. Sem `id`, vale para o mod que chama. `nil` para um mod que não está rodando.

## Menu dentro do jogo {#menu}

A janela dentro do jogo pertence ao Obsidian API, então muda com as versões da API, e mods novos usam `api.menu` ([API Obsidian](wiki:obsidian-api)). **F6** abre a janela, uma tecla do Obsidian API que os jogadores trocam na aba Teclas do Obsidian API, e as páginas de cada mod ficam numa pasta com o nome do mod. `obsidian.menu` continua existindo para mods escritos antes, e passa cada chamada para a janela do Obsidian API, em nome do mod que a fez. O mod precisa ter `obsidian.api` em `requires`: sem o Obsidian API rodando não há janela, e `open()` responde `false`.

### obsidian.menu.add(title, build, opts) {#obsidian-menu-add-title-build-opts}

Adiciona uma página à lista à esquerda do menu dentro do jogo, na pasta do mod. `title` é um texto, ou uma função que o devolve. `build(page)` roda cada vez que a página abre. `opts.icon` é um dos valores de `obsidian.ui.glyphs()`:

```lua
obsidian.menu.add("My mod", function(page)
  page:header("Tuning")
  page:toggle("Enabled", { get = function() return on end, set = function(v) on = v end })
  page:number("Power", { min = 1, max = 10, step = 1, get = get_power, set = set_power })
  page:choice("Mode", { choices = { { id = "a", label = "First" } }, get = get_mode, set = set_mode })
  page:action("Do it", function() return "Done" end)
  page:input("Name", function(text) end)
  page:page("More", function(sub) sub:label("Deeper") end)
end, { icon = "player" })
```

Qualquer linha aceita `help`, um texto mostrado enquanto está selecionada, e `enabled`, uma função que devolve `false` e um motivo. Um texto devolvido por uma ação aparece como aviso. Uma ação pode chamar `page:refresh()` para montar a página de novo. Uma linha que falha mostra o erro e não conta contra o seu mod.

Uma linha `toggle`, `number` ou `choice` aceita `default`, o valor inicial. Ela ganha um botão de redefinir no fim da linha, aceso quando o valor muda, e R faz o mesmo pelo teclado. Quando escrever um valor não é o caminho de volta, passe `reset`, uma função que desfaz, e `changed`, uma que diz se há algo para desfazer.

As páginas de um mod de sessão não aparecem, e fecham, enquanto o lobby não estiver liberado. As páginas de uma expansão continuam na lista, para que o conteúdo possa ser iniciado por uma delas.

### obsidian.menu.open(), close(), toggle(), is_open() {#obsidian-menu-open}

A tecla do Obsidian API abre a janela assim que uma página está na lista, então o mod não precisa de tecla para isso. Estas funções servem para um mod que abre a janela num momento que ele escolhe.

### obsidian.menu.overlay(fn) {#obsidian-menu-overlay-fn}

Chama `fn(draw, width, height)` a cada quadro, para mostrar algo na tela fora do menu. `draw.text(text, x, y, color)` e `draw.rect(x, y, w, h, color, alpha)` usam pixels a partir do canto inferior esquerdo. Cores são `{ r, g, b }`.

### obsidian.toast(text, seconds) {#obsidian-toast-text-seconds}

Mostra uma mensagem curta no canto superior esquerdo da tela, ou ao lado do menu dentro do jogo enquanto ele está aberto. Com o Obsidian API rodando, quem desenha a mensagem é a janela da API.

### obsidian.notify(text, seconds) {#obsidian-notify-text-seconds}

Mostra as suas palavras na caixa de dicas do próprio jogo, com o ícone e o som dela, mesmo com as dicas do jogo desligadas. `seconds` é 7 se você não informar. Onde o jogo não tem caixa de dicas, como no menu principal, aparece como toast.

A caixa mostra uma mensagem por vez, então use para o que é raro e importante, e `obsidian.toast` para respostas rápidas, como ao apertar uma tecla. Um texto que já está esperando não entra de novo, e um mod pode ter 3 esperando: além disso, `notify` retorna `false` e `"queue_full"`. As mensagens do próprio Obsidian vão primeiro.

## Substituir as telas do jogo {#ui}

`obsidian.ui` é para um mod que redesenha os menus e o HUD do jogo. Qualquer mod pode usar, de qualquer escopo, sem nada em `requires`, então um reskin continua sendo um mod client e mantém a Guerra Galáctica e o matchmaking. Nada aqui roda até um mod pedir.

A ideia é manter a lógica e a entrada do jogo, parar o desenho dele, ler os dados e desenhar por cima. As funções de atualização e desenho do jogo sempre rodam, porque teclas, mouse e rede moram dentro delas.

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

Se o código de uma tela falhar, essa tela para pelo resto da sessão, o jogo volta a desenhar a dele, e o erro vai para o `loader.log`.

### obsidian.ui.current() {#obsidian-ui-current}

Onde o jogador está: `{ state, screen, sub }`. `screen` é o id que o resto desta seção usa: um menu pelo nome (`"loadout"`, `"armory_profile"`, `"main"`), `"title"` e `"ship"` sem menu aberto, `"hud"` em uma missão, ou o nome do próprio estado, como `"mission_end"`. `sub` é a etapa dentro dele, quando existe.

### obsidian.ui.on(id, handlers), off(id) {#obsidian-ui-on-id-handlers-off-id}

Roda código enquanto uma tela está aberta. `id` é um valor de `screen`, ou `"*"` para todas.

| Handler | Quando |
|---|---|
| `enter(screen)` | A tela abriu. Recebe o que `current()` dá |
| `draw(kit, data, dt)` | A cada quadro. `kit` é um canvas só dele e `data` é `obsidian.ui.data` |
| `exit()` | A tela saiu |
| `mute` | Não é função: `true` para esta tela, ou uma lista de ids, válidos só enquanto ela está aberta |

Os handlers de um mod de sessão param, e os mutes desse mod saem, enquanto o lobby não estiver liberado. Os de uma expansão continuam rodando fora do conteúdo. `off(id)` tira os handlers do mod que chama.

### obsidian.ui.mute(id), unmute(id) {#obsidian-ui-mute-id-unmute-id}

Faz uma parte do jogo parar de desenhar, até `unmute`. A lógica dela continua rodando.

| Id | O que some |
|---|---|
| O nome de um menu | Esse menu: `"loadout"`, `"armory"`, `"options"`, `"mission_briefing"` e assim por diante |
| `"hud"` | Todas as partes abaixo |
| `"hud.panels"` | Os painéis dos jogadores |
| `"hud.map"` | O mapa |
| `"hud.stratagems"` | A lista de estratagemas e as recargas |
| `"hud.names"` | Os nomes sobre os jogadores |
| `"hud.world"` | Rótulos em coisas do mundo |
| `"hud.prompts"` | Avisos em coisas para usar |
| `"hud.dialogue"` | Diálogos |
| `"chat"`, `"hints"`, `"samples"`, `"news"`, `"save_icon"` | Cada um desses |
| `"mission_end"` | As etapas das telas finais |

As partes ficam umas dentro das outras: silenciar `"hud.panels"` deixa o mapa, até você silenciá-lo também. Os consoles 3D da nave, o mapa da galáxia e o filme de abertura não são desenhados assim, então ficam. A tela de campanha desenha alguns quadros diretamente, e esses ficam também.

### obsidian.ui.scope(id, target, method) {#obsidian-ui-scope-id-target-method}

Adiciona uma parte sua. `mute(id)` passa a parar o que essa função do jogo desenha: `target` é uma tabela ou o nome de uma global. Não muda mais nada.

### obsidian.ui.data {#obsidian-ui-data}

Cópias do que o jogo sabe, então escrever em uma não muda nada. Cada função responde `nil` onde não se aplica. `player` é um lugar de 1 a 4, e `nil` quer dizer o jogador neste teclado.

| Função | Dá |
|---|---|
| `players()` | `name`, `title`, `rank_icon`, `xp`, `color`, `index`, `slot`, `is_local`, `pad`, de cada jogador |
| `loadout(player)` | Na tela de equipamento: `items`, `available`, `selection`, `submenu`, `slot`, `ready`, `random`, `loadout`, `presets`, `new_unlocks`, `upgrades` |
| `hud(player)` | `state`, `health`, `shield`, `inventory`, `character`, `stratagems`, `cooldowns`, `respawn`. As tabelas usam os nomes de campo do próprio jogo |
| `map(player)` | `zoom`, `offset`, `size`, `pings`, `markers` |
| `mission()` | `objectives`, `enemy`, `difficulty`, `environment`, `type`, `seed`, `game_over`, `exit` |
| `campaign()` | `status`, `statistics`, `season`. É grande, então peça uma vez |
| `mission_end()` | `sub`, e `values` com cada campo simples do estado |

### obsidian.ui.canvas(id, opts) {#obsidian-ui-canvas-id-opts}

Um kit de desenho, para código que não é um handler `draw`. `opts.layer` vai de 0 a 875, e `opts.over_fades` mantém a força total enquanto o jogo escurece a tela. Por padrão um kit escurece junto com o jogo.

As coordenadas são pixels a partir do canto inferior esquerdo, ou o seu espaço de design depois de `obsidian.ui.frame`. Cores são `{ r, g, b }`, `{ r, g, b, a }`, `"#rrggbb"` ou `"#rrggbbaa"`.

| Método | Desenha |
|---|---|
| `kit:rect(x, y, w, h, color)` | Um retângulo |
| `kit:poly(points, color)` | Uma forma convexa, a partir de `{ x1, y1, x2, y2, ... }` |
| `kit:outline(points, color, width)` | A borda de uma forma, por dentro |
| `kit:panel{ x, y, w, h, cut, corners, fill, line, line_width }` | Um painel com cantos cortados a 45 graus. `corners` é qualquer um de `"tl tr br bl"`, por padrão `"tr bl"` |
| `kit:hex(x, y, r, color, icon)` | Um hexágono em volta de um ponto, com um ícone dentro se você passar um |
| `kit:glyph(name, x, y, size, color)` | Um dos ícones desenhados com formas, a partir do canto inferior esquerdo. `obsidian.ui.glyphs()` lista os nomes |
| `kit:text(text, x, y, opts)` | Texto. `y` é a linha de base, e uma quebra de linha começa outra linha. `opts`: `font`, `size`, `tracking`, `align`, `color`, `leading` |
| `kit:measure(text, opts)` | Largura e altura de um texto, com o espaçamento. Não desenha |
| `kit:point(x, y)` | Um ponto em pixels, como o do mouse, nas coordenadas deste kit |
| `kit:icon(name, x, y, w, h, color, uv)` | Uma imagem do jogo. `uv` é `{ u0, v0, u1, v1 }`. `obsidian.ui.icons()` lista os nomes |
| `kit:art(name, x, y, w, h, color, uv)` | Uma imagem de um material que o seu mod traz, adicionada com `obsidian.ui.art`. `uv` é `{ u0, v0, u1, v1 }`, e `u0` acima de `u1` espelha a imagem |
| `kit:band(x, y, w, h, color)` | A faixa de jogador do jogo: o degradê dele, tingido |
| `kit:bar(x, y, w, h, value, color, back)` | Uma barra cheia de 0 a 1, sobre `back` |
| `kit:clip(x, y, w, h, fn)` | Roda `fn(kit)` com o desenho preso ao retângulo |
| `kit:layer(n)`, `kit:opacity(a)` | A camada e a força do que vem depois |

Ícones desenhados (glyphs) não precisam de arte, então continuam funcionando onde `kit:icon` não funciona. O menu dentro do jogo é montado com essas mesmas chamadas, então tudo o que ele desenha um mod também pode desenhar.

Fontes: `"body"`, `"body_large"`, `"heading"`, `"heading_large"`, `"ui"`, `"ui_heading"`, `"chat"`, `"debug"`, `"debug_medium"`. `"ui"` e `"ui_heading"` são a Barlow e a Saira, as fontes do menu dentro do jogo, e têm letras latinas com acento. `"heading"` é a Insignia, e `"chat"` é a que tem chinês, japonês e coreano, para onde um texto vai quando falta uma letra na fonte dele. `"debug"` não precisa de nada carregado, e as outras caem nela até o jogo carregá-las.

Um clip corta formas e ícones com exatidão, e texto em letras inteiras. A arte de menu da tela de equipamento não está disponível: o material dela sai da memória entre estados, e uma gui que lista um material não carregado pode parar o jogo. A sua própria arte precisa da mesma verificação, então por enquanto um canvas aceita só as imagens do jogo.

### obsidian.ui.font(id, opts) {#obsidian-ui-font-id-opts}

Adiciona uma fonte sua, para `kit:text`. O `tools/bake_font.py`, no repositório do Obsidian, transforma um arquivo TrueType no que o jogo lê:

```
pip install pillow uharfbuzz
python tools/bake_font.py --out res --base mymod/fonts title=MyFont.ttf
```

Depois liste cada arquivo em `resources` no seu manifesto, no bundle de inicialização:

```json
"resources": [
  { "type": "font", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.font" },
  { "type": "texture", "name": "mymod/fonts/title", "bundle": "9cf71054ea26dddf", "data": "res/title.texture" },
  { "type": "material", "name": "mymod/fonts/fonts", "bundle": "9cf71054ea26dddf", "data": "res/fonts.material" }
]
```

Adicione quando o seu mod iniciar, e use pelo id:

```lua
obsidian.ui.font("title", { path = "mymod/fonts/title", size = 32 })
kit:text("HELLDIVERS", 100, 200, { font = "title" })
```

`size` é o tamanho que um texto recebe quando não pede um. A resposta é `false` para uma fonte que a aplicação não tem, e um texto que a nomeia recebe a fonte debug. Ela precisa estar no bundle de inicialização, porque uma gui que lista um material não carregado pode parar o jogo. O texto fica nítido até o tamanho em que você gerou a fonte, 48 por padrão, e uma letra que a sua fonte não tem vem de `"chat"`. Confira a licença da sua fonte: gerar a fonte cria uma versão modificada, então uma fonte com nome reservado não pode manter esse nome.

### obsidian.ui.art(name, opts) {#obsidian-ui-art-name-opts}

Adiciona um submaterial de um material que o seu mod traz no bundle de boot, para `kit:art`. `opts.path` é o nome do recurso do material. A resposta é `false` para um material que o deploy não tem.

### obsidian.ui.frame(w, h), safe() {#obsidian-ui-frame-w-h-safe}

`frame(1920, 1080)` faz das coordenadas deste mod um espaço de design, escalado e centralizado como o jogo faz, a partir da janela e da escala de interface do jogador. `frame()` volta para pixels. Chame uma vez, mesmo enquanto o mod inicia. O jogo deixa o espaço na parte de baixo da janela quando a escala de interface é menor que 1; `frame(1920, 1080, { centered = true })` o mantém no meio.

`safe()` dá `{ x, y, w, h, scale }`, a área em que o jogo mantém os próprios menus, nas suas coordenadas.

### obsidian.ui.remap_hits(id, fn) {#obsidian-ui-remap-hits-id-fn}

O jogo testa o mouse contra o lugar onde desenha. Quando o seu layout move algo, `fn(x, y, w, h)` recebe cada retângulo em pixels e responde onde ele está agora, `nil` para deixar, ou `false` para lugar nenhum. `remap_hits(id)` para.

### obsidian.ui.prompt(name, pad) {#obsidian-ui-prompt-name-pad}

O botão do dispositivo de um jogador, como texto para desenhar. `name` é uma ação como `"select"`, `"back"`, `"fire"`, `"reload"` ou `"map"`, um token como `"#mod_map"`, ou uma chave de texto do jogo, que volta com os botões preenchidos. `pad` é o número do controle em `data.players()`, e `nil` quer dizer o jogador principal.

## Valores salvos {#storage}

### obsidian.storage.get(key, default), set(key, value), all() {#obsidian-storage}

Textos, números e booleanos que este mod guarda entre aberturas do jogo. `set` com `nil` remove a chave. O arquivo é gravado na hora, então não chame a cada quadro.

## Versão e recursos {#version-and-features}

`obsidian.version` é a versão da API, hoje `1`. Um mod que precisa de uma API mais nova define `loader_api` no `obsidian.json`, e loaders mais antigos se recusam a iniciá-lo.

`obsidian.capabilities` é uma tabela, só de leitura, do que o loader em execução oferece, uma entrada por recurso: `hook`, `on_require`, `on_frame`, `on_session`, `after_startup`, `settings`, `keybind`, `session`, `text`, `log`, `module`, `import`, `caller`, `info`, `menu`, `toast`, `notify`, `storage`, `ui`, `expansion`. Teste um recurso pelo nome antes de usar um que chegou depois:

```lua
if obsidian.capabilities.on_frame then
  obsidian.on_frame(update)
end
```

## Quando um mod falha {#when-a-mod-fails}

- Um erro na inicialização desliga o mod nesta sessão e grava o erro no `loader.log`.
- Erros em hooks e callbacks vão para o log no máximo três vezes por lugar, então uma função por quadro quebrada não inunda o log.
- Um mod que falha sem parar é desligado pelo resto da sessão.
- Se o jogo travar enquanto um mod inicia, o app aponta esse mod na próxima abertura.
