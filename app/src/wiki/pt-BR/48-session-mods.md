---
title: Escrever mods de sessão
group: Criar mods
summary: Como mudar o jogo compartilhado de um jeito que as proteções continuem funcionando para você.
---

Um mod de sessão muda algo que outros jogadores percebem: dano, inimigos, spawns, missões. O loader decide quando um mod assim pode rodar ([Jogar online](wiki:multiplayer)) e liga e desliga o mod conforme os jogadores entram e saem. A sua parte é fazer cada mudança pelo loader, para que ela possa ser desligada.

Um mod cujas mudanças acontecem todas dentro de um conteúdo à parte, como um modo de jogo com uma tela de missão própria, pode ser uma [expansão](#expansions) e deixar todas as outras missões como estão.

## Defina o escopo {#set-the-scope}

```json
"scope": "session"
```

Sem o campo, o mod fica não especificado e roda como mod de cliente. Só use `client` quando nada do que o mod faz chega aos outros jogadores.

## Mude o jogo por hooks {#change-the-game-through-hooks}

Os hooks de um mod de sessão chamam o código original do jogo sozinhos enquanto o lobby não está liberado. Um mod feito só de hooks não precisa de nenhum código extra para ser seguro:

```lua
-- Dobra o dano de cada tiro, só em lobbies liberados.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)
```

O mesmo vale para funções `on_frame` e teclas: num lobby que não está liberado, elas não rodam.

## Desfaça o que fica para trás {#undo-what-stays-behind}

Algumas mudanças duram mais que a chamada que as fez: um valor gravado numa tabela de configuração, uma unidade que você criou. Escute a troca e ponha as coisas de volta:

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

## O que evitar {#things-to-avoid}

- **Substituir globais diretamente.** `SomeTable.func = mine` continua rodando em qualquer lobby, porque o loader não consegue desligar. O loader lista mudanças assim no log, para cada mod, na inicialização.
- **Substituir scripts do jogo por recursos para mudar a jogabilidade.** Um script substituído também não desliga na hora. O Obsidian aceita, mas num lobby público com um jogador que bloqueia mods de sessão, a mudança continua no seu jogo enquanto o resto dos seus mods de sessão desliga.
- **Mandar mensagens de rede próprias** para outros jogadores fora do código do próprio jogo.

> [!NOTE]
> Mudar o seu próprio progresso, como XP ou desbloqueios, é permitido. O Obsidian faz backup do save antes de cada aplicação e não desfaz essas mudanças.

## Escrever uma biblioteca {#libraries}

Um mod que existe para outros mods usarem pode se declarar uma biblioteca:

```json
"scope": "library"
```

A cada aplicação, o Obsidian o trata como mod de sessão se um mod de sessão ligado precisar dele, diretamente ou por outra biblioteca. Nos outros casos, ele é um mod de cliente. O que ele faz como mod de sessão segue as regras de sempre, e ele entra no conjunto que os jogadores comparam. Uma biblioteca que substitui arquivos do jogo com `resources` mantém esses arquivos nos dois casos, então não coloque mudanças de jogabilidade neles.

O seu código precisa funcionar dos dois jeitos. `obsidian.scope()` diz qual dos dois esta aplicação definiu, e `obsidian.session.guard` separa as funções que você entrega a outros mods. Liste as que são seguras para qualquer um: ler, desfazer e mudanças que só este jogador vê ou ouve. Qualquer outra função responde `false, "needs_session"` quando a biblioteca roda como mod de cliente, e `false, "not_cleared"` num lobby que não está liberado. Uma função que você esquecer de listar é recusada.

```lua
local M = obsidian.module("lib/enemies")

-- Os hooks de jogabilidade esperam um mod de sessão que precise deles.
if obsidian.scope() == "session" then
  obsidian.hook("Damage", "apply", M.on_damage)
end

return {
  enemies = obsidian.session.guard(M, { "list", "count" }),
}
```

O seu próprio código continua com a tabela real, então ainda consegue desfazer as mudanças depois que o lobby deixa de estar liberado.

## Mods que todos os jogadores precisam ter {#every-player}

O `consent` de um jogador diz ao seu mod que ele o permite, não que o roda. Isso basta para a maioria dos mods, já que o jogo do host decide o que acontece. Um mod que só funciona quando todos os jogos do lobby o têm, como um que muda quantos jogadores cabem no lobby, pode pedir mais:

```json
"scope": "session",
"every_player": true
```

Um mod assim só roda quando todos os outros jogadores também o rodam, na mesma versão e com os mesmos valores de opção compartilhados. Quem só permite não conta. O Obsidian liga e desliga o mod como qualquer mod de sessão e diz ao jogador o motivo. Os jogadores dele também têm um grupo de matchmaking próprio, então só encontram uns aos outros.

`obsidian.session.cleared()` responde pelo mod que pergunta, e `obsidian.on_session` chama o mod quando o interruptor dele muda.

A verificação roda quando o jogador já está no lobby. Convites ainda podem cruzar grupos, então deixe o jogo como estava até todos os jogadores estarem confirmados: num lobby maior, mantenha o limite normal até `obsidian.session.cleared()` ser `true`, e volte a ele quando `on_session` disser `false`.

## Escrever uma expansão {#expansions}

Uma expansão é um conteúdo separado do jogo normal, como um modo arcade. Fora do conteúdo, ela conta como mod de cliente: os jogadores mantêm o matchmaking normal, a Guerra Galáctica e Proving Grounds. Só as missões em que o conteúdo roda ficam fora da guerra compartilhada ([Escopos](wiki:scopes#expansions)).

```json
"scope": "expansion",
"min_obsidian": "1.2.0",
"requires": ["obsidian.api"]
```

`every_player` não combina com ela: o conteúdo já só roda quando todos os jogadores rodam a expansão.

### O que roda e quando {#what-runs-when}

| | Fora do conteúdo | Enquanto o conteúdo roda |
|---|---|---|
| `obsidian.hook`, `obsidian.on_frame` e teclas | Desligados: os hooks só chamam a função do jogo | Ligados, em todos os jogos do lobby |
| `obsidian.wrap`, e teclas com `opts.always` | Ligados | Ligados |
| `obsidian.ui`, páginas de menu e overlays | Ligados | Ligados |
| Funções protegidas de uma biblioteca, como a maior parte do Obsidian API | `false, "not_started"` | Uso completo |
| `obsidian.session.cleared()` | `false` | `true` |

O caminho para o conteúdo, uma página de menu ou uma tela desenhada com `obsidian.ui`, funciona em qualquer lobby. O conteúdo em si fica nos hooks.

### Iniciar e parar {#starting-and-stopping}

```lua
-- Um Obsidian anterior às expansões rodaria todos os hooks em qualquer lobby.
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

-- Só enquanto o conteúdo roda, em todos os jogos do lobby.
obsidian.hook("Damage", "apply", function(original, target, amount, ...)
  return original(target, amount * 2, ...)
end)

obsidian.on_session(function(on)
  if on then set_up() else undo() end
end)
```

`obsidian.expansion.start()` funciona no jogo do host. Ele confere se todos os outros jogadores do lobby rodam a mesma expansão, na mesma versão e com os mesmos valores de opção compartilhados. Então o conteúdo começa, e todos os outros jogos do lobby seguem em cerca de um segundo. `obsidian.on_session(true)` roda em cada um deles, e `obsidian.on_session(false)` quando o conteúdo termina, então é ali que o mod prepara e desfaz as coisas. Os outros jogos só ficam sabendo do conteúdo desse jeito, pelo host.

A resposta é `true`, ou `false` e um motivo:

| Motivo | Quando |
|---|---|
| `"players_missing"` | Alguém no lobby não a roda. Um terceiro valor lista os nomes, e o host os vê num aviso |
| `"not_host"` | Outro jogador é o host do lobby |
| `"busy"` | O conteúdo de outra expansão está rodando. Só um roda por vez |
| `"not_expansion"` | O mod que chama não é uma expansão |

`obsidian.expansion.stop()` termina o conteúdo, no jogo do host, e responde `false, "not_running"` quando ele não está rodando. Sair do lobby também o termina. Quando o host sai, o jogador que assume o lobby continua o conteúdo. `obsidian.expansion.running()` diz se o conteúdo do mod que chama está rodando agora, e `running(id)` pergunta sobre outro mod.

### Jogadores que entram {#players-who-join}

Enquanto o conteúdo roda, o lobby do host só aparece no matchmaking para jogadores que rodam a mesma expansão na mesma versão. Um jogador que tenta entrar sem ela, por convite ou pela lista de amigos, é recusado antes de carregar, e o host vê o nome. Quem chega à missão sem ela mesmo assim é removido com o kick do próprio jogo, e pode voltar quando o conteúdo terminar. Jogadores que entram ou saem nunca desligam o conteúdo para quem já está nele.

### Bibliotecas que uma expansão exige {#libraries-an-expansion-requires}

Uma biblioteca que uma expansão exige conta como mod de cliente na aplicação, a não ser que um mod de sessão também a exija. `obsidian.scope()` responde `"session"` nela, para que a biblioteca prepare tudo o que uma expansão pode usar, e no resto do tempo ela roda como uma biblioteca de cliente rodaria. Enquanto o conteúdo roda, as funções protegidas da biblioteca funcionam para a expansão, e `obsidian.on_session` avisa a biblioteca quando o conteúdo começa e termina, para que ela possa desfazer o que mudou. Mods de cliente que usam a mesma biblioteca continuam recebendo `"needs_session"` dessas funções.

### Bom saber {#things-to-know}

- Arquivos substituídos com `resources` ficam no jogo o tempo todo, com ou sem conteúdo. Eles devem se limitar ao que um mod de cliente poderia mudar, como fontes e imagens.
- Valores de opção compartilhados contam, como num mod de sessão. As opções pessoais levam `"scope": "client"`.
- Os resultados de uma missão em que o conteúdo rodou nunca chegam à Guerra Galáctica, mesmo quando o conteúdo para antes das telas de resultado. A próxima missão sem o conteúdo volta a contar.
- `min_obsidian` impede o Obsidian 1.1 de aplicar o mod e pede ao jogador que atualize. O Obsidian 1.0 não lê esse campo e roda o mod como não especificado, então a verificação de recurso no topo do script de entrada mantém o mod fora do jogo nesse caso.

## Opções em mods de sessão {#options-in-session-mods}

Todo valor de opção conta quando o Obsidian compara os conjuntos dos jogadores, a não ser que a opção tenha `"scope": "client"`. O mesmo vale para uma expansão. As opções que só mudam o que este jogador vê devem ser marcadas assim ([Opções e teclas](wiki:options-and-keys#personal-options-in-session-mods)).
