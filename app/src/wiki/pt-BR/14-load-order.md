---
title: Ordem, conflitos e pastas
group: Primeiros passos
summary: Qual mod vence quando dois mudam a mesma coisa, e como manter uma lista longa organizada.
---

## Ordem de carregamento {#load-order}

O número ao lado de cada mod é a posição dele na ordem de carregamento. Quando dois mods ligados mudam o mesmo recurso do jogo, vence o que está mais abaixo na lista. Arraste um mod para cima ou para baixo para mudar a ordem, ou selecione e use Alt+↑ e Alt+↓.

## Conflitos {#conflicts}

Um sinal de alerta na linha indica que o mod divide recursos com outro mod ligado. Passe o mouse para ver quais mods e quantos arquivos; clique para abrir a lista no painel da direita.

- Sinal amarelo: este mod sobrescreve arquivos de outro.
- Sinal cinza: outro mod sobrescreve alguns arquivos deste.

Conflito nem sempre é problema. Dois mods podem mudar o mesmo script de propósito, e a ordem decide qual versão você recebe. Se o resultado no jogo parecer errado, tente a ordem inversa primeiro.

> [!TIP]
> Os conflitos só contam os recursos que um mod aplicaria com as opções que você escolheu. Uma opção desligada nunca causa conflito.

## Pastas {#folders}

Pastas agrupam mods na lista. Elas não mudam nada no jogo: a ordem de carregamento continua indo do topo da lista até o fim, passando por todas as pastas.

- O botão azul **Criar pasta**, ao lado do botão de perfil, cria uma. Com mods marcados, o botão direito também põe todos numa pasta nova.
- Arraste mods para o nome de uma pasta para colocá-los dentro, e arraste a pasta para mover junto os mods que estão nela.
- Clique na seta para recolher a pasta.
- Clique com o botão direito numa pasta para renomear, trocar a cor ou apagar. Ao apagar, o Obsidian pergunta se os mods vão junto ou ficam na lista.

### Pastas travadas {#locked-folders}

O cadeado antes do nome da pasta trava a pasta. Uma pasta travada mantém o nome, a cor, o lugar e os mods que tem: nada entra nem sai até você destravar.

## Marcar vários mods {#checking-several-mods}

As caixas à esquerda selecionam mods para ações em lote. Shift+clique marca um intervalo. Com mods marcados, a barra no pé da lista liga, desliga, move para uma pasta ou remove todos de uma vez.

Um mod pode pedir um lugar quando entra num perfil: no fim da lista ou depois de outro mod. Depois disso a ordem é sua, e o que você mover fica onde está.

As listas que dizem ao jogo quais unidades existem são a exceção. Mods que adicionam uma arma ou unidade nova mudam essas listas, então o Obsidian as junta, e esses mods funcionam juntos em qualquer ordem. Todos numa partida ainda precisam dos mesmos mods.
