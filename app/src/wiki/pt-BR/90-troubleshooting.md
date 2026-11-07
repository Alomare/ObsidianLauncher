---
title: Quando algo dá errado
group: Ajuda
summary: Logs, travamentos na abertura, mods que não carregam e como voltar ao jogo original.
---

## Primeiro, volte ao original {#back-to-vanilla-first}

Quando o jogo se comporta mal, **Limpar** tira do jogo todos os arquivos do Obsidian. Se o problema continuar depois disso, a causa não é o Obsidian; verifique a integridade dos arquivos na Steam.

## O jogo travou {#the-game-crashed}

O loader escreve uma linha logo antes de iniciar cada mod. Se o jogo parar nesse momento, na próxima vez que você abrir o Obsidian ele aponta o mod que estava iniciando. Desligue esse mod, aplique e avise o autor.

Se o jogo parar depois disso, o Obsidian avisa que ele não fechou normalmente. Um travamento dentro do motor do jogo costuma não deixar erro no log, então tente desligar os mods que você adicionou por último, um de cada vez.

## Um mod não faz nada {#a-mod-does-nothing}

- Confira se o mod está ligado e se você aplicou depois de ligar. Um botão **Aplicar** verde quer dizer que há mudanças esperando.
- Num mod de sessão, confira o lobby: num lobby público com jogadores sem mods, ou que bloqueiam mods de sessão, os mods de sessão desligam ([Jogar online](wiki:multiplayer)).
- Leia os logs (abaixo). Um mod que falhou ao iniciar tem uma linha de erro ali.

## "Arquivos do jogo alterados" {#build-unknown}

Alguns arquivos do jogo não batem com a versão que o Obsidian conhece. Isso acontece quando um mod foi copiado direto para a pasta `contents` do jogo por cima dos arquivos originais. Aplicar e Jogar continuam funcionando, e clicar no aviso lista os arquivos alterados. Use Limpar e depois verifique a integridade dos arquivos na Steam para trazer os originais de volta. Depois, adicione esse mod no Obsidian com **Adicionar**, que o aplica sem mexer nos originais.

Se o próprio jogo disser que a versão é desconhecida, o programa do jogo não é a versão para a qual o Obsidian foi feito, e nenhum mod roda.

## Logs {#logs}

Os logs ficam em `%APPDATA%\Arrowhead\Helldivers\ObsidianModManager\logs`:

| Arquivo | O que tem |
|---|---|
| `loader.log` | Cada mod iniciando, os hooks entrando, o estado do multijogador e o que cada mod mudou fora do loader |
| `<id do mod>.log` | As linhas que aquele mod escreveu com `obsidian.log`, e mais nada |

## Pedir ajuda {#asking-for-help}

Use **Copiar relatório** em [Configurações](app:settings), na seção Dados. Ele copia as versões, os mods ligados e o fim do `loader.log`, sem o nome da sua conta do Windows. Cole isso, junto com o que aconteceu, no [Discord](https://dsc.gg/hd1modding) ou a uma [issue no GitHub](https://github.com/Alomare/ObsidianModManager/issues).
