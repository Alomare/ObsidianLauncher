---
title: Jogar online
group: Jogar online
summary: Matchmaking, o interruptor de multijogador e quando os mods de sessão rodam num lobby.
---

## O interruptor de multijogador {#the-multiplayer-switch}

**Permitir mods de sessão de outros jogadores no meu jogo** é um dos interruptores de [Proteções](wiki:guardrails), no topo da janela. Ele pertence ao perfil ativo e vale a partir da próxima abertura do jogo. Ligar o interruptor diz uma coisa só: outros jogadores do Obsidian podem rodar os mods de sessão deles no seu jogo.

Ele não decide se os seus próprios mods de sessão rodam. Isso depende dos outros jogadores, como explicado abaixo.

## Matchmaking {#matchmaking}

O Obsidian muda quais lobbies públicos você encontra, para que um jogador sem mods nunca caia num jogo modificado por acaso.

| A sua configuração | Quem você encontra no matchmaking |
|---|---|
| Sem mods de sessão, interruptor desligado | Todo mundo. É o jogo normal. Expansões não mudam isso |
| Mods de sessão aplicados, ou interruptor ligado | Só outros jogadores do Obsidian com mods de sessão ou com o interruptor ligado |
| Um mod que todos os jogadores precisam ter aplicado | Só jogadores que rodam esse mesmo mod, na mesma versão |
| Hospedando o conteúdo de uma expansão | O lobby só aparece para jogadores que rodam essa expansão, na mesma versão |

Ou seja: com o interruptor ligado, o seu lobby aparece para quem roda mods de sessão, e você deixa de ver os lobbies sem mods. Desligue quando quiser o matchmaking normal de volta.

Com uma expansão aplicada, a lista também mostra os lobbies de jogadores que estão no conteúdo dessa mesma expansão, qualquer que seja o grupo de matchmaking do jogador.

Os filtros de lobby do terminal de multijogador mostram **(só Obsidian)** enquanto você está no grupo do Obsidian, para que uma lista vazia não pareça defeito.

## Quando os mods de sessão rodam {#when-session-mods-run}

Num lobby, os seus mods de sessão só rodam quando cada um dos outros jogadores:

1. roda o mesmo conjunto de mods de sessão, nas mesmas versões e com os mesmos valores de opção, ou
2. está com o interruptor de multijogador ligado.

Sozinho, os seus mods de sessão sempre rodam. Mods de cliente sempre rodam.

Alguns exemplos, do ponto de vista do jogador A, que tem mods de sessão:

| Outros jogadores | Mods de sessão de A |
|---|---|
| B roda o mesmo conjunto | Ligados |
| B roda outros mods de sessão, interruptor ligado | Ligados |
| B roda outros mods de sessão, interruptor desligado | Desligados |
| B sem mods, interruptor ligado | Ligados |
| B com HELLDIVERS original | Desligados |
| B ligado, C desligado e com outro conjunto | Desligados: todos precisam concordar |

A verificação roda de novo sempre que alguém entra, sai ou muda a configuração. Os mods ligam e desligam na hora, e um mod de sessão pode desfazer o que mudou quando isso acontece.

> [!TIP]
> Valores de opção que o autor marcou como pessoais, como a distância da câmera, não contam quando o Obsidian compara conjuntos. Um amigo pode escolher outra distância de câmera e ainda bater com o seu.

## Mods que todos os jogadores precisam ter {#mods-every-player-needs}

Alguns mods só funcionam quando todo mundo os tem, como um que deixa mais jogadores entrarem no lobby. O autor marca esses mods, e eles seguem uma regra mais rígida: o mod só roda quando todos os outros jogadores também o rodam, na mesma versão e com os mesmos valores de opção. O interruptor de multijogador não conta para eles. Quando alguém sem o mod está no seu lobby, só esse mod desliga, e o jogo avisa. Os seus outros mods de sessão seguem as regras de sempre.

Esses mods também têm um grupo de matchmaking próprio, então você só encontra jogadores que rodam os mesmos.

> [!TIP]
> Valores de opção que o autor marca como pessoais, como a distância da câmera, não contam quando o Obsidian compara os conjuntos. Um amigo pode escolher outra distância de câmera e ainda combinar com você.

## Expansões {#expansions}

Uma [expansão](wiki:scopes#expansions) roda como mod de cliente até o host iniciar o conteúdo. O início confere todos os jogadores do lobby: cada um precisa rodar a mesma expansão, na mesma versão e com os mesmos valores de opção. Se alguém não roda, nada começa e o host fica sabendo quem. O interruptor de multijogador não conta aqui.

Depois que o conteúdo começa, todos os jogos do lobby seguem o host. Um jogador que tenta entrar sem a expansão é recusado, e o host vê o nome. Ninguém que entra ou sai desliga o conteúdo para quem já está nele. Quando o host para o conteúdo, ou quando o jogador sai do lobby, a expansão volta a ser um mod de cliente.

## Mods que não desligam na hora {#mods-that-can-t-switch-off-live}

Parte do conteúdo de sessão não pode ser desligada no meio do jogo: substituições de dados e mods antigos importados. Se um jogador que não concordou entrar num lobby assim, por exemplo por convite, o loader registra um aviso no `loader.log` e os mods de sessão que podem desligar continuam desligados. Saia do lobby, ou jogue com quem concordou.

## Convites {#invites}

Convites da Steam e entrar no jogo de amigos podem atravessar os grupos de matchmaking. As regras acima continuam decidindo quais mods rodam depois que você entra.
