---
title: Escopos
group: Jogar online
summary: Mods de cliente, de sessão, expansões, bibliotecas e não especificados, Escopos Híbridos e o que cada um pode fazer.
---

HELLDIVERS é um jogo cooperativo. Um mod que muda inimigos, armas ou missões muda o jogo de todos no lobby, não só de quem o instalou. O Obsidian põe cada mod num escopo, para que um jogador que nunca instalou um mod nunca seja afetado por ele.

## Mods de cliente {#client-mods}

Um mod de cliente muda só o que um jogador vê, ouve ou aperta: a câmera, o HUD, os controles, os menus, os visuais locais e o som. Nada do que ele faz chega aos outros jogadores, então mods de cliente rodam em qualquer lugar, em qualquer lobby, com qualquer pessoa. O Obsidian Patches é um mod de cliente.

## Mods de sessão {#session-mods}

Um mod de sessão pode mudar o jogo compartilhado: equilíbrio, inimigos, surgimentos, missões, física, qualquer coisa que outros jogadores notariam. Num lobby público, mods de sessão só rodam quando todos os outros jogadores os permitem. Sozinho, e num lobby só para amigos ou privado, eles sempre rodam ([Jogar online](wiki:multiplayer)).

Um mod de sessão aplicado afeta toda missão, rodando ou desligado: resultados da Guerra Galáctica e dos placares não são enviados, e o Proving Grounds fica fechado ([A Guerra Galáctica](wiki:galactic-war)). O matchmaking continua igual ao dos jogadores vanilla ([Matchmaking](wiki:multiplayer#matchmaking)).

## Expansões {#expansions}

Uma expansão estende o jogo principal e não afeta o vanilla. Ela adiciona conteúdo separado do jogo normal, como um modo arcade acessado por uma tela de missão própria na nave, e não muda nada fora desse conteúdo.

Fora do conteúdo, uma expansão conta como mod de cliente. O matchmaking continua normal, os resultados da Guerra Galáctica são enviados e o Proving Grounds continua aberto. As telas e páginas de menu que levam ao conteúdo aparecem em qualquer lobby, e nada mais da expansão roda.

O anfitrião inicia o conteúdo. O Obsidian primeiro confere se todos os jogadores do lobby rodam a mesma expansão, na mesma versão e com os mesmos valores de opção. Se alguém não roda, o início é recusado e o anfitrião fica sabendo quem. Senão, todos os jogos do lobby seguem o anfitrião para o conteúdo, e enquanto ele roda:

- a expansão pode mudar o jogo compartilhado, como um mod de sessão;
- os resultados daquela missão para a Guerra Galáctica e os placares não são enviados, e o Proving Grounds fica fechado;
- o lobby passa para um grupo de matchmaking daquela expansão, onde jogadores que a rodam na mesma versão encontram o lobby;
- um jogador que tenta entrar sem a expansão é barrado antes de chegar à missão.

Quando o conteúdo termina, ou quando o jogador sai do lobby, a expansão volta a ser um mod de cliente.

## Bibliotecas {#libraries}

Uma biblioteca é código em que outros mods se apoiam, como o Obsidian API. Ela assume o escopo dos mods que a usam: mod de sessão enquanto um mod de sessão ou híbrido ligado precisa dela, e mod de cliente no resto do tempo. Uma biblioteca que nenhum mod usa, ou que só mods de cliente e expansões usam, mantém o grupo de matchmaking normal e os resultados da Guerra Galáctica. Uma expansão tem uso completo das bibliotecas de que precisa enquanto o conteúdo roda.

## Escopos Híbridos {#hybrid-scopes}

Um mod pode rodar com mais de um escopo. O escopo que o autor declara em `scope` é o escopo principal: um Obsidian mais antigo lê só esse e roda o mod inteiro com ele. Escopos secundários guardam o código, as opções e os arquivos do mod que se comportam de forma diferente do escopo principal.

Um mod de sessão com um escopo secundário de cliente mantém esse código rodando em qualquer lobby. Um mod de câmera, por exemplo, tem sessão como escopo principal, deixa a câmera num escopo secundário de cliente e põe um alcance maior das armas num escopo secundário de sessão. Num lobby não liberado, os escopos secundários de sessão desligam, os de cliente continuam rodando, e o jogo diz quais desligaram.

Uma expansão com um escopo secundário de sessão também roda esse código fora do conteúdo, num lobby liberado, e o desliga num lobby não liberado. Dentro do conteúdo da expansão, tudo roda como antes.

Um escopo secundário pode ter uma opção nos detalhes do mod que o liga e desliga. Enquanto um escopo secundário de sessão está ligado, o mod conta como mod de sessão para a aplicação: os resultados da Guerra Galáctica não são enviados e o Proving Grounds fica fechado. Com todos os escopos secundários de sessão desligados, um mod de sessão é aplicado como mod de cliente e uma expansão como expansão, e a Guerra Galáctica e o Proving Grounds voltam depois da próxima aplicação.

Um mod de sessão com um escopo secundário de cliente e um de sessão ligados é **híbrido**.

## Mods não especificados {#unspecified-mods}

Um mod que não declara escopo é não especificado, assim como um mod antigo importado de bundles inteiros, porque o Obsidian não sabe o que ele muda. Mods não especificados rodam como mods de cliente em qualquer lobby, já que barrar um jogador do multijogador por palpite seria pior. Quando um deles muda o jogo compartilhado, os outros jogadores podem notar, então jogue com pessoas que também o têm.

Um mod que pede para rodar só enquanto todos os jogadores o rodam (`every_player`) é um mod de sessão, qualquer que seja o escopo declarado.

## No app {#in-the-app}

Cada linha da lista de mods tem um losango para o escopo: preto para não especificado, verde para cliente, vermelho para sessão, branco para biblioteca (cinza no tema claro) e amarelo para expansão. Um mod híbrido tem um losango dividido em dois, verde à esquerda e vermelho à direita. Passar o mouse sobre ele mostra o nome do escopo e o que ele significa, com uma etiqueta WIKI: um clique abre esta página naquele escopo. Os detalhes de cada mod também citam o escopo, e a aba Sobre termina com a lista de escopos de um mod que tem mais de um, marcando o que estiver desligado.

## Quem decide o escopo {#who-decides-the-scope}

O autor do mod o declara no arquivo `obsidian.json`, e o Obsidian o impõe. Os ganchos de um mod de sessão desligam sozinhos quando o lobby não está liberado, e os ganchos de uma expansão ficam desligados até o conteúdo começar, então o autor não escreve nada disso ([Escrever mods de sessão](wiki:session-mods)).

> [!WARNING]
> Um escopo é uma promessa do autor. Um mod que se diz de cliente mas muda o jogo compartilhado quebra as regras, e o Obsidian nem sempre consegue perceber. [O que as proteções não fazem](wiki:honest-limits) trata disso.
