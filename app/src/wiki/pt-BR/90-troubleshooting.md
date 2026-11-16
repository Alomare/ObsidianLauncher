---
title: Solução de problemas
group: Ajuda
summary: Logs, travamentos na inicialização, mods que não carregam e voltar ao vanilla.
---

## Primeiro, volte ao vanilla {#back-to-vanilla-first}

Quando o jogo se comporta mal, **Limpar** tira do jogo todos os arquivos do Obsidian. Um problema que continua depois de limpar não vem do Obsidian, e verificar os arquivos do jogo na Steam é o próximo passo.

## O jogo travou {#the-game-crashed}

O carregador escreve uma linha logo antes de iniciar cada mod. Quando o jogo para nesse momento, um aviso na aba Mods cita o mod que estava começando, na próxima vez que o Obsidian abrir. **Desligar**, no aviso, desliga esse mod: aplique e avise o autor.

Quando o jogo para mais tarde, o Obsidian só diz que o jogo não fechou normalmente. Um travamento dentro do motor costuma não deixar erro no log, então desligue os últimos mods adicionados, um por vez, até o travamento sumir.

## Um mod não faz nada {#a-mod-does-nothing}

- Confira se o mod está ligado e se uma aplicação rodou depois de ligá-lo. Um botão **Aplicar** verde quer dizer que há mudanças pendentes.
- Para um mod de sessão, confira o lobby: num lobby público com jogadores vanilla, ou com jogadores que bloqueiam mods de sessão, os mods de sessão desligam ([Jogar online](wiki:multiplayer)).
- Leia os logs abaixo. Um mod que falhou na inicialização deixa uma linha de erro ali.

## "Arquivos do jogo alterados" {#build-unknown}

Alguns arquivos do jogo não batem com a versão que o Obsidian conhece. Isso acontece quando um mod foi copiado direto para a pasta `contents` do jogo, por cima dos arquivos originais. Aplicar e Jogar continuam funcionando, e clicar no aviso lista os arquivos alterados. Para recuperar os originais, limpe, depois verifique os arquivos do jogo na Steam, e adicione esse mod no Obsidian com **Adicionar**, que o aplica sem tocar nos originais. Quando o próprio jogo diz que a versão é desconhecida, o programa do jogo não é a versão para a qual o Obsidian foi feito, e nenhum mod roda.

## Logs {#logs}

Os logs ficam em `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs` no Windows ([Onde ficam os arquivos](wiki:how-obsidian-works#where-your-files-are) tem a pasta no Linux). **Abrir logs**, na aba Sistema das [Configurações](app:settings), abre a pasta.

| Arquivo | O que guarda |
|---|---|
| `loader.log` | Cada mod começando, ganchos entrando, o estado do multijogador, as linhas da guerra personalizada, e o que cada mod mudou fora do carregador |
| `<id do mod>.log` | As linhas que aquele mod escreveu com `obsidian.log`, e nada mais |

## Pedir ajuda {#asking-for-help}

**Copiar relatório**, em **Relatório de problema** na aba Sistema das [Configurações](app:settings), copia as versões, os mods ligados e o fim do `loader.log`, sem o nome da conta do Windows. Cole-o com uma descrição do que aconteceu no [Discord](https://dsc.gg/hd1modding) ou numa [issue no GitHub](https://github.com/Alomare/ObsidianLauncher/issues). Problemas com guerras personalizadas têm uma página separada na Wiki do Commander ([Solução de problemas](wiki:commander-troubleshooting)).
