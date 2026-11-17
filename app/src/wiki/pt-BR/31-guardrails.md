---
title: Proteções
group: Jogar online
summary: O escudo no topo: mods de sessão de outros jogadores, conquistas, progressão da conta e a lista de lobbies.
---

As **Proteções**, o botão de escudo na barra no topo da janela, têm quatro interruptores para o perfil ativo. Eles valem a partir do próximo início do jogo, e para tudo o que roda nele: nenhum mod pode mudá-los ou contorná-los. Os quatro começam desligados.

## Mods de sessão de outros jogadores {#session-mods-of-other-players}

**Bloquear mods de sessão de outros jogadores em lobbies públicos** mantém os mods de sessão de outros jogadores fora do jogo enquanto se está num lobby público com eles. O interruptor começa desligado, então jogar com outros jogadores do Obsidian simplesmente funciona, e lobbies só para amigos e privados nunca são bloqueados. Ele não decide se os mods de sessão do jogador que o ligou rodam ([Jogar online](wiki:multiplayer)).

## Conquistas {#achievements}

**Impedir que conquistas sejam desbloqueadas** bloqueia todas as conquistas da Steam durante o jogo. O progresso que o jogo conta para uma conquista também não fica guardado, então nada aparece depois num jogo normal.

## Progressão congelada {#frozen-progression}

**Congelar a progressão da conta** mantém XP e patente, amostras, desbloqueios, melhorias e estatísticas como estavam quando o jogo abriu. Missões, o Arsenal e mods ainda podem mudá-los durante o jogo, mas nada disso é salvo, e o próximo início do jogo parte da mesma conta. Configurações e aparência, como teclas ou o capacete, continuam sendo salvas. O interruptor serve para testar mods, o Trainer ou um novo modo de jogo sem que nada conte.

> [!NOTE]
> Comprar algo no Arsenal com isto ligado parece funcionar, e é desfeito no próximo salvamento.

## A lista de lobbies {#the-lobby-list}

Um lobby hospedado com o Obsidian lista os mods que roda no terminal de multijogador do jogo ([A aba Com mods](wiki:multiplayer#the-modded-tab)). Mods de sessão e expansões sempre aparecem, porque mudam o jogo dos outros jogadores. **Esconder mods de cliente da lista de lobbies** deixa os mods de cliente fora dessa lista.

## No app {#in-the-app}

O escudo fica preenchido enquanto **Bloquear mods de sessão de outros jogadores em lobbies públicos**, **Impedir que conquistas sejam desbloqueadas** ou **Congelar a progressão da conta** está ligado, e passar o mouse sobre ele mostra o estado desses três. Na lista de perfis, um escudo marca um perfil com conquistas ou progressão protegidas, e o ícone de jogadores marca um que bloqueia mods de sessão de outros jogadores.

Um perfil compartilhado ou importado mantém os interruptores de conquistas e de progressão, e o Obsidian avisa quando um perfil importado os tem ligados. **Bloquear mods de sessão de outros jogadores em lobbies públicos** fica desligado num perfil importado, para cada jogador escolher.

## Sempre ligado {#always-on}

Resultados da Guerra Galáctica e placares nunca são enviados enquanto há mods de sessão aplicados, nem de uma missão em que o conteúdo de uma expansão rodou, e o Proving Grounds fica fechado nesses casos ([A Guerra Galáctica](wiki:galactic-war)). Isso protege os outros jogadores, então não é uma escolha. [O que as proteções não fazem](wiki:honest-limits) cobre os limites de tudo isso.

Autores de mods podem ler os três primeiros interruptores com `obsidian.guardrails()` ([Loader API](wiki:loader-api#obsidian-guardrails)).
