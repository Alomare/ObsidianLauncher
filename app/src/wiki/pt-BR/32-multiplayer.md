---
title: Jogar online
group: Jogar online
summary: Matchmaking, lobbies públicos e quando os mods de sessão rodam num lobby.
---

## Lobbies públicos {#the-multiplayer-switch}

O Obsidian só segura mods de sessão em lobbies públicos, onde estranhos podem entrar. Num lobby só para amigos ou privado, todos escolheram com quem jogar, então mods de sessão rodam ali com qualquer pessoa, inclusive jogadores vanilla.

**Bloquear mods de sessão de outros jogadores em lobbies públicos** é um dos interruptores das [Proteções](wiki:guardrails), no topo da janela. O interruptor começa desligado, pertence ao perfil ativo e vale a partir do próximo início do jogo. Ligado, os mods de sessão de outros jogadores desligam enquanto se está num lobby público com eles. Ele não decide se os mods de sessão do jogador que o ligou rodam.

## Matchmaking {#matchmaking}

Mods de sessão não mudam quem um jogador encontra nem quem pode entrar. Jogadores vanilla veem um lobby com mods e podem entrar nele, e vice-versa. Num lobby público, mods de sessão desligam enquanto um jogador vanilla está ali (veja abaixo).

| Situação | Quem o matchmaking encontra |
|---|---|
| Sem mods de sessão, ou com mods de sessão que desligam ao vivo | Todos. O lobby aparece para todos, e expansões não mudam isso |
| Mods de sessão que não desligam ao vivo | Todos. Enquanto o lobby é público, só jogadores do Obsidian podem entrar nele, e entrar em outro lobby público só funciona quando o anfitrião de lá também roda esse tipo de conteúdo |
| Hospedando o conteúdo de uma expansão | O lobby aparece só para jogadores que rodam aquela expansão, na mesma versão |

Com uma expansão aplicada, os lobbies de jogadores rodando o conteúdo dessa mesma expansão também aparecem, onde quer que o matchmaking ponha o jogador.

O primeiro carregamento na nave mostra uma mensagem que diz qual caso vale: os mods não afetam outros jogadores, os mods de sessão desligam em alguns lobbies públicos, algum conteúdo mantém jogadores sem o Obsidian fora de um lobby público, ou uma expansão pode separar o jogador de quem não a tem enquanto o conteúdo roda.

## Quando os mods de sessão rodam {#when-session-mods-run}

Sozinho, e num lobby só para amigos ou privado, mods de sessão sempre rodam. Num lobby público, eles só rodam quando todos os outros jogadores:

1. rodam o mesmo conjunto de mods de sessão, nas mesmas versões e com os mesmos valores de opção, ou
2. rodam o Obsidian e não bloqueiam mods de sessão de outros jogadores.

Mods de cliente sempre rodam. Alguns exemplos, do ponto de vista do jogador A com mods de sessão, num lobby público:

| Outros jogadores | Mods de sessão de A |
|---|---|
| B roda o mesmo conjunto | Ligados |
| B roda outros mods de sessão, sem bloquear | Ligados |
| B roda outros mods de sessão, bloqueando | Desligados |
| B roda só mods de cliente, sem bloquear | Ligados |
| B tem HELLDIVERS vanilla | Desligados |
| B sem bloquear, C bloqueando com outro conjunto | Desligados: todos precisam permitir |

Num lobby só para amigos ou privado, toda linha fica Ligados. A verificação roda de novo sempre que alguém entra, sai ou muda de configuração, e quando o anfitrião muda quem pode entrar. Os mods ligam e desligam ao vivo, e um mod de sessão pode desfazer o que mudou quando isso acontece.

## Escopos Híbridos {#hybrid-scopes}

Um mod com [Escopos Híbridos](wiki:scopes#hybrid-scopes) segue as mesmas regras, um escopo secundário por vez. Os escopos secundários de sessão rodam onde mods de sessão rodam, e os de cliente rodam em qualquer lobby. Quando os escopos secundários de sessão desligam num lobby público, o jogo os cita e diz que o resto continua funcionando.

| Outros jogadores | Câmera de A (escopo secundário de cliente) | Alcance das armas de A (escopo secundário de sessão) |
|---|---|---|
| B roda o mesmo conjunto | Ligada | Ligado |
| B tem HELLDIVERS vanilla | Ligada | Desligado |

As opções de escopos secundários de cliente não contam quando o Obsidian compara conjuntos, e um escopo secundário desligado nas opções do mod não conta para nada.

## Mods de que todos precisam {#mods-every-player-needs}

Alguns mods só funcionam quando todos os têm, como um que deixa mais jogadores entrarem num lobby. O autor os marca, e eles seguem uma regra mais rígida: um mod assim só roda quando todos os outros jogadores também o rodam, na mesma versão e com os mesmos valores de opção, em qualquer lobby, inclusive só para amigos. Não bloquear não conta para eles. Enquanto um jogador sem ele está no lobby, aquele mod desliga e o jogo avisa, e os outros mods de sessão seguem as regras normais.

> [!TIP]
> Valores de opção que o autor marca como pessoais, como a distância da câmera, não contam quando o Obsidian compara conjuntos. Um amigo com outra distância de câmera continua batendo.

## Expansões {#expansions}

Uma [expansão](wiki:scopes#expansions) roda como mod de cliente até o anfitrião iniciar o conteúdo. O início confere cada jogador do lobby: cada um precisa rodar a mesma expansão, na mesma versão e com os mesmos valores de opção. Se alguém não roda, nada começa e o anfitrião fica sabendo quem. O interruptor das Proteções não conta aqui.

Quando o conteúdo começa, todos os jogos do lobby seguem o anfitrião. Um jogador que tenta entrar sem a expansão é barrado, e o anfitrião vê o nome. Ninguém entrando ou saindo desliga o conteúdo para quem já está nele. Quando o anfitrião para o conteúdo, ou um jogador sai do lobby, a expansão volta a ser um mod de cliente para aquele jogador.

## Mods que não desligam ao vivo {#mods-that-can-t-switch-off-live}

Arquivos do jogo que um mod de sessão substitui, listados em `resources` no manifesto do mod, não podem ser desligados no meio de uma partida. Enquanto um jogador com um mod assim hospeda um lobby público, jogadores vanilla não conseguem entrar, e o jogo diz a eles que a versão não bate. Esse jogador também não pode levar esse conteúdo para um lobby público cujo anfitrião não roda esse tipo de conteúdo. Lobbies só para amigos e privados ficam abertos a todos.

Quando um jogador que bloqueia mods de sessão acaba num lobby público assim, o carregador registra um aviso em `loader.log`, e os mods de sessão que podem desligar ficam desligados. Saia do lobby, ou jogue num lobby só para amigos.

## A aba Com mods {#the-modded-tab}

O terminal de multijogador da nave tem três abas com o Obsidian: **Públicos**, **Com mods** e a aba de amigos do jogo. Um anfitrião com o Obsidian publica uma lista de mods com o lobby: o nome e a versão de cada mod de sessão e expansão que roda, e de cada mod de cliente, a não ser que o anfitrião os esconda com as [Proteções](wiki:guardrails#the-lobby-list). **Com mods** lista só os lobbies públicos com essa lista, e as outras abas marcam esses lobbies como **COM MODS**. Selecionar um mostra a lista em **Mods neste lobby**. A lista é só informação: quais mods rodam depois que um jogador entra segue as regras acima.

Num save Sandbox, a aba **Públicos** vira **Sandbox públicos** e lista só lobbies de outros jogadores Sandbox ([Saves](wiki:saves#the-sandbox-matchmaking-pool)).

## Convites {#invites}

Convites da Steam e entrar no jogo de amigos podem atravessar grupos de matchmaking. As regras acima continuam decidindo quais mods rodam depois que o jogador está no lobby.
