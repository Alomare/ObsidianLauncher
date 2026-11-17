---
title: Como o Obsidian funciona
group: Primeiros passos
summary: O que o Obsidian faz no jogo, o que fica intocado e para que serve cada aba.
---

O Obsidian é um launcher e gerenciador de mods para HELLDIVERS (2015). Os mods ficam numa biblioteca, um perfil escolhe quais entram no jogo, e o Obsidian os coloca no jogo. Nenhum arquivo do jogo é editado, então vários mods rodam lado a lado e o jogo sempre pode voltar ao vanilla.

## Patches em vez de arquivos substituídos {#patches-instead-of-replaced-files}

O jogo guarda o conteúdo em bundles, na pasta `contents` da instalação. Ao lado de cada bundle, o motor também carrega arquivos chamados `<bundle>.patch_0`, `.patch_1` e assim por diante, e um recurso num patch toma o lugar do recurso do bundle.

Mods feitos à mão costumam substituir bundles inteiros, então dois desses mods se sobrescrevem mesmo quando mudam coisas diferentes. O Obsidian lê cada mod ligado, encontra os recursos que cada um realmente muda e os escreve num patch por bundle. Os bundles vanilla ficam como estavam.

> [!NOTE]
> **Aplicar** escreve os patches, e [Limpar](wiki:deploy-and-purge#purge) os apaga. Depois de limpar, a instalação é, byte a byte, o jogo vanilla.

## Obsidian Loader {#obsidian-loader}

Mods que rodam código, como um mod de câmera, precisam de um jeito de começar dentro do jogo. A cada aplicação, o Obsidian adiciona ao jogo um pequeno script, o Obsidian Loader. Quando o jogo abre, o carregador lê a lista de mods aplicados, inicia cada um e vigia o que eles podem fazer.

O carregador também aplica as regras do multijogador. Num lobby público, mods que mudam o jogo compartilhado esperam até que todos os jogadores os permitam, e missões com mods ficam fora da Guerra Galáctica. [Escopos](wiki:scopes) explica como.

## As abas {#the-tabs}

| Aba | Para que serve |
|---|---|
| Início | Botões para a aba Loja, o Nexus Mods, o servidor do Discord, o Ko-fi e o código-fonte no GitHub |
| [Loja](app:store) | Mods para instalar com um clique ([mais](wiki:tab-store)) |
| [Mods](app:mods) | A biblioteca: mods ligados e desligados, ordem de carregamento, opções, teclas, **Aplicar** e **Limpar** ([mais](wiki:tab-mods)) |
| [Saves](app:saves) | Saves Oficiais e Sandbox, e o editor de saves ([mais](wiki:tab-saves)) |
| [Guerras](app:commander) | Guerras Galácticas personalizadas, explicadas na subaba Commander desta Wiki ([mais](wiki:tab-wars)) |
| Wiki | Estes guias ([mais](wiki:tab-wiki)) |
| [Conta](app:account) | O login do GitHub, relatos de problemas e sincronização entre PCs ([mais](wiki:tab-account)) |
| [Configurações](app:settings) | Pasta do jogo, idioma, atualizações, controle e aparência ([mais](wiki:tab-settings)) |

A barra no topo da janela tem, da esquerda para a direita:

- As abas Início, Loja, Mods, Saves e Guerras.
- O botão das [Proteções](wiki:guardrails), um escudo.
- **Jogar**, que abre o jogo pela Steam com os mods da última aplicação. Quando há mais de uma guerra ou save para escolher, um botão de menu ao lado de **Jogar** escolhe a guerra e o save que **Jogar** abre.
- Três ícones: a Wiki (um livro), a Conta (uma pessoa) e as Configurações (uma engrenagem). Um ponto na engrenagem quer dizer que saiu uma nova versão do Obsidian.

**Aplicar** e **Limpar** ficam na ponta direita da aba [Mods](app:mods) ([Aplicar e limpar](wiki:deploy-and-purge)).

A barra de status, embaixo, mostra o que o Obsidian está fazendo, a versão, as notificações e a janela Sobre.

## Atualizar o Obsidian {#updating-obsidian}

A aba **Atualizações** das [Configurações](app:settings) mostra a versão instalada, procura uma nova e lista o que cada versão mudou. **Quando sair uma nova versão**, na aba Sistema, decide o que acontece ao iniciar: **Mostrar aqui**, **Perguntar ao iniciar** ou **Instalar**. A lista abre com a versão instalada e as mais novas, e as mais antigas abrem com um clique.

Enquanto uma nova versão baixa, uma janela na frente do app mostra o progresso. Depois o Obsidian fecha, o instalador roda por um instante e o Obsidian abre de novo na nova versão. Uma cópia portátil não instala novas versões: baixe-as na página de releases e descompacte por cima da cópia antiga.

## Onde ficam os arquivos {#where-your-files-are}

| O quê | Windows | Linux |
|---|---|---|
| Biblioteca de mods, perfis, configurações do app | `%LOCALAPPDATA%\ObsidianLauncher`, ou a pasta `data` ao lado do app numa cópia portátil | `~/.local/share/ObsidianLauncher` |
| Opções e teclas que o jogo lê, logs | `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher` | A mesma pasta dentro do prefixo Proton do jogo: `drive_c/users/steamuser/AppData/Roaming/Arrowhead/Helldivers/ObsidianLauncher` |
| Backups dos saves | A pasta `backups` na pasta de dados do app | A mesma |

**Pasta de dados**, na aba Sistema das [Configurações](app:settings), abre a pasta de dados do app, e **Abrir logs** abre os logs.
