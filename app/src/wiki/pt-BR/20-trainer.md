---
title: Treinador Obsidian (Trainer)
group: Primeiros passos
summary: O menu dentro do jogo que muda quase tudo.
---

O Obsidian Trainer é um menu dentro do jogo para personagem, armas, estratagemas, inimigos, missões e dados do jogo. Ele vem com o Obsidian e começa desligado.

## Como ligar {#turning-it-on}

Ligue o **Obsidian Trainer** na lista de mods e aplique. O Obsidian API, o mod em que ele é construído, liga junto. No jogo, aperte **F6** para abrir o menu. A aba Teclas do mod troca a tecla.

| Tecla | O que faz |
|---|---|
| Setas, ou W A S D | Mover e mudar um valor |
| Enter, ou E | Escolher, ou digitar um número |
| Shift com esquerda ou direita | Mudar um número dez passos por vez |
| Tab | Filtrar a lista |
| R | Redefinir o valor selecionado |
| Esc, ou Q | Voltar e fechar |

O menu também funciona com o mouse, usando o cursor normal do sistema.

| Mouse | O que faz |
|---|---|
| Apontar para uma linha | Selecionar |
| Clicar em uma linha | Usar |
| Clicar em `<` ou `>` | Mudar um valor; segure para continuar mudando |
| Clicar em um número | Digitar um valor |
| Clicar na seta no fim de uma linha | Redefinir o valor, depois de alterado |
| Roda, ou a barra ao lado | Rolar a lista |
| Botão direito | Voltar |
| Barra de título | Arraste para mover a janela |
| Canto inferior direito | Arraste para mudar o tamanho da janela |

A janela abre onde você a deixou, no mesmo tamanho.

O menu trava o personagem enquanto está aberto, e fecha sozinho quando o jogo muda de tela, como no início de uma missão. Um controle navega pelo menu, mas quem abre é o F6.

## O que tem nele {#whats-in-it}

- **Jogador:** modo deus, corrida infinita, velocidade, teleporte, reaparecer, e capacetes, armaduras e capas que você tem.
- **Armas:** munição infinita, recarga instantânea, reabastecimento, e qualquer arma, mochila ou vantagem do jogo base ou de uma DLC que você tem, com as melhorias que escolher.
- **Estratagemas:** sem tempo de recarga, usos ilimitados, chamada instantânea, ignorar zonas sem estratagemas, qualquer estratagema do jogo base ou de uma DLC que você tem lançado no cursor, e sinalizadores reais, inclusive Reforço.
- **Veículos:** velocidade, giro, armadura, sem dano e foguetes de exotraje infinitos no veículo que você dirige, mais reparo e reabastecimento.
- **Criar:** munição, caixas de suprimentos e itens no chão na hora, e sinalizadores no cursor.
- **Inimigos:** parar o surgimento, congelar, cegar ou atordoar, matar, criar mais e ajustar a pressão de encontros e patrulhas.
- **Missão:** a lista de objetivos, concluir objetivos, proteger comboios e escoltas, extração rápida, pular esperas, modificadores do campo de provas em qualquer missão, quem está no lobby, amostras de pesquisa e missões de treino de qualquer tipo a partir da nave.
- **Mundo:** velocidade do jogo, câmera, HUD, neblina, hora do dia e gravidade.
- **Editor:** os números de cada arma, granada, mochila, estratagema, sentinela, mina, ataque, veículo, inimigo e efeito de status.
- **Console:** os comandos de desenvolvedor.
- **Alterações:** tudo que está ligado ou editado, com um jeito de desfazer tudo de uma vez.

O surgimento para quando o mapa tem 64 inimigos, contando cadáveres, porque muito mais que isso pode travar o jogo.

Ações que só o host pode fazer avisam quando você as escolhe.

## O que ele não faz {#what-it-wont-do}

- **Não edita sua progressão.** Nenhuma das páginas dele escreve no seu save. O que você ganha numa missão com ele ligado é seu, como em qualquer outra. Para jogar sem que conte, use as [Proteções](wiki:guardrails).
- **Não dá DLC que você não tem.** As listas mostram todo o jogo base, o que você desbloqueou jogando, mais as DLCs que você tem. Os itens só chegam a você durante aquela missão, então nada é adicionado ao seu save.
- **Não mexe na guerra compartilhada.** Comandos que informam a Guerra Galáctica são recusados, e os resultados continuam bloqueados ([A Guerra Galáctica](wiki:galactic-war)).

> [!WARNING]
> O jogo tem uma verificação anti-adulteração que corta sua conexão até reiniciar. O Obsidian Trainer só muda valores protegidos do jeito que o próprio jogo muda, então ela não deve disparar. Outras ferramentas e scripts feitos à mão podem disparar.

## Jogar com outras pessoas {#playing-with-others}

O trainer é um mod de sessão, então todos no lobby precisam usar os mesmos mods ou permitir mods de sessão ([Jogar online](wiki:multiplayer)). Se entrar alguém que não permite, cada mudança é desfeita e o menu para até a pessoa sair. Depois suas mudanças voltam.

Os números do Editor e as duas linhas de dano rodam no jogo de cada jogador, então funcionam melhor quando todos usam os mesmos.

Duas coisas exigem que você esteja sozinho no lobby: dar a si mesmo a mira laser, porque o jogo dos outros a rodaria sem preparo, e o `spawn_enemy` do console, porque ele carrega um pacote só no seu jogo.

## Manter mudanças {#keeping-changes}

As mudanças duram até fechar o jogo. Em **Alterações**, **Manter estas edições na próxima vez** salva suas edições do Editor, e elas voltam a cada início enquanto a opção **Manter edições salvas** do mod estiver ligada.
