---
title: Escopos
group: Jogar online
summary: Mods de cliente, de sessão, expansões, bibliotecas e mods não especificados, e o que cada um pode fazer.
---

HELLDIVERS é um jogo cooperativo. Um mod que muda inimigos, armas ou missões muda o jogo de todo mundo no lobby, não só o de quem o instalou. O Obsidian separa cada mod em um escopo para que um jogador que nunca instalou mod nenhum não seja afetado por um.

## Mods de cliente {#client-mods}

Um mod de cliente muda só o que o jogador vê, ouve ou aperta: a câmera, o HUD, os controles, os menus, o visual e o som locais. Nada do que ele faz chega aos outros jogadores. Mods de cliente rodam em qualquer lugar, em qualquer lobby, com qualquer pessoa.

O Obsidian Qualidade de Vida é um mod de cliente.

## Mods de sessão {#session-mods}

Um mod de sessão pode mudar o jogo compartilhado: balanceamento, inimigos, spawns, missões, física, qualquer coisa que outros jogadores perceberiam. Mods de sessão só rodam num lobby em que todos os outros jogadores concordaram com eles ([Jogar online](wiki:multiplayer)). Jogando sozinho, eles sempre rodam.

Um mod de sessão aplicado vale para todas as missões: os resultados da Guerra Galáctica e dos placares não são enviados, Proving Grounds fica fechado e o matchmaking só encontra outros jogadores do Obsidian ([A Guerra Galáctica](wiki:galactic-war)).

## Expansões {#expansions}

Uma expansão amplia o jogo principal e deixa o jogo original como está. Ela adiciona conteúdo separado do jogo normal, como um modo arcade aberto por uma tela de missão própria na nave, e não muda nada fora desse conteúdo.

Fora do conteúdo, uma expansão conta como mod de cliente. O matchmaking continua normal, os resultados da Guerra Galáctica são enviados e Proving Grounds continua aberto. As telas e as páginas de menu que levam ao conteúdo aparecem em qualquer lobby, e nada mais do que a expansão faz roda.

O host inicia o conteúdo. Antes, o Obsidian confere se todos os jogadores do lobby rodam a mesma expansão, na mesma versão e com os mesmos valores de opção. Se alguém não roda, o início é recusado e o host fica sabendo quem. Caso contrário, todos os jogos do lobby seguem o host para dentro do conteúdo e, enquanto ele roda:

- a expansão pode mudar o jogo compartilhado, como um mod de sessão;
- os resultados da Guerra Galáctica e dos placares daquela missão não são enviados, e Proving Grounds fica fechado;
- o lobby passa para um grupo de matchmaking daquela expansão, onde os jogadores que a rodam na mesma versão encontram o lobby;
- um jogador que tenta entrar sem a expansão é recusado antes de chegar à missão.

Quando o conteúdo termina, ou quando o jogador sai do lobby, a expansão volta a ser um mod de cliente.

## Mods não especificados {#unspecified-mods}

Um mod que não declara escopo fica como não especificado, e o mesmo vale para mods antigos importados de bundles inteiros, porque o Obsidian não tem como saber o que eles mudam. Mods não especificados rodam como mods de cliente, em qualquer lobby: ficar de fora do multijogador por um palpite seria pior. Se um deles mudar o jogo compartilhado, os outros jogadores podem perceber, então o melhor é jogar com quem também o tem.

Um mod que pede para rodar só enquanto todos os jogadores o rodam (`every_player`) é um mod de sessão, com ou sem escopo.

## Bibliotecas {#libraries}

Uma biblioteca é código usado por outros mods, como o Obsidian API. Ela assume o escopo dos mods que a usam: é um mod de sessão enquanto um mod de sessão ligado precisar dela e um mod de cliente no resto do tempo. Uma biblioteca que nenhum mod usa, ou que só mods de cliente e expansões usam, não tira o jogador da busca de partidas normal nem bloqueia os resultados dele na Guerra Galáctica. Enquanto o conteúdo roda, uma expansão usa por completo as bibliotecas de que precisa.

## No aplicativo {#in-the-app}

Cada linha da lista de mods tem um losango para o escopo: preto para não especificado, verde para cliente, vermelho para sessão, branco para biblioteca e amarelo para expansão. Passar o mouse por cima mostra o nome. Os detalhes de cada mod também mostram o escopo.

## Quem decide o escopo {#who-decides-the-scope}

O autor do mod declara o escopo no arquivo `obsidian.json`. O Obsidian então faz valer: os hooks de um mod de sessão se desligam sozinhos quando o lobby não está liberado, e os hooks de uma expansão ficam desligados até o conteúdo começar, então o autor não precisa escrever nada disso ([Escrever mods de sessão](wiki:session-mods)).

> [!WARNING]
> O escopo é uma promessa do autor. Um mod que se diz de cliente mas muda o jogo compartilhado quebra as regras, e o Obsidian nem sempre consegue perceber. [O que as proteções não fazem](wiki:honest-limits) fala disso.
