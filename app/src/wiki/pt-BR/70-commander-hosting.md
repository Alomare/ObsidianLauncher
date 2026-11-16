---
title: Hospedar uma guerra
group: Hospedar e compartilhar
summary: O que o PC do anfitrião faz, o que para quando o Obsidian fecha e onde as guerras ficam guardadas.
---

## O PC do anfitrião {#the-host-pc}

O PC com o pacote do Commander hospeda todas as guerras criadas nele. O servidor da guerra roda dentro do Obsidian ali, responde aos jogos de todos os jogadores da guerra e mantém o relógio da guerra. O anfitrião pode jogar na mesma guerra pelo mesmo PC como qualquer outro jogador.

Toda guerra precisa desse PC ligado e do Obsidian aberto nele. Quando o Obsidian fecha, o servidor da guerra para junto: o relógio da guerra congela, os jogos dos outros jogadores mostram a guerra como offline e **Share** fecha de novo. Depois do próximo início, abra **Share** e ligue de novo **LAN** e **Internet**. O código de convite continua o mesmo, então os jogadores não precisam de um novo.

## Share {#share}

**Share**, na tela da guerra, abre as opções de compartilhamento daquela guerra: **LAN** para jogadores na mesma rede, **Internet** para jogadores em outros lugares, e o código de convite que a Internet exige. [Compartilhar uma guerra](wiki:commander-sharing) cobre cada um. Uma etiqueta ao lado da temporada mostra quando uma guerra está aberta para a LAN ou para a internet.

Na primeira vez que uma guerra abre para a rede, o Windows pergunta se o servidor da guerra pode passar pelo firewall. Permita em redes privadas, senão os PCs da rede não alcançam a guerra. Quando o Windows lista a rede de casa como pública, mude antes o perfil de rede para privado nas configurações de rede do Windows. No Windows, o Obsidian nunca cria regras de firewall por conta própria, e **Remover todos os dados do Commander** pode remover a regra de novo.

No Linux, quando o firewalld ou o ufw mantêm as portas fechadas, o Obsidian oferece **Permitir guerras na LAN**: ele abre as portas TCP 17494 a 17499 e a porta UDP 17493 depois que o sistema pede a senha. **Remover todos os dados do Commander** as fecha de novo.

## Onde as guerras ficam guardadas {#where-wars-are-kept}

As guerras do anfitrião, com histórico e estatísticas, ficam em `%LOCALAPPDATA%\ObsidianLauncher\commander\server`. **Export War**, na tela da guerra, salva uma guerra num único arquivo, que é o jeito de fazer backup de uma guerra ou movê-la para outro PC.

Uma guerra tem um anfitrião por vez. Dois PCs não podem dividir a hospedagem, e uma guerra importada em outro PC começa uma cópia separada, em que os jogadores entram de novo.
