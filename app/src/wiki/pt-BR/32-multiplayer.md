---
title: Jogar online
group: Jogar online
summary: Matchmaking, lobbies públicos e quando os mods de sessão rodam num lobby.
---

## Lobbies públicos {#the-multiplayer-switch}

O Obsidian só segura os mods de sessão em lobbies públicos, onde você pode cair com desconhecidos. Num lobby só para amigos ou privado, todo mundo escolheu com quem jogar, então os mods de sessão rodam ali com qualquer um, inclusive com jogadores sem mods.

**Bloquear mods de sessão dos outros no meu lobby público** é um dos interruptores de [Proteções](wiki:guardrails), no topo da janela. Ele vem desligado, pertence ao perfil ativo e vale a partir da próxima abertura do jogo. Ligado, os mods de sessão dos outros jogadores desligam enquanto você estiver num lobby público com eles. Ele não decide se os seus próprios mods de sessão rodam.

## Matchmaking {#matchmaking}

Mods de sessão não mudam quem você encontra nem quem pode entrar no seu lobby. Jogadores sem mods veem o seu lobby e podem entrar, e você pode entrar no deles. Num lobby público, os seus mods de sessão desligam enquanto houver um jogador sem mods (veja abaixo).

| A sua configuração | Quem você encontra no matchmaking |
|---|---|
| Sem mods de sessão, ou com mods de sessão que desligam na hora | Todo mundo. O seu lobby aparece para todo mundo. Expansões não mudam isso |
| Mods de sessão que não desligam na hora | Todo mundo. Enquanto o seu lobby for público, só jogadores do Obsidian podem entrar, e você só entra em outro lobby público se o anfitrião também rodar conteúdo assim |
| Hospedando o conteúdo de uma expansão | O lobby só aparece para jogadores que rodam essa expansão, na mesma versão |

Com uma expansão aplicada, a lista também mostra os lobbies de jogadores que estão no conteúdo dessa mesma expansão, qualquer que seja o grupo de matchmaking do jogador.

Na primeira vez que você carrega a nave, uma mensagem diz qual caso vale para você: os seus mods não afetam outros jogadores, os seus mods de sessão desligam em alguns lobbies públicos, parte do seu conteúdo deixa jogadores sem Obsidian fora do seu lobby público, ou uma expansão pode separar você de jogadores que não a têm enquanto o conteúdo dela roda.

## Quando os mods de sessão rodam {#when-session-mods-run}

Sozinho, e num lobby só para amigos ou privado, os seus mods de sessão sempre rodam. Num lobby público, eles só rodam quando cada um dos outros jogadores:

1. roda o mesmo conjunto de mods de sessão, nas mesmas versões e com os mesmos valores de opção, ou
2. usa o Obsidian e não bloqueia os mods de sessão dos outros.

Mods de cliente sempre rodam.

Alguns exemplos, do ponto de vista do jogador A, que tem mods de sessão, num lobby público:

| Outros jogadores | Mods de sessão de A |
|---|---|
| B roda o mesmo conjunto | Ligados |
| B roda outros mods de sessão, sem bloquear | Ligados |
| B roda outros mods de sessão, bloqueando | Desligados |
| B roda só mods de cliente, sem bloquear | Ligados |
| B com HELLDIVERS original | Desligados |
| B sem bloquear, C bloqueando e com outro conjunto | Desligados: todos precisam permitir |

Num lobby só para amigos ou privado, todas as linhas ficam Ligados.

A verificação roda de novo sempre que alguém entra, sai ou muda a configuração, e quando o host muda quem pode entrar. Os mods ligam e desligam na hora, e um mod de sessão pode desfazer o que mudou quando isso acontece.

## Mods que todos os jogadores precisam ter {#mods-every-player-needs}

Alguns mods só funcionam quando todo mundo os tem, como um que deixa mais jogadores entrarem no lobby. O autor marca esses mods, e eles seguem uma regra mais rígida: o mod só roda quando todos os outros jogadores também o rodam, na mesma versão e com os mesmos valores de opção, em qualquer lobby, inclusive só para amigos. Não bloquear não conta para eles. Quando alguém sem o mod está no seu lobby, só esse mod desliga, e o jogo avisa. Os seus outros mods de sessão seguem as regras de sempre.

> [!TIP]
> Valores de opção que o autor marca como pessoais, como a distância da câmera, não contam quando o Obsidian compara os conjuntos. Um amigo pode escolher outra distância de câmera e ainda combinar com você.

## Expansões {#expansions}

Uma [expansão](wiki:scopes#expansions) roda como mod de cliente até o host iniciar o conteúdo. O início confere todos os jogadores do lobby: cada um precisa rodar a mesma expansão, na mesma versão e com os mesmos valores de opção. Se alguém não roda, nada começa e o host fica sabendo quem. O interruptor de Proteções não conta aqui.

Depois que o conteúdo começa, todos os jogos do lobby seguem o host. Um jogador que tenta entrar sem a expansão é recusado, e o host vê o nome. Ninguém que entra ou sai desliga o conteúdo para quem já está nele. Quando o host para o conteúdo, ou quando o jogador sai do lobby, a expansão volta a ser um mod de cliente.

## Mods que não desligam na hora {#mods-that-can-t-switch-off-live}

Parte do conteúdo de sessão não pode ser desligada no meio do jogo: substituições de dados e mods antigos importados. Enquanto um jogador com esse conteúdo hospeda um lobby público, jogadores sem mods não conseguem entrar, e o jogo diz que a versão não bate. Esse jogador também não leva o conteúdo para um lobby público cujo anfitrião não rode conteúdo assim. Lobbies só para amigos e privados ficam abertos a todos. Se um jogador que bloqueia mods de sessão cair num lobby público assim, o loader registra um aviso no `loader.log` e os mods de sessão que podem desligar continuam desligados. Saia do lobby, ou jogue num lobby só para amigos.

## Convites {#invites}

Convites da Steam e entrar no jogo de amigos podem atravessar os grupos de matchmaking. As regras acima continuam decidindo quais mods rodam depois que você entra.
