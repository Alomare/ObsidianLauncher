---
title: Regras da guerra
group: Guerras
summary: Como frentes, pressão inimiga, defesas e ataques a mundos natais avançam, e cada configuração com o padrão.
---

## Frentes e regiões {#fronts-and-regions}

Uma guerra Vanilla tem três frentes, uma por inimigo. Cada frente tem pontos de 0 a **Points per Front**, e dez regiões os dividem igualmente: a região 1 encosta na Super Terra e a região 10 encosta no mundo natal do inimigo. Uma temporada começa cada frente ativa em **Start Points**, na região 2 por padrão.

Um planeta concluído no jogo soma **Planet Value** vezes a dificuldade e o sucesso da missão à frente daquele planeta. Enquanto nenhum evento roda numa frente, **Enemy Pressure** tira pontos a cada hora de guerra.

## Defesas {#defenses}

Com o **War Director** ligado, cada hora de guerra tem uma **Defense Chance** de iniciar uma defesa numa frente ativa aleatória, na região atual daquela frente. Roda uma defesa por vez. Cada missão de defesa vencida soma **Defense Mission Value** vezes a dificuldade, e alcançar **Defense Goal** antes de **Defense Duration** acabar vence a defesa e soma **Defense Reward** à frente. Uma defesa fracassada custa uma região à frente.

Uma frente que cai a 0 pontos inicia uma defesa da Super Terra. Segurá-la põe a frente de volta no começo da região 2, e perdê-la perde a temporada.

## Ataques a mundos natais {#home-world-attacks}

Uma frente que chega ao último ponto inicia um ataque ao mundo natal do inimigo. Cada missão de ataque vencida soma **Attack Mission Value** vezes a dificuldade, e alcançar **Attack Goal** antes de **Attack Duration** acabar derrota aquele inimigo. Um ataque fracassado leva a frente de volta a 80% de **Points per Front**, na região 9.

## Configurações {#settings}

As três predefinições do formulário **Create War**, aberto com **Nova guerra**, definem estes números. **War Settings**, na tela da guerra, muda qualquer um deles enquanto a guerra roda, e uma mudança vale dali em diante. Com o War Director desligado, só o Mestre de Jogo inicia defesas, enquanto as frentes continuam se movendo com as missões e a pressão inimiga.

| Configuração | Standard | Short Campaign | Hard |
|---|---|---|---|
| Points per Front | 100.000 | 30.000 | 100.000 |
| Start Points | 15.000 | 3.000 | 15.000 |
| Enemy Pressure, por hora | 600 | 250 | 1.200 |
| Planet Value | 250 | 250 | 200 |
| War Director | Ligado | Ligado | Ligado |
| Defense Chance, por hora | 8% | 12% | 16% |
| Defense Duration | 4 h | 2 h | 3 h |
| Defense Goal | 3.000 | 1.500 | 4.500 |
| Defense Mission Value | 150 | 150 | 120 |
| Defense Reward | 3.000 | 1.500 | 2.000 |
| Attack Duration | 6 h | 3 h | 4 h |
| Attack Goal | 5.000 | 2.500 | 8.000 |
| Attack Mission Value | 150 | 150 | 120 |
| Own Save Only | Desligado | Desligado | Desligado |
| Allow Achievements | Desligado | Desligado | Desligado |

O grupo **Players** tem os dois últimos. **Own Save Only** restringe a guerra ao save Sandbox que o Obsidian cria para ela; desligado, qualquer save Sandbox a roda. **Allow Achievements** deixa as conquistas da Steam serem desbloqueadas enquanto a guerra é jogada, exceto para um jogador que bloqueia conquistas nas [Proteções](wiki:guardrails).
