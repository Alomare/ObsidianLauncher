---
title: "Escopos: cliente e sessão"
group: Jogar online
summary: Por que todo mod é de cliente ou de sessão, e o que cada um pode fazer.
---

HELLDIVERS é um jogo cooperativo. Um mod que muda inimigos, armas ou missões muda o jogo de todo mundo no lobby, não só o seu. O Obsidian separa cada mod em um de dois escopos para que um jogador que nunca instalou mod nenhum não seja afetado por um.

## Mods de cliente {#client-mods}

Um mod de cliente muda só o que você vê, ouve ou aperta: a câmera, o HUD, os controles, os menus, o visual e o som locais. Nada do que ele faz chega aos outros jogadores. Mods de cliente rodam em qualquer lugar, em qualquer lobby, com qualquer pessoa.

O Obsidian Qualidade de Vida é um mod de cliente.

## Mods de sessão {#session-mods}

Um mod de sessão pode mudar o jogo compartilhado: balanceamento, inimigos, spawns, missões, física, qualquer coisa que outros jogadores perceberiam. Mods de sessão só rodam num lobby em que todos os outros jogadores concordaram com eles ([Jogar online](wiki:multiplayer)). Sozinho, você sempre pode usá-los.

## Mods não especificados {#unspecified-mods}

Um mod que não declara escopo fica como não especificado, e o mesmo vale para mods antigos importados de bundles inteiros, porque o Obsidian não tem como saber o que eles mudam. Mods não especificados rodam como mods de cliente, em qualquer lobby: ficar de fora do multijogador por um palpite seria pior. Se um deles mudar o jogo compartilhado, os outros jogadores podem perceber, então jogue com quem também o tem.

Um mod que pede para rodar só enquanto todos os jogadores o rodam (`every_player`) é um mod de sessão, com ou sem escopo.

## Bibliotecas {#libraries}

Uma biblioteca é código usado por outros mods, como o Obsidian API. Ela assume o escopo dos mods que a usam: é um mod de sessão enquanto um mod de sessão ligado precisar dela e um mod de cliente no resto do tempo. Uma biblioteca sozinha, ou usada só por mods de cliente, não tira você da busca de partidas normal nem bloqueia seus resultados na Guerra Galáctica.

## No aplicativo {#in-the-app}

Cada linha da lista de mods tem um losango para o escopo: verde para cliente, laranja para sessão, azul para biblioteca e cinza para não especificado. Passe o mouse por cima para ver o nome. Os detalhes de cada mod também mostram o escopo.

## Quem decide o escopo {#who-decides-the-scope}

O autor do mod declara o escopo no arquivo `obsidian.json`. O Obsidian então faz valer: os hooks de um mod de sessão se desligam sozinhos quando o lobby não está liberado, então o autor não precisa escrever nada disso ([Escrever mods de sessão](wiki:session-mods)).

> [!WARNING]
> O escopo é uma promessa do autor. Um mod que se diz de cliente mas muda o jogo compartilhado quebra as regras, e o Obsidian nem sempre consegue perceber. [O que as proteções não fazem](wiki:honest-limits) fala disso.
