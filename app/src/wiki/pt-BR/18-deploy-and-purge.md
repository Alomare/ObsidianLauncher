---
title: Aplicar e limpar
group: Primeiros passos
summary: O que uma aplicação faz, o que Limpar remove e por que o Obsidian às vezes recusa.
---

## Aplicar {#deploy}

**Aplicar** e **Limpar** são os dois ícones na ponta direita da fileira de cima da aba [Mods](app:mods). Os dois exigem o jogo fechado.

Aplicar coloca no jogo os mods ligados do perfil ativo. O ícone **Aplicar** fica verde enquanto há mudanças que ainda não estão no jogo, inclusive um valor de opção ou uma tecla.

Cada aplicação substitui todos os patches da anterior, então não é preciso limpar antes: um mod desligado ou removido sai do jogo na próxima aplicação. Quando só mudaram valores de opção ou teclas, nada é reconstruído e a aplicação leva um instante. Cada aplicação segue estes passos:

1. Verifica se o jogo é a versão que o Obsidian conhece (veja abaixo).
2. Faz backup da pasta de saves quando ela mudou desde o último backup. O primeiro backup e os 9 mais novos são guardados, então o save de antes de qualquer mod fica.
3. Decide qual mod vence em cada recurso, seguindo a ordem de carregamento.
4. Compila o Lua de cada mod, e o carregador, em código que o jogo consegue rodar.
5. Escreve um patch por bundle do jogo, e a lista de mods aplicados para o carregador.

**Jogar**, na barra no topo da janela, abre o jogo com os mods da última aplicação. **Jogar** não aplica, então mudanças pendentes ficam fora do jogo até **Aplicar**. Sem nada aplicado, **Jogar** pergunta antes, já que o jogo abriria vanilla. **Não perguntar de novo** para a pergunta, e **Perguntar antes de jogar sem mods**, na aba Sistema das [Configurações](app:settings), a traz de volta.

## Limpar {#purge}

Limpar apaga todos os arquivos que o Obsidian escreveu e confere que nenhum ficou, então o jogo volta a ser o jogo vanilla. A biblioteca e os perfis ficam, e a próxima aplicação traz os mods de volta.

> [!NOTE]
> Patches que outra ferramenta ou uma pessoa colocou na pasta do jogo aparecem em **Outros patches**, na aba Sistema das [Configurações](app:settings), e Limpar nunca os apaga. Os patches do Obsidian ficam acima deles. **Remover outros patches**, ali, os tira do jogo para a pasta `removed-patches` na pasta de dados, de onde podem ser copiados de volta à mão.

> [!WARNING]
> Limpe antes de desinstalar o Obsidian. Os patches ficam na pasta do jogo até algo removê-los, então o jogo continuaria rodando os mods.

## A verificação da versão {#the-build-check}

O Obsidian foi feito para uma versão da Steam de HELLDIVERS. Quando alguns arquivos do jogo diferem dessa versão, a barra no topo da janela mostra **Arquivos do jogo alterados**, e um clique nesse aviso mostra quais arquivos diferem e o que fazer. Aplicar e Jogar continuam funcionando, e o Obsidian nunca escreve por cima de um arquivo original do jogo.

HD1 não recebe atualização desde 2020, então isso quase sempre quer dizer que um mod foi copiado direto para a pasta `contents`. Os mods do Obsidian carregam por cima dele, e onde os dois mudam a mesma coisa, vence o do Obsidian. Para recuperar os arquivos conhecidos, limpe, verifique os arquivos do jogo na Steam e então adicione o mod copiado com **Adicionar**. Adicionar mods espera até os arquivos baterem de novo, já que compara cada mod com os arquivos originais.

O carregador confere a versão do jogo toda vez que o jogo abre. Numa versão que ele não conhece, nenhum mod roda e o jogo avisa.

## Saves {#saves}

Uma aplicação nunca edita o progresso. Enquanto há conteúdo de sessão em jogo, o carregador só apaga os resultados esperando envio, para que não cheguem à Guerra Galáctica depois. Mods de sessão podem mudar progresso, como XP e desbloqueios, e o Obsidian não desfaz isso, por isso cada aplicação faz um backup antes. Os backups ficam na pasta `backups` da pasta de dados do app, e a lista **Backups dos saves**, na aba Sistema das [Configurações](app:settings), abre cada um. O [editor de saves](wiki:saves#the-save-editor) é a única parte do Obsidian que muda progresso, e só em saves Sandbox.
