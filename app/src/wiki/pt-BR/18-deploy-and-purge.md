---
title: Aplicar e limpar
group: Primeiros passos
summary: O que acontece quando você aplica, o que Limpar remove e por que o Obsidian às vezes recusa.
---

## Aplicar {#deploy}

Aplicar coloca no jogo os mods ligados do perfil ativo. O botão fica verde quando há mudanças que ainda não estão no jogo, inclusive um valor de opção ou uma tecla. O jogo precisa estar fechado.

Cada aplicação substitui todos os patches da anterior, então não é preciso limpar antes: um mod que você desligou ou removeu sai do jogo na próxima aplicação. Quando só mudaram valores de opção ou teclas, nada é reconstruído e a aplicação leva um instante.

Cada aplicação segue estes passos:

1. Verifica se o jogo é a versão que o Obsidian conhece (veja abaixo).
2. Faz backup da pasta de saves, quando ela mudou desde o último backup. O primeiro backup e os 9 mais novos ficam guardados, então o seu save de antes de qualquer mod continua lá.
3. Decide qual mod vence cada recurso, seguindo a ordem de carregamento.
4. Compila o Lua de cada mod, e do loader, em código que o jogo consegue rodar.
5. Grava um patch por bundle do jogo e a lista de mods aplicados para o loader.

**Jogar** abre o jogo com os mods que você aplicou por último. Ele não aplica nada, então as mudanças esperando ficam fora do jogo até você usar **Aplicar**.

## Limpar {#purge}

Limpar apaga todos os arquivos que o Obsidian gravou e confere que nenhum ficou para trás. O jogo volta a ser o original. A biblioteca e os perfis continuam; aplique de novo para trazer os mods de volta.

> [!NOTE]
> Patches que outra ferramenta ou uma pessoa colocou na pasta do jogo aparecem em Jogo, nas [Configurações](app:settings), e Limpar nunca os apaga. Os patches do Obsidian ficam acima deles. **Remover outros patches**, ali mesmo, tira esses patches do jogo e os guarda na pasta `removed-patches`, dentro da pasta de dados, de onde você pode copiá-los de volta à mão.

> [!WARNING]
> Use o Purge antes de desinstalar o Obsidian. Os patches ficam na pasta do jogo até alguém removê-los, então o jogo continuaria rodando os seus mods.

## A verificação de versão {#the-build-check}

O Obsidian foi feito para uma versão Steam do HELLDIVERS. Se alguns arquivos do jogo forem diferentes dela, a barra de cima mostra **Arquivos do jogo alterados**. Clique para ver quais arquivos mudaram e o que fazer. Aplicar e Jogar continuam funcionando, e o Obsidian nunca grava por cima dos arquivos do próprio jogo.

O HD1 não recebe atualização desde 2020, então isso quase sempre quer dizer que um mod foi copiado direto para a pasta `contents`. Os mods do Obsidian carregam por cima dele, e onde os dois mudam a mesma coisa, o do Obsidian vence. Para ter os arquivos conhecidos de volta, use Limpar, verifique a integridade dos arquivos na Steam e depois adicione o mod copiado com **Adicionar mods**. Adicionar mods espera até os arquivos voltarem a bater, porque compara cada mod com os arquivos originais.

O loader confere a versão do jogo toda vez que o jogo abre. Numa versão que ele não conhece, nenhum mod roda e o jogo avisa.

## Saves {#saves}

O Obsidian nunca edita o seu progresso. Com conteúdo de sessão em jogo, ele só limpa os resultados que esperam envio, para que não cheguem à Guerra Galáctica depois. Mods de sessão podem mudar o seu próprio progresso (XP, desbloqueios), e o Obsidian não desfaz isso; por isso cada aplicação faz um backup antes. Os backups ficam na pasta `backups`, dentro da pasta de dados do app.
