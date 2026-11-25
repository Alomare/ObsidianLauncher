---
title: Obsidian API
group: Criar mods
summary: Uma biblioteca para mods: dados do jogo por nome, mudanças conferidas, efeitos, eventos e ações prontas.
---

O Obsidian API é um mod em que outros mods se apoiam. Ele encontra coisas no jogo pelo nome, muda-as de jeitos que o jogo aceita e desfaz as mudanças quando precisa. O Obsidian Trainer é feito sobre ele e não usa nada que outros mods não possam usar.

Esta página é a visão geral. Cada chamada, com argumentos, respostas e exemplos, está no [REFERENCE.md](https://github.com/Alomare/ObsidianAPI/blob/main/REFERENCE.md), que também vem dentro da pasta do Obsidian API no Obsidian Kit, onde uma ferramenta de IA trabalhando num mod pode lê-lo.

## Usar {#using-it}

Liste-o em `requires`, e o Obsidian o liga junto com o mod e o inicia antes:

```json
"scope": "session",
"entry": "main.lua",
"requires": ["obsidian.api"]
```

Depois peça por ele no script:

```lua
local api = obsidian.import("obsidian.api")

obsidian.keybind("panic", function()
  api.player.heal()
  api.enemies.kill_near(20)
end)
```

## Mods de cliente e mods de sessão {#client-mods-and-session-mods}

O Obsidian API é uma [biblioteca](wiki:session-mods#libraries), então assume o escopo dos mods que precisam dele. Um mod de cliente pode precisar dele e continuar sendo mod de cliente, sem efeito no matchmaking nem na Guerra Galáctica. Um mod de cliente pode ler qualquer coisa, usar o que só este jogador vê ou ouve (`api.ui`, `api.map`, `api.audio` e `api.music`) e escutar `api.events`. Toda chamada que muda o jogo compartilhado responde `false, "needs_session"` num mod de cliente.

Num mod de sessão, essas chamadas funcionam enquanto o lobby está liberado. Quando o lobby deixa de estar liberado, o Obsidian API tira os efeitos e as edições do mod, e as chamadas respondem `false, "not_cleared"`. Quando o lobby é liberado de novo, ele os devolve. Uma expansão recebe todas as chamadas enquanto o conteúdo roda, e `false, "not_started"` fora dele ([Escrever uma expansão](wiki:session-mods#expansions)).

## Como as chamadas respondem {#how-calls-answer}

Ações respondem `true`, ou `false` e um motivo curto como `"host_only"`, `"no_avatar"` ou `"not_in_mission"`. Chamadas que criam algo, como `api.bots.add`, respondem a coisa nova, ou `nil` e um motivo. Uma recusa comum nunca gera erro:

```lua
local ok, why = api.stratagems.call("stratagem_resupply", api.world.cursor())
if not ok then obsidian.toast("No resupply: " .. why) end
```

## O que ele tem {#what-it-holds}

| Tabela | Para |
|---|---|
| `api.game` | O estado atual, o jogador local, o anfitrião e escritas seguras em valores protegidos |
| `api.lobby` | Os jogadores do lobby, e textos curtos compartilhados com todas as máquinas nele |
| `api.catalog` | Armas, estratagemas, vantagens, inimigos, veículos e itens por nome: o jogo base e o DLC que o jogador possui |
| `api.data` | O que pode ser editado e criado no mundo, gerado a partir dos dados do jogo |
| `api.edits` | Mudanças em tabelas de configuração, conferidas e registradas, com desfazer |
| `api.effects` | Mudanças que continuam entre renascimentos e missões, como modo deus ou velocidade, e efeitos personalizados |
| `api.events` | Chamadas quando o jogo informa uma morte de inimigo, uma morte de jogador, um objetivo, um estratagema e mais |
| `api.player` | Vida, vidas, renascimento, teletransporte, aparência e efeitos de status |
| `api.gear` | Armas, mochilas, vantagens, drones e reabastecimento, e os atributos de uma única arma |
| `api.perks`, `api.cosmetics` | Vantagens, capacetes, armaduras e capas personalizados, cada um feito a partir de um do jogo; outros jogadores sem o mesmo mod veem o original |
| `api.vehicles` | O veículo sendo dirigido, reparos e reabastecimento |
| `api.stratagems` | Recargas, usos, chamadas, e novos estratagemas feitos a partir dos do jogo |
| `api.enemies` | Encontrar, criar, ferir e remover inimigos, e o quanto os encontros pressionam |
| `api.mission` | Objetivos, extração, amostras, modificadores do Proving Grounds e encerrar uma missão |
| `api.combat` | Multiplicadores de dano para jogadores e inimigos |
| `api.world` | Posições, objetos de cena, a câmera, luzes, hora do dia e a aparência de uma missão |
| `api.map` | O mapa da missão deste jogador: zoom, pings e marcadores |
| `api.audio`, `api.music` | Volumes, sons e a trilha sonora |
| `api.menu` | A janela dentro do jogo que **F6** abre, onde cada mod tem uma pasta de páginas |
| `api.mod_settings` | Uma página de configurações do mod em **Mods** no menu de pausa do jogo, com grupos que abrem uma página separada ([Opções no menu de pausa](wiki:options-and-keys#options-in-the-pause-menu)) |
| `api.ui` | Dicas, pings no mapa, falas sobre um Helldiver, uma linha no chat de texto, os menus da nave e textos do jogo mostrados com outras palavras |
| `api.progress` | O save: amostras, desbloqueios, melhorias e o equipamento |
| `api.console` | Os comandos de desenvolvedor do jogo, menos os que chegam ao servidor compartilhado |
| `api.bots` | Helldivers que esta máquina controla, com ordens, equipamento e ajuda |
| `api.brain` | As decisões dos bots, como funções simples que um mod ou um teste pode chamar |
| `api.nav` | Rotas a pé para bots e mods |
| `api.capture` | Capturas de tela do jogo, salvas como arquivos de imagem |

## Memory Bridge {#memory-bridge}

A opção **Memory Bridge** do Obsidian API, desligada por padrão, roda no jogo aberto o código Lua enviado por uma ferramenta de desenvolvimento, para testar uma ideia de mod sem reiniciar. A ferramenta é o `tools/bridge.py` do [HD1 Lua Modding Skill](https://github.com/Alomare/HD1-LuaModdingSkill). A ponte funciona só jogando sozinho ou num lobby que não é público, e uma partida com ela ligada não envia resultados da Guerra Galáctica nem dos placares.

## Regras que vale conhecer {#rules-worth-knowing}

As chamadas do Obsidian API já seguem estas regras. Elas importam quando um mod vai além da API, direto nas tabelas e funções do jogo:

> [!WARNING]
> O jogo guarda cópias com hash de valores protegidos como `hitpoints_real`, `rounds`, `cooldown_timer`, `uses_left`, `num_grenades` e `extra_lives`. Escrever um deles diretamente dispara a verificação antiadulteração do jogo, que corta o jogador da rede até o jogo reiniciar. Use `api.game.protected` ou um comando enfileirado. `api.edits` recusa essas chaves.

- Um comando enfileirado que o jogo não conhece, ou uma unidade criada cujo pacote não está carregado, para o jogo com um erro que nenhum script consegue capturar.
- Um temporizador que o jogo roda enquanto está acima de zero, como o `drop_time` de um hellpod, nunca termina em 0. Use 0.01.
- Os vetores do jogo duram só o quadro em que foram criados. Copie os números para guardá-los.
- XP e patente chegam aos placares compartilhados com os próximos resultados normais, mesmo depois de limpar.

O REFERENCE.md tem a lista completa.
