---
title: Ordem, conflitos e pastas
group: Primeiros passos
summary: Qual mod vence quando dois mudam a mesma coisa, e como manter uma lista longa organizada.
---

## Ordem de carregamento {#load-order}

O número ao lado de cada mod é a posição dele na ordem de carregamento. Quando dois mods ligados mudam o mesmo recurso do jogo, vence o que está mais abaixo na lista. Arraste um mod para cima ou para baixo para mudar a ordem, ou selecione e use Alt+↑ e Alt+↓.

## Ordenar mods {#sort-mods}

Um mod pode pedir um lugar na ordem de carregamento: primeiro, último, antes ou depois de outros mods, ou antes ou depois de todo mod que substitui arquivos do jogo. O botão **Ordenar mods**, logo depois do perfil no topo da aba Mods, ajusta a lista a esses pedidos, tanto para mods ligados quanto para desligados.

Nada sai do lugar até você escolher **Confirmar**. A janela Ordenar mods primeiro lista a ordem nova inteira, com cada mod movido marcado, o lugar antigo e o novo, e o motivo. Passe o mouse sobre um motivo que cita arquivos para vê-los. Depois de Confirmar, **Desfazer** na barra no pé da lista traz a ordem antiga de volta.

- Mods que não pedem nada mantêm a ordem, e o mínimo possível de mods muda de lugar.
- Quando duas ordens movem a mesma quantidade de mods, vence aquela em que dois mods que substituem os mesmos arquivos perdem menos. O mod que carrega depois fica com o arquivo em comum inteiro, então o mod com menos arquivos vai depois: ele sobrescreve uma parte menor do outro mod e não se perde por inteiro.
- Pedidos que não podem ser todos atendidos, como dois mods que pedem para carregar por último, não são adivinhados. Esses mods ficam onde estão, e a janela os cita em **Não movidos**, para você ordená-los à mão.
- Mods numa pasta travada não saem do lugar, e nenhum mod entra numa pasta travada. Um mod movido que cai dentro de outra pasta entra nela, e um que sai da pasta dele fica fora dela.

## Conflitos {#conflicts}

Um sinal de alerta na linha indica que o mod divide recursos com outro mod ligado. Passe o mouse para ver quais mods e quantos arquivos; clique para abrir a lista no painel da direita.

- Sinal amarelo: este mod sobrescreve arquivos de outro.
- Sinal cinza: outro mod sobrescreve alguns arquivos deste.

Conflito nem sempre é problema. Dois mods podem mudar o mesmo script de propósito, e a ordem decide qual versão você recebe. Se o resultado no jogo parecer errado, tente a ordem inversa primeiro.

> [!TIP]
> Os conflitos só contam os recursos que um mod aplicaria com as opções que você escolheu. Uma opção desligada nunca causa conflito.

## Pastas {#folders}

Pastas agrupam mods na lista. Elas não mudam nada no jogo: a ordem de carregamento continua indo do topo da lista até o fim, passando por todas as pastas.

- O botão **Criar pasta**, no topo da aba Mods, cria uma. Com mods marcados, o botão direito também põe todos numa pasta nova.
- Arraste mods para o nome de uma pasta para colocá-los dentro, e arraste a pasta para mover junto os mods que estão nela.
- Clique na seta para recolher a pasta.
- Clique com o botão direito numa pasta para renomear, trocar a cor ou apagar. Ao apagar, o Obsidian pergunta se os mods vão junto ou ficam na lista.

### Pastas travadas {#locked-folders}

O cadeado antes do nome da pasta trava a pasta. Uma pasta travada mantém o nome, a cor, o lugar e os mods que tem: nada entra nem sai até você destravar.

## Marcar vários mods {#checking-several-mods}

As caixas à esquerda selecionam mods para ações em lote. Shift+clique marca um intervalo. Com mods marcados, a barra no pé da lista liga, desliga, procura atualizações ou remove todos de uma vez. Clique com o botão direito num deles para todo o resto, como mover todos para uma pasta.

Um mod também fica no lugar que pede quando entra num perfil. Depois disso a ordem é sua: um mod que você move fica onde está até você ordenar de novo.

As listas que dizem ao jogo quais unidades existem são a exceção. Mods que adicionam uma arma ou unidade nova mudam essas listas, então o Obsidian as junta, e esses mods funcionam juntos em qualquer ordem. Todos numa partida ainda precisam dos mesmos mods.
