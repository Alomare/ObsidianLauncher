---
title: Obsidian Trainer
group: Primeiros passos
summary: O menu dentro do jogo que muda quase tudo.
---

O Obsidian Trainer é um menu dentro do jogo para o personagem, armas, estratagemas, inimigos, missões e dados do jogo. Ele vem com o Obsidian e começa desligado.

## Ligar {#turning-it-on}

Ligue o **Obsidian Trainer** na lista de Mods e aplique. O Obsidian API, o mod em que o Trainer se baseia, liga junto. No jogo, **F6** abre o menu, e as páginas do Trainer ficam na pasta **Obsidian Trainer** à esquerda. A janela e a tecla vêm do Obsidian API, então páginas de outros mods também aparecem ali, cada mod numa pasta, e a aba Teclas do Obsidian API muda a tecla.

| Tecla | O que faz |
|---|---|
| Setas, ou W A S D | Move, e muda um valor |
| Enter, ou E | Seleciona, ou digita um número |
| Shift com esquerda ou direita | Muda um número de dez em dez |
| Tab | Filtra a lista |
| R | Restaura o valor selecionado |
| Esc, ou Q | Volta, e fecha |

O rodapé do menu mostra essas teclas como teclas desenhadas. Uma linha em edição mostra o texto no lugar, e uma linha de código de estratagema aceita as setas ou W A S D, com Backspace para apagar um passo. O menu também funciona com o mouse, com o cursor normal do sistema:

| Mouse | O que faz |
|---|---|
| Apontar para uma linha | Seleciona a linha |
| Clicar numa linha | Usa a linha |
| Clicar em `<` ou `>` | Muda um valor, e segurar continua mudando |
| Clicar num número | Digita um valor |
| Clicar na seta no fim de uma linha | Restaura o valor, depois que ele mudou |
| Roda, ou a barra lateral | Rola a lista |
| Botão direito | Volta |
| Barra de título | Arrastar move a janela |
| Canto inferior direito | Arrastar redimensiona a janela |

A janela abre onde foi deixada, com o mesmo tamanho. Na lista, Enter ou um clique abre e fecha uma pasta, e cada pasta fica como foi deixada.

O menu trava o personagem enquanto está aberto e fecha sozinho quando o jogo troca de tela, como no começo de uma missão. Um controle pode navegar pelo menu, mas só F6 o abre.

## O que tem nele {#what-is-in-it}

- **Jogador:** modo deus, corrida infinita, velocidade, teletransporte, renascimento, e capacetes, armaduras e capas possuídos.
- **Armas:** munição infinita, recarga instantânea, reabastecimento, cadência, dano, carregador, recarga, dispersão e perfuração de blindagem de cada arma que o jogador segura, e qualquer arma, mochila ou vantagem do jogo base ou de um DLC possuído, com as melhorias escolhidas.
- **Estratagemas:** sem recarga, usos ilimitados, chamadas instantâneas, ignorar zonas sem estratagemas, qualquer estratagema do jogo base ou de um DLC possuído lançado no cursor, e sinalizadores de verdade, Reforço incluído. **Novo estratagema** cria um estratagema a partir de um do jogo: nome, código, ícone, recarga, usos e tempo de chamada, e o que acontece quando ele cai. Os outros jogadores veem o estratagema de origem, e ele volta no próximo início do jogo.
- **Veículos:** velocidade, curva, blindagem, sem dano e foguetes infinitos do exotraje para o veículo que está sendo dirigido, além de reparos e reabastecimento.
- **Bots:** Helldivers controlados por esta máquina, com um espaço no HUD ou como corpos simples além de quatro jogadores. Ordens para todos os bots ou para cada um: seguir o jogador, esperar, andar até o cursor, vir até o jogador, lutar, olhar para o cursor, atirar, arremessar, recarregar, digitar um código de estratagema como um jogador e lançar o sinalizador no cursor, e chamar estratagemas, além de armas, mochilas, vantagens e estratagemas para dar a eles. Só para o anfitrião, numa missão.
- **Criar:** munição, caixas de suprimento e itens no local, e sinalizadores no cursor.
- **Inimigos:** parar o surgimento, congelar, cegar ou atordoar inimigos, matá-los, criar mais e ajustar o quanto os encontros e patrulhas pressionam.
- **Missão:** a lista de objetivos, concluir objetivos, proteger comboios e escoltas, extração rápida, pular esperas, modificadores do Proving Grounds em qualquer missão, os jogadores do lobby, amostras de pesquisa e missões personalizadas de qualquer tipo a partir da nave, que pagam XP e amostras e não mexem na campanha.
- **Mundo:** velocidade do jogo, câmera, HUD, névoa, hora do dia e gravidade, e o som: volumes, e uma página **Trilha sonora** que toca qualquer faixa de música que a tela tem, silencia ou pausa a faixa e a devolve ao jogo.
- **Editor:** os números de cada arma, granada, mochila, estratagema, sentinela, mina, ataque, veículo, inimigo e efeito de status.
- **Console:** os comandos de desenvolvedor.
- **Alterações:** tudo o que foi ligado ou editado, com um jeito de desfazer tudo de uma vez.

Cada opção tem uma descrição no rodapé do menu enquanto está selecionada. O surgimento para quando o mapa tem 64 inimigos, cadáveres incluídos, porque muito mais que isso pode travar o jogo. Ações que só o anfitrião pode fazer avisam quando escolhidas.

## O que ele não faz {#what-it-doesn-t-do}

- **Nenhuma edição de progressão.** Nenhuma página do Trainer escreve no save. O que uma missão com ele ligado rende conta, como em qualquer outra. As [Proteções](wiki:guardrails) podem impedir que um início do jogo conte.
- **Nenhum DLC que o jogador não possui.** As listas mostram todo o jogo base, o que foi desbloqueado jogando e o DLC possuído. Os itens duram só aquela missão, então nada é adicionado ao save.
- **Nenhuma mudança na guerra compartilhada.** Comandos que reportam à Guerra Galáctica são recusados, e os resultados continuam bloqueados ([A Guerra Galáctica](wiki:galactic-war)).

> [!WARNING]
> O jogo tem uma verificação antiadulteração que corta o jogador da rede até o jogo reiniciar. O Obsidian Trainer muda valores protegidos só do jeito que o próprio jogo faz, então não deve dispará-la. Outras ferramentas e scripts feitos à mão podem.

## Jogar com outros {#playing-with-others}

O Trainer é um mod de sessão, então num lobby público todos precisam rodar os mesmos mods ou permitir mods de sessão ([Jogar online](wiki:multiplayer)). Quando entra alguém que não faz isso, toda mudança é desfeita e as páginas do Trainer fecham até esse jogador sair, e então as mudanças voltam. Num lobby só para amigos ou privado, o Trainer sempre roda. Os números do Editor e as duas linhas de dano rodam no jogo de cada jogador, então funcionam melhor quando todos usam os mesmos.

Duas coisas exigem um lobby com um só jogador: dar a mira laser, porque os jogos dos outros jogadores a rodariam sem preparação, e `spawn_enemy` no console, porque carrega um pacote num jogo só.

## Manter as mudanças {#keeping-changes}

As mudanças duram até o jogo fechar. Em **Alterações**, **Manter edições para os próximos inícios** salva as edições do Editor, e elas voltam a cada início do jogo enquanto a opção **Manter edições salvas** do mod estiver ligada.
