---
title: FAQ
group: FAQ
summary: Respostas curtas, cada uma com um link para a página que trata do assunto por completo.
---

## Jogar {#playing}

### Um mod pode fazer um jogador ser banido? {#will-a-mod-get-me-banned}

HELLDIVERS não tem anticheat, e o Obsidian nunca escreve por cima de um arquivo do jogo. Mods de sessão ficam fora da Guerra Galáctica, e num lobby público eles desligam enquanto um jogador ali não os permite ([Escopos](wiki:scopes)).

### Jogar com amigos que não usam o Obsidian funciona? {#can-i-play-with-friends-who-don-t-use-obsidian}

Sim. Mods de sessão não mudam quem um jogador encontra nem quem pode entrar. Num lobby só para amigos ou privado eles rodam com qualquer pessoa, e num lobby público desligam enquanto um jogador vanilla está ali. Arquivos do jogo substituídos por um mod de sessão não podem desligar, então um lobby público hospedado com um mod assim fica fechado para jogadores vanilla. O primeiro carregamento na nave mostra uma mensagem que diz qual caso vale ([Jogar online](wiki:multiplayer#matchmaking)).

### Dois amigos usam o Obsidian. Por que os mods de sessão estão desligados? {#my-friend-and-i-both-use-obsidian-why-are-my-session-mods-off}

Num lobby público, os mods de sessão rodam quando todos os outros jogadores têm o mesmo conjunto ou não bloqueiam mods de sessão de outros jogadores. Deixar o lobby só para amigos os faz rodar com qualquer pessoa. Exportar o perfil e mandar o arquivo para o amigo é o jeito mais rápido de bater os conjuntos ([Perfis](wiki:profiles#sharing-a-profile)).

### Missões com mods contam para a Guerra Galáctica? {#do-my-missions-count-for-the-galactic-war}

Não enquanto houver conteúdo de sessão aplicado, nem quando o conteúdo de uma expansão roda nelas. Só com mods de cliente e expansões elas contam normalmente ([A Guerra Galáctica](wiki:galactic-war)). As guerras personalizadas do Commander são diferentes: lá toda missão conta, inclusive com mods de sessão.

### O save está seguro? {#is-my-save-safe}

Antes de uma aplicação, o Obsidian faz backup da pasta de saves quando ela mudou desde o último backup. O primeiro backup e os 9 mais novos são guardados, então o save de antes de qualquer mod fica ([Aplicar e limpar](wiki:deploy-and-purge#saves)).

### A barra diz Arquivos do jogo alterados. E agora? {#the-bar-says-game-files-changed-what-now}

Alguns arquivos do jogo diferem da versão que o Obsidian conhece, geralmente porque um mod foi copiado direto para a pasta `contents` do jogo. Aplicar e Jogar continuam funcionando. Clicar no aviso mostra quais arquivos diferem e como recuperar os originais ([Solução de problemas](wiki:troubleshooting#build-unknown)).

### O que é o Commander? {#what-is-commander}

O Obsidian Commander, na aba [Guerras](app:commander), hospeda Guerras Galácticas personalizadas com regras definidas pelo anfitrião, num PC ou compartilhadas com amigos pela rede ou pela internet. A subaba Commander desta Wiki cobre o assunto ([O que é o Commander](wiki:commander)).

## Mods {#mods}

### Mods feitos antes do Obsidian funcionam? {#can-i-use-mods-made-before-obsidian}

Sim. Solte o mod na lista como sempre. O Obsidian guarda só as partes que diferem do jogo, então mods antigos podem rodar juntos ([Adicionar e atualizar mods](wiki:adding-mods#older-mods)).

### Dois mods mudam a mesma coisa. Qual vence? {#two-mods-change-the-same-thing-which-one-wins}

O que está mais abaixo na lista vence. Arrastar um mod para cima ou para baixo muda a ordem ([Ordem de carregamento](wiki:load-order)).

### É preciso um mod de ultrawide? {#do-i-need-an-ultrawide-mod}

Não. O Obsidian Patches remove as barras pretas em telas largas, e essa opção começa ligada. Um patch de ultrawide importado faz o mesmo trabalho.

### O que é o Obsidian Patches? {#what-is-obsidian-patches}

Um mod de cliente que vem com o Obsidian, feito de pequenas correções, cada uma uma opção. Começam ligadas: sem introdução nem escaneamento de planetas, sem barras pretas em telas largas, sem moldura branca em tela cheia sem bordas, telas de resultado mais rápidas depois de uma missão, uma pausa que congela um jogo solo, duas correções de animação (o recuo da arma depois de lançar um estratagema e a pose de corrida depois de um mergulho) e sem eco depois de uma rajada do Rumbler. Prontas para ligar: pular a tela de login e os resumos da guerra, uma mira que fica visível e um minimapa que fica aberto. O Obsidian Patches fica na pasta Obsidian Kit, no topo da lista.

## Criar mods {#making-mods}

### Que ferramentas criar um mod exige? {#do-i-need-any-tools}

Um editor de texto. O Obsidian compila o Lua quando aplica ([Um primeiro mod](wiki:first-mod)).

### Cliente ou sessão? {#client-or-session}

Se outros jogadores poderiam notar a mudança, sessão. Se toda mudança acontece dentro de um conteúdo separado do jogo normal, como um novo modo, expansão. Na dúvida, sessão ([Escopos](wiki:scopes)).
