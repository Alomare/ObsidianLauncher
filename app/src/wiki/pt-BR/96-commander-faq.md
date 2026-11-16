---
title: FAQ
group: FAQ
summary: Respostas curtas sobre guerras personalizadas, cada uma com um link para a página que trata do assunto por completo.
---

## Guerras {#wars}

### Uma guerra personalizada muda a guerra oficial ou o save Oficial? {#does-a-custom-war-change-the-official-war-or-the-official-save}

Não. Uma guerra personalizada roda num save Sandbox, num grupo de matchmaking separado, e os resultados das missões só chegam ao servidor da guerra ([O que é o Commander](wiki:commander#kept-apart-from-the-official-war)).

### Mods funcionam numa guerra personalizada? {#do-mods-work-in-a-custom-war}

Sim, e missões jogadas com mods de sessão contam lá, já que os resultados nunca chegam à guerra oficial. Num lobby, os mods de sessão seguem as regras normais do multijogador ([Jogar online](wiki:multiplayer)).

### Para onde vão a patente e os desbloqueios ganhos numa guerra? {#where-do-rank-and-unlocks-earned-in-a-war-go}

Para o save Sandbox em que a guerra foi jogada. Eles ficam ali entre temporadas e não passam para o save Oficial ([Jogar uma guerra](wiki:commander-playing#the-war-save)).

### Uma guerra continua andando com o PC do anfitrião desligado? {#does-a-war-keep-running-while-the-host-pc-is-off}

Não. O servidor da guerra roda dentro do Obsidian no PC do anfitrião, então o relógio da guerra congela enquanto o Obsidian está fechado ali, e a guerra continua do mesmo ponto no próximo início ([Hospedar uma guerra](wiki:commander-hosting)).

### Uma guerra pode mudar de PC? {#can-a-war-move-to-another-pc}

Sim, como uma cópia. **Export War** no anfitrião antigo e **Importar** na aba Guerras do novo, depois os jogadores entram de novo no novo anfitrião ([Criar uma guerra](wiki:commander-wars#export-import-and-delete)).

## Compartilhar e entrar {#sharing-and-joining}

### Os jogadores que entram precisam do pacote do Commander? {#do-players-who-join-need-the-commander-package}

Não. O Obsidian basta: **Entrar em guerra**, na aba Guerras, funciona sem o pacote ([Entrar numa guerra](wiki:commander-joining)).

### Amigos pela internet podem entrar? {#can-friends-over-the-internet-join}

Sim. **Share**, depois **Internet**, e uma porta TCP redirecionada no roteador ou um túnel como o playit.gg. Os amigos digitam o endereço e o código de convite em **Em outra rede**, dentro de **Entrar em guerra**. Uma VPN como o Tailscale funciona sem nenhum dos dois e sem o código ([Compartilhar uma guerra](wiki:commander-sharing#internet)).

### Abrir uma guerra para a internet é seguro? {#is-opening-a-war-to-the-internet-safe}

Para amigos, sim. Só dados da guerra passam pela conexão, nunca arquivos do jogo nem o ticket de login da Steam, e quem chama sem o código de convite não recebe resposta. O servidor da guerra confia no ID de jogador que cada jogo informa, porém, então passe o código só para pessoas confiáveis que não vão repassá-lo, e use **New Code** se ele vazar ([O código de convite](wiki:commander-sharing#the-invite-code)).

### Uma guerra pode ficar aberta a qualquer um, com o endereço publicado? {#can-a-war-be-open-to-anyone-with-the-address-posted-in-public}

Ainda não. Um código público não protege nada, e o servidor da guerra não tem login de jogadores, nem expulsão ou banimento, nem criptografia ([O que é possível hoje](wiki:commander-sharing#what-is-possible-today)).

### Quantos jogadores uma guerra pode ter? {#how-many-players-can-a-war-have}

Até 5000 jogadores. Cada lobby de missão continua com quatro, como na guerra oficial.
