---
title: Saves
group: Primeiros passos
summary: Saves Oficiais e Sandbox, Steam Cloud e o grupo de matchmaking Sandbox.
---

A aba [Saves](app:saves) guarda todos os saves da conta Steam conectada no PC. Há dois tipos: saves **Oficiais** são os arquivos normais do jogo, e saves **Sandbox** são saves extras que o Obsidian guarda separados deles. O save marcado **Ativo** é o que o jogo abre na próxima vez, e nada aqui muda enquanto o jogo está aberto.

A aba segue o layout da aba Mods: uma tabela dos saves à esquerda, com patente, amostras e tempo jogado, e o painel do save selecionado à direita. Arrastar a linha entre eles redimensiona o painel, e um clique duplo na linha o restaura. Os botões no topo do painel são o que o save selecionado pode fazer, cada um com uma dica, e o menu do botão direito numa linha tem as mesmas ações. Um clique duplo num Sandbox abre o editor. **Novo**, no topo, cria um save Sandbox ou Oficial em branco, ou importa um Sandbox de um zip.

## Saves Oficiais {#official-saves}

Um save Oficial é o progresso que o jogo e a Steam conhecem. Na primeira vez que a aba Saves abre, os arquivos atuais do jogo entram como um save Oficial chamado Main. Um save Oficial pode:

- **Mudar para este save**, um save Oficial mais antigo, ou **Restaurar** uma cópia. O Obsidian primeiro guarda o save em uso na biblioteca e faz backup dos arquivos do jogo, depois escreve o save escolhido por cima deles.
- **Novo** começa um save Oficial em branco, guardado na biblioteca até alguém mudar para ele.
- **Cópia** guarda uma cópia do save para voltar depois.
- **Duplicar como Sandbox** copia o progresso para um novo Sandbox e deixa o save Oficial como está.
- **Apagar** remove um save Oficial guardado na biblioteca, um por vez, depois de um backup e de digitar o nome do save. O save Oficial nos arquivos do jogo não pode ser apagado: mude para outro save Oficial antes.

Saves Oficiais não podem ser renomeados, exportados nem editados.

### Steam Cloud {#steam-cloud}

O jogo carrega primeiro a cópia da Steam Cloud, então mudar de save Oficial, com o jogo fechado, escreve os arquivos da Cloud além dos locais. A Steam envia o novo save na próxima sincronização e não mostra nenhuma janela. Quando a Steam não está acompanhando os arquivos, o Obsidian escreve só os locais, e o jogo recria os arquivos da Cloud no próximo salvamento. Mudar de save também apaga os resultados esperando envio aos servidores do jogo, para que um save antigo não relate missões antigas.

Os backups dos arquivos do jogo ficam na pasta de backups ([Onde ficam os arquivos](wiki:how-obsidian-works#where-your-files-are)).

## Saves Sandbox {#sandbox-saves}

Um Sandbox existe só neste PC, então nunca toca a Steam Cloud nem os arquivos Oficiais. Saves Sandbox podem ser criados, duplicados (também a partir de um save Oficial), renomeados, apagados, exportados para um zip, importados de um e [editados](#the-save-editor).

Todo Sandbox novo, em branco, duplicado ou importado, começa com todos os tutoriais feitos: o treinamento básico, os tutoriais da guerra galáctica, a introdução e as dicas do almirante na nave, e as setas do hellpod.

Um Sandbox precisa do carregador do Obsidian no jogo, então os mods precisam estar aplicados. Apertar Jogar com um Sandbox selecionado e nada aplicado pede para [Aplicar](wiki:deploy-and-purge) antes. Sem o carregador, o jogo abre o save Oficial, intacto.

### O grupo de matchmaking Sandbox {#the-sandbox-matchmaking-pool}

Saves Sandbox não podem mexer na guerra oficial. Enquanto um Sandbox está ativo:

- O envio de resultados aos servidores do jogo é bloqueado e os resultados pendentes são descartados.
- As conquistas são bloqueadas.
- Jogadores Sandbox só veem e entram em lobbies de outros jogadores Sandbox, e o terminal de multijogador mostra **Sandbox públicos** acima da lista de lobbies. Amigos estão incluídos: um jogador num Sandbox e um amigo num save Oficial não entram um no lobby do outro, e o convite recusado explica isso.

Saves Oficiais entram em qualquer lobby, como sempre ([Jogar online](wiki:multiplayer)). Uma guerra personalizada do Commander roda num save Sandbox, num grupo separado: qualquer Sandbox, ou o que foi criado para a guerra quando nenhum é escolhido ou quando o anfitrião restringe a guerra a ele.

## O editor de saves {#the-save-editor}

**Editar** num Sandbox abre o editor, com o jogo fechado. As mudanças esperam por **Salvar**, que escreve todas de uma vez. **Descartar**, a seta ao lado de Salvar, esquece todas, e a seta no fim de uma linha desfaz só a mudança daquela linha. As abas são:

- **Visão geral**: as estatísticas do save, **Maximizar tudo** e **Zerar progresso**. Os dois perguntam antes e gravam na hora, então não podem ter outras mudanças pendentes.
- **Personagem**: a patente (uma lista ligada ao campo de XP), as amostras com os pontos de pesquisa que elas dão, cosméticos, influência dos chefes e marcadores de tutorial.
- **Arsenal**: armas, estratagemas e vantagens, uma tabela por categoria, com uma caixa de busca. Cada linha tem um interruptor de posse, as armas têm melhorias nomeadas e os estratagemas têm um nível.
- **Mods**: uma seção por mod com dados no save. Um mod desligado fica acinzentado e mantém os dados. Um mod que saiu da biblioteca lista os dados pelo nome, com um botão de apagar.

Nomes e limites dos itens vêm de uma lista tirada do jogo. Itens do save que a lista não conhece são mantidos e mostrados com o nome bruto. Um item que vem com uma patente não pode ser desligado, já que a patente o devolve.

### DLC pago {#paid-dlc}

O Obsidian lê quais DLC a conta Steam possui nos saves Oficiais, onde o jogo registra isso a cada login.

- DLC possuído mostra **Possuído na Steam**. A Steam o concede, então o interruptor de posse fica ligado, mas as melhorias podem mudar, e os cosméticos de DLC possuído podem ser escolhidos. **Maximizar tudo** maximiza as melhorias de DLC possuído, e **Zerar progresso** as zera.
- DLC que a conta não possui aparece trancado, com um cadeado no lugar do interruptor, e as listas de cosméticos deixam esses cosméticos de fora. O editor nunca escreve esse DLC, e Maximizar tudo e Zerar progresso não mexem nele.

A verificação é tão recente quanto o último login. DLC comprado depois disso, ou desmarcado na Steam naquele login, conta como não possuído até o jogo abrir uma vez com um save Oficial. O registro dentro de um Sandbox é ignorado, já que um Sandbox importado pode trazer o registro de outro jogador.

> [!NOTE]
> Saves exportados e importados são sempre saves Sandbox. A importação confere o arquivo, remove os resultados pendentes e remove do save importado o DLC pago que a conta não possui.
