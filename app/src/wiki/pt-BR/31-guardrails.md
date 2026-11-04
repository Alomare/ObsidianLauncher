---
title: Proteções
group: Jogar online
summary: O escudo no topo: mods de sessão de outros jogadores, conquistas e a progressão da sua conta.
---

**Proteções**, o escudo no topo da janela, reúne três interruptores do perfil ativo. Eles valem a partir da próxima vez que você abrir o jogo, e para tudo o que roda nele: nenhum mod consegue mudá-los nem passar por cima deles.

## Mods de sessão de outros jogadores {#session-mods-of-other-players}

**Bloquear mods de sessão dos outros no meu lobby público** mantém os mods de sessão dos outros jogadores fora do seu jogo enquanto você estiver num lobby público com eles. Ele vem desligado, então jogar com outros jogadores do Obsidian simplesmente funciona. Lobbies só para amigos e privados nunca são bloqueados. Ele não decide se os seus próprios mods de sessão rodam ([Jogar online](wiki:multiplayer)).

## Conquistas {#achievements}

**Impedir que conquistas sejam desbloqueadas** barra todas as conquistas da Steam enquanto você joga. O progresso que o jogo conta para uma também não é guardado, então nada aparece depois numa partida normal.

## Progressão congelada {#frozen-progression}

**Congelar a progressão da minha conta** mantém seu XP e patente, amostras, desbloqueios, melhorias e estatísticas como estavam quando o jogo abriu. Missões, a Armaria e mods ainda podem mudá-los enquanto você joga, mas nada disso é salvo, e a próxima abertura começa com a mesma conta. Configurações e visual, como suas teclas ou seu capacete, continuam sendo salvos.

Serve para testar mods, o trainer ou um modo de jogo novo sem que conte.

> [!NOTE]
> Comprar algo na Armaria com isso ligado parece funcionar, e é desfeito no próximo salvamento.

## No aplicativo {#in-the-app}

O escudo fica preenchido, com um número, enquanto algum dos interruptores estiver ligado. Passe o mouse por cima para ver os três. Na lista de perfis, um escudo marca o perfil com um dos dois últimos ligado, e o ícone de jogadores marca o que bloqueia os mods de sessão dos outros.

Um perfil que você compartilha ou importa mantém os dois últimos, e o Obsidian avisa quando um perfil importado vem com eles ligados. **Bloquear mods de sessão dos outros no meu lobby público** fica desligado num perfil importado, para cada jogador escolher.

## Sempre ligado {#always-on}

Resultados da Guerra Galáctica e placares nunca são enviados enquanto houver mods de sessão aplicados, nem de uma missão em que o conteúdo de uma expansão rodou, e Proving Grounds fica fechado nesses casos ([A Guerra Galáctica](wiki:galactic-war)). Isso protege os outros jogadores, então não é uma escolha. [O que as proteções não fazem](wiki:honest-limits) explica os limites de tudo isso.

Autores de mods podem ler as escolhas do jogador com `obsidian.guardrails()` ([API do loader](wiki:loader-api#obsidian-guardrails)).
