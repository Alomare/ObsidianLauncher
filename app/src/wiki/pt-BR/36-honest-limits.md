---
title: O que as proteções não fazem
group: Jogar online
summary: Contra o que o Obsidian protege, e o que nenhum gerenciador de mods consegue impedir.
---

As proteções impedem que mods bem-comportados prejudiquem outros jogadores por acidente, e fazem da configuração segura a padrão. Elas não são um anticheat.

## O que elas cobrem {#what-they-cover}

- Um jogador vanilla num lobby público nunca joga sob mods de sessão: eles desligam, e conteúdo que não pode desligar mantém jogadores vanilla fora daquele lobby.
- Mods de sessão que usam o carregador desligam sozinhos num lobby público que não os permitiu.
- Missões jogadas com conteúdo de sessão não são enviadas à Guerra Galáctica.
- Conquistas e progressão da conta ficam como estavam sempre que as [Proteções](wiki:guardrails) pedem, não importa o que os mods façam.
- Um mod que falha repetidamente é desligado, e um travamento na inicialização cita o mod que estava começando.

## O que elas não cobrem {#what-they-can-t-cover}

Tudo roda no PC do jogador, então uma pessoa que quer trapacear pode escrever um script que ignora o carregador por completo. Nenhum gerenciador de mods impede isso. Um mod também pode quebrar as regras por engano ou de propósito, dizendo-se mod de cliente enquanto muda o jogo compartilhado, ou mudando o código do jogo diretamente, sem os ganchos do carregador, de modo que o carregador não consegue desligar a mudança.

Quando um mod começa, o carregador compara o jogo antes e depois e escreve uma linha por mod no log do carregador, listando o que o mod adicionou ou substituiu fora do carregador. Essa linha é onde olhar quando um mod se comporta mal.

Um mod que escrevesse os arquivos de save por conta própria, de fora do código do jogo, poderia contornar **Congelar a progressão da conta**. O Obsidian faz backup do save antes de cada aplicação ([Aplicar e limpar](wiki:deploy-and-purge#saves)).

> [!TIP]
> Instale mods de autores e lugares confiáveis, e leia o que um mod diz que faz. Um mod de cliente que muda coisas que outros jogadores veem merece um relato ao autor ou no Discord.
