---
title: Escrever mods de sessão
group: Criar mods
summary: Como mudar o jogo compartilhado de modo que as proteções continuem funcionando.
---

Um mod de sessão muda algo que outros jogadores notam: dano, inimigos, surgimentos, missões. O carregador decide quando um mod assim pode rodar ([Jogar online](wiki:multiplayer)), e o liga e desliga conforme jogadores entram e saem. O trabalho do mod é fazer toda mudança pelo carregador, para que o carregador possa desligá-la.

Um mod cujas mudanças acontecem todas dentro de um conteúdo à parte, como um modo de jogo com uma tela de missão própria, pode ser uma [expansão](#expansions), e deixar todas as outras missões em paz.

## Definir o escopo {#set-the-scope}

```json
"scope": "session"
```

Deixar o campo de fora torna o mod não especificado, que roda como mod de cliente. Só use `client` quando nada do que o mod faz chega a outros jogadores.

## Mudar o jogo por ganchos {#change-the-game-through-hooks}

Os ganchos de um mod de sessão chamam o código original do jogo sozinhos enquanto o lobby não está liberado. Um mod feito só de ganchos não precisa de código extra para ser seguro:

```lua
-- Double the damage of every shot, in cleared lobbies only.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)
```

O mesmo vale para funções `on_frame` e teclas: num lobby não liberado, elas não rodam.

## Desfazer o que fica para trás {#undo-what-stays-behind}

Algumas mudanças duram mais que a chamada que as fez: um valor escrito numa tabela de configurações, uma unidade criada. Escute a troca e ponha as coisas de volta:

```lua
local original_speed

obsidian.on_require("lua/settings/player_settings", function(settings)
  original_speed = settings.run_speed
end)

obsidian.on_session(function(on)
  local settings = rawget(_G, "PlayerSettings")
  if settings and original_speed then
    settings.run_speed = on and original_speed * 1.5 or original_speed
  end
end)
```

## Escopos Híbridos {#hybrid-scopes}

Um mod de sessão muitas vezes tem código que só este jogador vê, como uma câmera, ao lado de código que muda o jogo compartilhado. Mantenha `session` como escopo principal, declare [`secondary_scopes`](wiki:manifest#hybrid-scopes) e marque o código de cada um, e o código pessoal continua rodando quando o lobby deixa de estar liberado:

```json
"scope": "session",
"secondary_scopes": [
  { "id": "view", "scope": "client", "name": { "en": "Camera" } },
  { "id": "weapon_range", "scope": "session", "option": "max_weapon_range", "name": { "en": "Max weapon range" } }
]
```

```lua
local VIEW = { secondary_scope = "view" }
obsidian.hook("CameraManager", "update", update_camera, VIEW)
obsidian.on_frame(steer, VIEW)
obsidian.hook("WeaponSettings", "range", longer_range, { secondary_scope = "weapon_range" })
obsidian.on_session(function(on) restore_range(on) end, { secondary_scope = "weapon_range" })
```

- Um escopo secundário de cliente segue as regras de um mod de cliente: só o que este jogador vê, ouve ou aperta. Uma câmera é pessoal; escala de tempo e gravidade não são, mesmo rodando numa só máquina.
- Quando os escopos secundários de sessão desligam, o jogo diz ao jogador quais, e que o resto continua funcionando.
- Um escopo secundário com `option` só fica ligado enquanto aquela opção está ligada. Com todos os escopos secundários de sessão desligados, o mod é aplicado como mod de cliente, e o jogador volta a ter os resultados da Guerra Galáctica.
- Uma biblioteca de que o mod precisa roda como mod de sessão enquanto um escopo secundário de sessão está ligado, e trata o mod inteiro como mod de sessão nas chamadas que protege.
- Um único estado Lua guarda todos os escopos secundários. Código num de cliente ainda pode alcançar código de sessão diretamente, então leia o que o código de sessão mudou a cada vez em vez de guardar uma cópia.
- Uma expansão também pode declarar um escopo secundário de sessão, para código que também roda fora do conteúdo num lobby liberado. Código sem marcação de uma expansão fica com o conteúdo.
- [Loader API: Escopos Híbridos](wiki:loader-api#hybrid-scopes) lista os argumentos.

## O que evitar {#things-to-avoid}

- **Substituir globais diretamente.** `SomeTable.func = mine` continua rodando em todo lobby, porque o carregador não consegue desligá-lo. O log do carregador lista essas mudanças de cada mod na inicialização.
- **Substituir scripts do jogo com recursos para mudanças de jogabilidade.** Um script substituído também não pode ser desligado ao vivo. O Obsidian aceita, mas num lobby público com um jogador que bloqueia mods de sessão, a mudança continua no jogo enquanto os outros mods de sessão desligam.
- **Enviar mensagens de rede próprias** a outros jogadores fora do código do jogo.

> [!NOTE]
> Mudar o progresso do jogador que roda o mod, como XP ou desbloqueios, é permitido. O Obsidian faz backup do save antes de cada aplicação e não desfaz essas mudanças.

## Escrever uma biblioteca {#libraries}

Um mod que existe para outros mods se apoiarem nele pode se declarar uma biblioteca:

```json
"scope": "library"
```

A cada aplicação, o Obsidian a torna mod de sessão se um mod de sessão ligado precisa dela, diretamente ou por outra biblioteca. Senão, ela é mod de cliente. Tudo o que ela faz como mod de sessão segue as regras normais, e ela entra no conjunto que os jogadores comparam. Uma biblioteca que substitui arquivos do jogo com `resources` os mantém nos dois casos, então deixe mudanças de jogabilidade fora deles.

O código da biblioteca precisa funcionar dos dois jeitos. `obsidian.scope()` diz qual dos dois esta aplicação deu, e `obsidian.session.guard` separa as funções que a biblioteca entrega a outros mods. Cite as que são seguras para qualquer um: leitura, desfazer e mudanças que só este jogador vê ou ouve. Todas as outras funções respondem `false, "needs_session"` quando a biblioteca roda como mod de cliente, e `false, "not_cleared"` num lobby não liberado. Uma função fora da lista é recusada.

```lua
local M = obsidian.module("lib/enemies")

-- The gameplay hooks wait for a session mod that needs them.
if obsidian.scope() == "session" then
  obsidian.hook("Damage", "apply", M.on_damage)
end

return {
  enemies = obsidian.session.guard(M, { "list", "count" }),
}
```

A biblioteca em si guarda a tabela de verdade, então ainda pode desfazer o que mudou depois que o lobby deixa de estar liberado.

## Mods de que todos precisam {#every-player}

O `consent` de um jogador diz ao mod que o jogador o permite, não que o roda. Isso basta para a maioria dos mods, já que o jogo do anfitrião decide o que acontece. Um mod que só funciona quando todos os jogos do lobby o têm, como um que muda quantos jogadores cabem num lobby, pode pedir mais:

```json
"scope": "session",
"every_player": true
```

Um mod assim só roda quando todos os outros jogadores também o rodam, na mesma versão e com os mesmos valores de opção compartilhados. Jogadores que só o permitem não contam. O Obsidian o desliga, e liga de novo, como qualquer mod de sessão, e diz ao jogador por quê.

`obsidian.session.cleared()` responde pelo mod que pergunta, e `obsidian.on_session` chama o mod quando a troca daquele mod muda.

A verificação roda depois que um jogador está no lobby, e qualquer um pode entrar primeiro, então deixe o jogo como estava até todos os jogadores estarem confirmados: para um lobby maior, mantenha o limite normal até `obsidian.session.cleared()` ser verdadeiro, e volte a ele quando `on_session` disser `false`.

## Escrever uma expansão {#expansions}

Uma expansão é conteúdo separado do jogo normal, como um modo arcade. Fora do conteúdo ela conta como mod de cliente: os jogadores mantêm o matchmaking normal, a Guerra Galáctica e o Proving Grounds. Só as missões em que o conteúdo roda saem da guerra compartilhada ([Escopos](wiki:scopes#expansions)).

```json
"scope": "expansion",
"min_obsidian": "1.2.0",
"requires": ["obsidian.api"]
```

`every_player` não combina com ela: o conteúdo só roda quando todos os jogadores rodam a expansão de qualquer forma.

### O que roda quando {#what-runs-when}

| | Fora do conteúdo | Enquanto o conteúdo roda |
|---|---|---|
| `obsidian.hook`, `obsidian.on_frame` e teclas | Desligados: os ganchos chamam só a função original | Ligados, em todos os jogos do lobby |
| `obsidian.wrap`, e teclas com `opts.always` | Ligados | Ligados |
| `obsidian.ui`, páginas de menu e sobreposições | Ligados | Ligados |
| Funções protegidas de uma biblioteca, como a maior parte do Obsidian API | `false, "not_started"` | Uso completo |
| `obsidian.session.cleared()` | `false` | `true` |

O caminho de entrada no conteúdo, uma página de menu ou uma tela desenhada com `obsidian.ui`, funciona em qualquer lobby. O conteúdo em si fica nos ganchos.

### Iniciar e parar {#starting-and-stopping}

```lua
-- An Obsidian from before expansions would run every hook in every lobby.
if not obsidian.capabilities.expansion then return end

local api = obsidian.import("obsidian.api")

api.menu.add("Arcade", function(page)
  page:action("Start", function()
    local ok, why, names = obsidian.expansion.start()
    if ok then return "Started" end
    if why == "players_missing" then return "Missing: " .. table.concat(names, ", ") end
    return why
  end)
  page:action("Stop", function() obsidian.expansion.stop() end)
end)

-- Only while the content runs, in every game in the lobby.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)

obsidian.on_session(function(on)
  if on then set_up() else undo() end
end)
```

`obsidian.expansion.start()` funciona no jogo do anfitrião. Ele confere se todos os outros jogadores do lobby rodam a mesma expansão, na mesma versão e com os mesmos valores de opção compartilhados. Então o conteúdo começa, e todos os outros jogos do lobby seguem em cerca de um segundo. `obsidian.on_session(true)` roda em cada um deles, e `obsidian.on_session(false)` quando o conteúdo termina, então esses são os lugares para preparar e desfazer. Os outros jogos só ficam sabendo do conteúdo desse jeito, pelo anfitrião.

A resposta é `true`, ou `false` e um motivo:

| Motivo | Quando |
|---|---|
| `"players_missing"` | Alguém no lobby não a roda. Um terceiro valor lista os nomes, e o anfitrião os vê num aviso |
| `"not_host"` | Outro jogador é o anfitrião do lobby |
| `"busy"` | O conteúdo de outra expansão está rodando. Roda uma de cada vez |
| `"not_expansion"` | O mod que chama não é uma expansão |

`obsidian.expansion.stop()` encerra o conteúdo, no jogo do anfitrião, e responde `false, "not_running"` quando ele não está rodando. Sair do lobby também o encerra. Quando o anfitrião sai, o jogador que assume o lobby continua o conteúdo. `obsidian.expansion.running()` diz se o conteúdo do mod que chama está rodando agora, e `running(id)` pergunta sobre outro mod.

### Jogadores que entram {#players-who-join}

Enquanto o conteúdo roda, o lobby do anfitrião aparece no matchmaking só para jogadores que rodam a mesma expansão na mesma versão. Um jogador que tenta entrar sem ela, por um convite ou um amigo, é barrado antes de carregar, e o anfitrião vê o nome. Quem chega à missão sem ela mesmo assim é removido com a expulsão própria do jogo, e pode voltar quando o conteúdo termina. Jogadores entrando ou saindo nunca desligam o conteúdo para quem já está nele.

### Bibliotecas de que uma expansão precisa {#libraries-an-expansion-requires}

Uma biblioteca de que uma expansão precisa conta como mod de cliente na aplicação, a não ser que um mod de sessão também precise dela. `obsidian.scope()` responde `"session"` nela, para que a biblioteca prepare tudo o que uma expansão pode usar, e no resto do tempo ela roda como uma biblioteca de cliente rodaria. Enquanto o conteúdo roda, as funções protegidas da biblioteca funcionam para a expansão, e `obsidian.on_session` avisa a biblioteca quando o conteúdo começa e termina, para que ela possa desfazer o que mudou. Mods de cliente que usam a mesma biblioteca continuam recebendo `"needs_session"` dessas funções.

### Bom saber {#things-to-know}

- Arquivos substituídos com `resources` ficam no jogo o tempo todo, com ou sem conteúdo. Limite-os ao que um mod de cliente poderia mudar, como fontes e imagens.
- Valores de opção compartilhados contam, como num mod de sessão. Marque os pessoais com `"scope": "client"`.
- Os resultados de uma missão em que o conteúdo rodou nunca chegam à Guerra Galáctica, mesmo quando o conteúdo para antes das telas de resultado. A próxima missão sem o conteúdo conta de novo.
- `min_obsidian` impede o Obsidian 1.1 de aplicar o mod, e pede ao jogador para atualizar. O Obsidian 1.0 não o lê e roda o mod como não especificado, então a verificação de capacidade no topo do script de entrada mantém o mod fora do jogo nesse caso.

## Opções em mods de sessão {#options-in-session-mods}

Todo valor de opção conta quando o Obsidian compara os conjuntos dos jogadores, a não ser que seja marcado com `"scope": "client"` ou fique num escopo secundário de cliente. O mesmo vale para uma expansão. Marque as opções que só mudam o que este jogador vê ([Opções e teclas](wiki:options-and-keys#personal-options-in-session-mods)).
