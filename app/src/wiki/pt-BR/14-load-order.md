---
title: Ordem de carregamento, conflitos e pastas
group: Primeiros passos
summary: Qual mod vence quando dois mudam a mesma coisa, e como manter uma lista longa organizada.
---

## Ordem de carregamento {#load-order}

O número ao lado de cada mod é o lugar dele na ordem de carregamento. Quando dois mods ligados mudam o mesmo recurso do jogo, vence o que está mais abaixo na lista. Arraste um mod para cima ou para baixo para mudar a ordem, ou selecione-o e use Alt+Cima e Alt+Baixo.

## Ordenar mods {#sort-mods}

Um mod pode pedir um lugar na ordem de carregamento: primeiro, último, antes ou depois de outros mods, ou antes ou depois de todos os mods que substituem arquivos do jogo. O botão **Ordenar mods**, logo depois do perfil no topo da aba Mods, ajusta a lista a esses pedidos, para mods ligados e desligados.

Nada se move antes de **Confirmar**. A janela Ordenar mods primeiro lista a nova ordem inteira, com cada mod movido marcado, o lugar antigo e o novo, e o motivo. Passar o mouse sobre um motivo que cita arquivos mostra esses arquivos. Depois de Confirmar, **Desfazer** na barra no pé da lista devolve a ordem antiga.

- Mods que não pedem nada mantêm a ordem, e o mínimo possível de mods se move.
- Quando duas ordens movem o mesmo número de mods, vence aquela em que os mods que substituem os mesmos arquivos perdem menos. O mod mais abaixo leva o arquivo compartilhado inteiro, então o mod com menos arquivos vai mais abaixo: ele sobrescreve uma parte menor do outro mod e não se perde por completo.
- Pedidos que não podem ser atendidos juntos, como dois mods que pedem para carregar por último, não são adivinhados. Esses mods ficam no lugar, e a janela os lista em **Não movidos** para ordenar à mão.
- Mods numa pasta trancada não se movem, e nenhum mod entra numa pasta trancada. Um mod movido que cai dentro de outra pasta entra nela, e um que sai de uma pasta sai dela.

Um mod também ocupa o lugar que pede quando entra num perfil. Depois disso a ordem é do jogador: um mod movido fica onde está até a próxima ordenação.

## Conflitos {#conflicts}

Um sinal de alerta numa linha quer dizer que o mod compartilha recursos com outro mod ligado. Passe o mouse sobre ele para ver quais mods e quantos arquivos, ou clique para abrir a lista no painel à direita. Um sinal amarelo quer dizer que este mod sobrescreve arquivos de outro mod, e um cinza quer dizer que outro mod sobrescreve alguns arquivos deste.

Um conflito muitas vezes não é problema. Dois mods podem mudar o mesmo script de propósito, e a ordem decide qual versão chega ao jogo. Quando o resultado parecer errado no jogo, tente primeiro a outra ordem.

> [!TIP]
> Os conflitos só contam os recursos que um mod aplicaria com as opções escolhidas. Uma opção desligada nunca causa conflito.

As listas que dizem ao jogo quais unidades existem são a exceção. Mods que adicionam uma arma ou unidade nova mudam essas listas, então o Obsidian as mescla, e esses mods funcionam juntos em qualquer ordem. Todos numa partida ainda precisam dos mesmos mods.

## Pastas {#folders}

Pastas agrupam mods na lista. Elas não mudam nada no jogo: a ordem de carregamento continua indo do topo da lista até o fim, passando por todas as pastas.

- O botão **Criar pasta** no topo da aba Mods cria uma, e clicar com o botão direito em mods marcados os coloca numa pasta nova.
- Arraste mods para o nome de uma pasta para colocá-los dentro, e arraste a pasta para mover juntos os mods dela.
- Clique na seta para recolher uma pasta.
- Clique com o botão direito numa pasta para renomear, mudar a cor ou apagar. Ao apagar, o Obsidian pergunta se os mods vão junto ou ficam na lista.

### Pastas trancadas {#locked-folders}

O cadeado antes do nome de uma pasta a tranca. Uma pasta trancada mantém o nome, a cor, o lugar e os mods que tem, e nada entra ou sai até ela ser destrancada.

## Marcar vários mods {#checking-several-mods}

As caixas à esquerda selecionam mods para ações em lote, e Shift+clique marca um intervalo. Com mods marcados, a barra no pé da lista os liga ou desliga, procura atualizações deles ou os remove. Clicar com o botão direito num deles oferece todo o resto, como mover todos para uma pasta.
