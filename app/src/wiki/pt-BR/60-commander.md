---
title: O que é o Commander
group: Primeiros passos
summary: Guerras Galácticas personalizadas hospedadas pelo Obsidian, e o que roda em cada PC.
---

O Commander roda Guerras Galácticas personalizadas para HELLDIVERS. Uma guerra personalizada tem frentes, regiões, defesas, ataques a mundos natais e temporadas como a campanha oficial, mas o anfitrião define cada regra e número, e um Mestre de Jogo pode mover frentes e iniciar eventos pela tela da guerra. Os jogadores lutam a guerra no jogo normalmente, e as missões que eles concluem movem a guerra personalizada em vez da oficial.

## O que roda onde {#what-runs-where}

Uma guerra vive num PC, o do anfitrião. O servidor da guerra começa pela aba [Guerras](app:commander) e roda dentro do Obsidian, então uma guerra só avança enquanto o Obsidian está aberto naquele PC. Quando o Obsidian fecha, o servidor para e o relógio da guerra congela até o próximo início.

Em todo PC de jogador, inclusive o do anfitrião, o jogo fala só com o relay do Obsidian naquele mesmo PC. O relay leva as requisições da guerra ao servidor da guerra e remove o ticket de login da Steam antes que qualquer coisa saia do PC. O Obsidian precisa ficar aberto enquanto uma guerra é jogada, já que o relay roda dentro dele.

Jogadores que entram numa guerra de outro PC precisam só do Obsidian. O pacote do Commander é para o anfitrião.

## Separada da guerra oficial {#kept-apart-from-the-official-war}

Cada guerra personalizada roda num save Sandbox e num grupo de matchmaking separado, então os lobbies só juntam jogadores da mesma guerra. A escolha ao lado de Jogar define o save: qualquer Sandbox, ou um novo criado para a guerra na primeira vez que ela é selecionada sem nenhum escolhido. Um anfitrião pode restringir uma guerra ao Sandbox criado para ela. As conquistas da Steam ficam bloqueadas num Sandbox a não ser que o anfitrião as permita para a guerra, e a proteção de conquistas do jogador continua valendo. Os resultados das missões vão para o servidor da guerra e para nenhum outro lugar. A Guerra Galáctica oficial e o save Oficial nunca veem uma guerra personalizada, e pelo mesmo motivo missões jogadas com mods de sessão contam numa guerra personalizada ([A Guerra Galáctica](wiki:galactic-war) explica por que elas não contam na oficial).

## O pacote {#the-package}

O servidor da guerra e as telas da guerra vêm no pacote do Commander, que se atualiza separado do Obsidian. **Instalar pacote**, na aba Guerras, baixa a release mais nova do GitHub. As telas da guerra estão só em inglês, qualquer que seja o idioma do Obsidian, então os nomes dos botões delas aparecem em inglês nestes guias. O Obsidian só instala um pacote baixado com uma assinatura válida da chave de assinatura do Obsidian, e recusa qualquer outro.

O botão do pacote fica na ponta direita da fileira abaixo da lista de guerras, e um ponto nele quer dizer que saiu um pacote mais novo:

| Item | O que faz |
|---|---|
| Atualizar pacote | Baixa e instala a release mais nova |
| Instalar de arquivo | Instala um ZIP de pacote do disco, como um build local. O Obsidian não confere a assinatura de um arquivo escolhido aqui, então só instale arquivos de fonte confiável |
| Remover pacote | Remove o servidor da guerra e as telas da guerra. Guerras e saves de guerra ficam no PC |
| Remover todos os dados do Commander | Remove tudo o que o Commander guarda no PC ([Remover o Commander](wiki:commander-troubleshooting#removing-commander)) |
