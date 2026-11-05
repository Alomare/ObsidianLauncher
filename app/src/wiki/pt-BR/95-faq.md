---
title: Perguntas frequentes
group: FAQ
summary: Respostas curtas, cada uma com um link para a página que trata do assunto por inteiro.
---

## Jogar {#playing}

### Um mod pode me banir? {#will-a-mod-get-me-banned}

O HELLDIVERS não tem anti-cheat, e o Obsidian nunca grava por cima de um arquivo do jogo. Mods de sessão ficam fora da Guerra Galáctica, e num lobby público eles desligam enquanto houver ali um jogador que não os permite ([Escopos](wiki:scopes)).

### Posso jogar com amigos que não usam o Obsidian? {#can-i-play-with-friends-who-don-t-use-obsidian}

Pode. Mods de sessão não mudam quem você encontra nem quem pode entrar no seu lobby. Num lobby só para amigos ou privado, eles rodam com qualquer um, e num lobby público desligam enquanto houver um jogador sem mods. Substituições de dados e mods antigos importados não desligam, então, enquanto você hospeda um lobby público com eles, jogadores sem mods não conseguem entrar. Na primeira vez que você carrega a nave, uma mensagem diz qual caso vale para você ([Jogar online](wiki:multiplayer#matchmaking)).

### Eu e um amigo usamos o Obsidian. Por que os meus mods de sessão estão desligados? {#my-friend-and-i-both-use-obsidian-why-are-my-session-mods-off}

Num lobby público, os mods de sessão rodam quando todos os outros jogadores têm o mesmo conjunto ou não bloqueiam os mods de sessão dos outros. Deixe o lobby só para amigos e eles rodam com qualquer um. Exportar o seu perfil e mandar para o seu amigo é o jeito mais rápido de bater os conjuntos ([Perfis](wiki:profiles#sharing-a-profile)).

### As minhas missões contam para a Guerra Galáctica? {#do-my-missions-count-for-the-galactic-war}

Não enquanto houver conteúdo de sessão aplicado, nem quando o conteúdo de uma expansão roda nelas. Só com mods de cliente e expansões, elas contam normalmente ([A Guerra Galáctica](wiki:galactic-war)).

### O meu save está seguro? {#is-my-save-safe}

Antes de aplicar, o Obsidian faz backup da pasta de saves quando ela mudou desde o último backup. O primeiro backup e os 9 mais novos ficam guardados, então o save de antes de qualquer mod continua lá ([Aplicar e limpar](wiki:deploy-and-purge#saves)).

### A barra diz Arquivos do jogo alterados. E agora? {#the-bar-says-game-files-changed-what-now}

Alguns arquivos do jogo diferem da versão que o Obsidian conhece, normalmente porque um mod foi copiado direto para a pasta `contents` do jogo. Aplicar e Jogar continuam funcionando. Clique no aviso para ver quais arquivos mudaram e como ter os originais de volta ([Quando algo dá errado](wiki:troubleshooting#build-unknown)).

## Mods {#mods}

### Posso usar mods feitos antes do Obsidian? {#can-i-use-mods-made-before-obsidian}

Pode. Solte o mod na lista como sempre. O Obsidian guarda só as partes que diferem do jogo, então mods antigos rodam juntos ([Adicionar e atualizar mods](wiki:adding-mods#older-mods)).

### Dois mods mudam a mesma coisa. Qual vence? {#two-mods-change-the-same-thing-which-one-wins}

O que está mais abaixo na lista ([Ordem de carregamento](wiki:load-order)).

### Preciso de um mod para ultrawide? {#do-i-need-an-ultrawide-mod}

Não. O Obsidian Patches tira as barras pretas em telas largas, e essa opção começa ligada. Um patch de ultrawide importado faz o mesmo.

### O que é o Obsidian Patches? {#what-is-obsidian-patches}

Um mod de cliente que vem com o Obsidian, feito de pequenas correções que são, cada uma, uma opção. Começam ligadas: sem abertura, tela de login ou varredura de planetas, sem barras pretas em telas largas, telas de resultado mais rápidas depois de uma missão, uma pausa que congela o jogo solo e duas correções de animação, o recuo da arma depois de lançar um estratagema e a pose de corrida depois de um mergulho. Prontas para ligar: pular os resumos da guerra, um cursor de mira sempre visível e um minimapa que fica aberto. Ele fica na pasta Obsidian Kit, no topo da lista.

## Criar mods {#making-mods}

### Preciso de alguma ferramenta? {#do-i-need-any-tools}

Um editor de texto. O Obsidian compila o Lua quando aplica ([O seu primeiro mod](wiki:first-mod)).

### Cliente ou sessão? {#client-or-session}

Se outros jogadores puderem perceber a mudança, sessão. Se todas as mudanças acontecem dentro de um conteúdo separado do jogo normal, como um modo novo, expansão. Na dúvida, sessão ([Escopos](wiki:scopes)).
