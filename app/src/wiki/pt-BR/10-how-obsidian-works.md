---
title: Como o Obsidian funciona
group: Primeiros passos
summary: O que o Obsidian faz no jogo, o que fica intocado e para que serve cada aba.
---

O Obsidian é um gerenciador de mods para HELLDIVERS (2015). Você guarda os mods numa biblioteca, escolhe com quais quer jogar e o Obsidian os coloca no jogo. Os arquivos do jogo nunca são editados, então vários mods rodam lado a lado e o jogo sempre pode voltar ao original.

## Patches em vez de arquivos substituídos {#patches-instead-of-replaced-files}

O jogo guarda o conteúdo em bundles, na pasta `contents` da instalação. Ao lado de cada bundle, o motor também carrega arquivos chamados `<bundle>.patch_0`, `.patch_1` e assim por diante, e um recurso dentro de um patch toma o lugar do recurso no bundle.

Mods feitos à mão costumam substituir bundles inteiros. Dois mods assim se sobrescrevem, mesmo quando mudam coisas diferentes. O Obsidian lê cada mod ligado, encontra os recursos que cada um realmente altera e grava tudo num patch por bundle. Os bundles originais ficam como estavam.

> [!NOTE]
> Aplicar grava os patches, e [Limpar](wiki:deploy-and-purge#purge) apaga. Depois de limpar, a instalação fica idêntica ao jogo original, byte por byte.

## Obsidian Loader {#obsidian-loader}

Mods que rodam código, como um mod de câmera, precisam de um jeito de começar dentro do jogo. A cada aplicação, o Obsidian coloca no jogo um script pequeno: o Obsidian Loader. Quando o jogo abre, o loader lê a lista de mods aplicados, inicia cada um e acompanha o que eles podem fazer.

O loader também cuida das regras do multijogador. Num lobby público, mods que mudam o jogo compartilhado esperam até que todos os jogadores os permitam, e missões com mods ficam fora da Guerra Galáctica. [Escopos](wiki:scopes) explica como.

## As abas {#the-tabs}

| Aba | Para que serve |
|---|---|
| Início | Links para a comunidade e o código-fonte |
| [Mods](app:mods) | A biblioteca: ligar e desligar mods, a ordem de carregamento, opções e teclas |
| Wiki | Estes guias |
| [Configurações](app:settings) | Idioma, tema, pasta do jogo, atualizações |

Início e Mods ficam à esquerda da barra. A Wiki (o livro) e as Configurações (a engrenagem) são os dois ícones na ponta direita.

A barra de cima tem as [Proteções](wiki:guardrails), o escudo, e três botões:

- **Limpar** tira do jogo todos os patches do Obsidian.
- **Aplicar** coloca os mods ligados no jogo. Ele fica verde quando há mudanças que ainda não foram aplicadas.
- **Jogar** abre o jogo pela Steam.

A barra de status, embaixo, mostra o que o Obsidian está fazendo, a versão, as notificações e o Sobre.

## Atualizar o Obsidian {#updating-obsidian}

A aba **Atualizações** em [Configurações](app:settings) mostra a sua versão, procura uma nova e lista o que cada versão mudou. Na aba Sistema, **Quando sair uma nova versão** decide o que acontece ao abrir: **Mostrar aqui**, **Perguntar ao iniciar** ou **Instalar**. A lista abre a versão instalada e as mais novas, e você pode abrir as mais antigas.

Enquanto uma nova versão baixa, uma janela na frente do app mostra o progresso. Depois o Obsidian fecha, o instalador roda por um instante e o Obsidian abre de novo na versão nova.

Uma cópia portátil não instala novas versões. Baixe-as na página de lançamentos e descompacte por cima da cópia antiga.

## Onde ficam os arquivos {#where-your-files-are}

| O quê | Onde |
|---|---|
| Biblioteca de mods, perfis, configurações do app | `%LOCALAPPDATA%\ObsidianModManager`, ou `data\` ao lado do app numa cópia portátil |
| Opções e teclas que o jogo lê, logs | `%APPDATA%\Arrowhead\Helldivers\ObsidianModManager` |
| Backups do save | a pasta `backups` dentro da pasta de dados do app |
