---
title: Treinador Obsidian (Trainer)
group: Primeiros passos
summary: O menu dentro do jogo que muda quase tudo.
---

O Obsidian Trainer é um menu dentro do jogo para personagem, armas, estratagemas, inimigos, missões e dados do jogo. Ele vem com o Obsidian e começa desligado.

## Como ligar {#turning-it-on}

Ligue o **Obsidian Trainer** na lista de mods e aplique. O Obsidian API, o mod em que ele é construído, liga junto. No jogo, **F6** abre o menu, e as páginas do Trainer ficam na pasta **Obsidian Trainer**, à esquerda. A janela e a tecla pertencem ao Obsidian API, então as páginas de outros mods também aparecem ali, cada mod numa pasta separada, e a aba Teclas do Obsidian API troca a tecla.

| Tecla | O que faz |
|---|---|
| Setas, ou W A S D | Mover e mudar um valor |
| Enter, ou E | Escolher, ou digitar um número |
| Shift com esquerda ou direita | Mudar um número dez passos por vez |
| Tab | Filtrar a lista |
| R | Redefinir o valor selecionado |
| Esc, ou Q | Voltar e fechar |

O rodapé do menu mostra essas teclas como ícones de tecla. Ao digitar numa linha, o texto aparece na própria linha, e uma linha de código de estratagema aceita as setas ou W A S D, com Backspace para apagar a última direção.

O menu também funciona com o mouse, com o cursor normal do sistema.

| Mouse | O que faz |
|---|---|
| Apontar para uma linha | Selecionar |
| Clicar em uma linha | Usar |
| Clicar em `<` ou `>` | Mudar um valor; segurar para continuar mudando |
| Clicar em um número | Digitar um valor |
| Clicar na seta no fim de uma linha | Redefinir o valor, depois de alterado |
| Roda, ou a barra ao lado | Rolar a lista |
| Botão direito | Voltar |
| Barra de título | Arrastar para mover a janela |
| Canto inferior direito | Arrastar para mudar o tamanho da janela |

A janela abre onde foi deixada da última vez, no mesmo tamanho. Na lista, Enter ou um clique abre e fecha uma pasta, e cada pasta fica como foi deixada.

O menu trava o personagem enquanto está aberto, e fecha sozinho quando o jogo muda de tela, como no início de uma missão. Um controle navega pelo menu, mas quem abre é o F6.

## O que tem nele {#whats-in-it}

- **Jogador:** modo deus, corrida infinita, velocidade, teleporte, reaparecer, e os capacetes, armaduras e capas que a conta possui.
- **Armas:** munição infinita, recarga instantânea, reabastecimento, a cadência, o dano, o carregador, a recarga, a dispersão e a perfuração de blindagem de cada arma que o jogador carrega, e qualquer arma, mochila ou vantagem do jogo base ou de uma DLC da conta, com as melhorias escolhidas.
- **Estratagemas:** sem tempo de recarga, usos ilimitados, chamada instantânea, ignorar zonas sem estratagemas, qualquer estratagema do jogo base ou de uma DLC da conta lançado no cursor, e sinalizadores reais, inclusive Reforço. **Novo estratagema** cria um estratagema a partir de um do jogo: nome, código, ícone, recarga, usos e tempo de chegada, e o que acontece quando ele chega. Os outros jogadores veem o estratagema de origem, e ele volta no próximo início do jogo.
- **Veículos:** velocidade, giro, armadura, sem dano e foguetes de exotraje infinitos no veículo em uso, mais reparo e reabastecimento.
- **Bots:** Helldivers controlados por este computador, com um espaço no HUD ou, acima de quatro jogadores, como Helldivers avulsos. Ordens para todos os bots ou para cada um: seguir o jogador, manter a posição, ir até o cursor, vir até o jogador, lutar, olhar para o cursor, atirar, lançar granadas, recarregar e chamar estratagemas, além de armas, mochilas, vantagens e estratagemas para dar a eles. Só para o host, durante uma missão.
- **Criar:** munição, caixas de suprimentos e itens no chão na hora, e sinalizadores no cursor.
- **Inimigos:** parar o surgimento, congelar, cegar ou atordoar, matar, criar mais e ajustar a pressão de encontros e patrulhas.
- **Missão:** a lista de objetivos, concluir objetivos, proteger comboios e escoltas, extração rápida, pular esperas, modificadores do campo de provas em qualquer missão, os jogadores do lobby, amostras de pesquisa e missões de treino de qualquer tipo a partir da nave.
- **Mundo:** velocidade do jogo, câmera, HUD, neblina, hora do dia e gravidade, e o som: volumes e uma página **Trilha sonora** que toca qualquer faixa de música da tela atual, cala ou pausa a música e a devolve ao jogo.
- **Editor:** os números de cada arma, granada, mochila, estratagema, sentinela, mina, ataque, veículo, inimigo e efeito de status.
- **Console:** os comandos de desenvolvedor.
- **Alterações:** tudo que está ligado ou editado, com um jeito de desfazer tudo de uma vez.

Cada opção tem uma descrição no rodapé do menu enquanto está selecionada. A criação de inimigos para quando o mapa chega a 64 inimigos, contando cadáveres, porque muito mais que isso pode travar o jogo. Ações que só o host pode fazer avisam isso quando escolhidas.

## O que ele não faz {#what-it-wont-do}

- **Sem edição de progressão.** Nenhuma página do Trainer escreve no save. O que uma missão com ele ligado rende conta, como em qualquer outra. As [Proteções](wiki:guardrails) podem fazer uma abertura do jogo não contar.
- **Sem DLC que o jogador não tem.** As listas mostram todo o jogo base, o que foi desbloqueado jogando, mais as DLCs da conta. Os itens duram só aquela missão, então nada é adicionado ao save.
- **Sem mudanças na guerra compartilhada.** Comandos que informam a Guerra Galáctica são recusados, e os resultados continuam bloqueados ([A Guerra Galáctica](wiki:galactic-war)).

> [!WARNING]
> O jogo tem uma verificação anti-adulteração que desconecta o jogador da rede até o jogo reiniciar. O Obsidian Trainer só muda valores protegidos do jeito que o próprio jogo muda, então ela não deve disparar. Outras ferramentas e scripts feitos à mão podem disparar.

## Jogar com outras pessoas {#playing-with-others}

O Trainer é um mod de sessão, então todos no lobby precisam usar os mesmos mods ou permitir mods de sessão ([Jogar online](wiki:multiplayer)). Quando entra alguém que não cumpre isso, cada mudança é desfeita e as páginas do Trainer fecham até esse jogador sair. Depois as mudanças voltam.

Os números do Editor e as duas linhas de dano rodam no jogo de cada jogador, então funcionam melhor quando todos usam os mesmos.

Duas coisas exigem um lobby com um só jogador: dar a mira laser, porque o jogo dos outros jogadores a rodaria sem preparo, e o `spawn_enemy` do console, porque ele carrega um pacote em um jogo só.

## Manter mudanças {#keeping-changes}

As mudanças duram até o jogo fechar. Em **Alterações**, **Manter edições para os próximos inícios** salva as edições do Editor, e elas voltam a cada início enquanto a opção **Manter edições salvas** do mod estiver ligada.
