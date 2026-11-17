---
title: Compartilhar uma guerra
group: Hospedar e compartilhar
summary: LAN, VPN, redirecionamento de porta e túneis, o código de convite, e o que é possível hoje para uma guerra compartilhada fora da rede de casa.
---

## Formas de compartilhar {#ways-to-share}

Só os dados da guerra passam pela rede: o estado do mapa uma vez por minuto, um relatório depois de cada missão e as telas da guerra. O tráfego do jogo entre os jogadores continua passando pela Steam como sempre, então compartilhar uma guerra não muda nada nos lobbies nem na qualidade da conexão.

| Forma | Para | Preparação no anfitrião | Os jogadores digitam |
|---|---|---|---|
| LAN | Jogadores na mesma rede de casa ou local | **LAN** em Share | Nada: a guerra aparece em **Entrar em guerra** |
| VPN | Amigos numa VPN como Tailscale ou ZeroTier | **LAN** em Share, todos na mesma VPN | O endereço do anfitrião na VPN, com a porta |
| Internet | Amigos em qualquer lugar | **Internet** em Share, uma porta redirecionada no roteador | O endereço público do anfitrião com a porta, e o código de convite |
| Túnel | Amigos em qualquer lugar, quando o roteador não consegue redirecionar uma porta | **Internet** em Share, um programa de túnel como playit.gg ou ngrok no PC do anfitrião | O endereço do túnel e o código de convite |

## LAN {#lan}

**Share**, na tela da guerra, depois **LAN**. O servidor da guerra começa a escutar na rede, na porta 17494 ou na próxima livre, e anuncia a guerra a cada poucos segundos. O Obsidian em qualquer PC da mesma rede a lista em **Entrar em guerra**, com o nome do computador do anfitrião, a temporada e o número de jogadores. Jogadores da rede não precisam de código.

Algumas redes impedem os dispositivos de se verem, como redes Wi-Fi de convidados e algumas redes mesh ou de escritório. Nelas o anúncio não chega, e os jogadores digitam o endereço local do anfitrião, em **Em outra rede**, dentro de **Entrar em guerra**: o endereço IPv4 que `ipconfig` mostra no PC do anfitrião, depois dois-pontos e a porta que Share mostra, como `192.168.1.20:17494`.

## VPN {#vpn}

Uma VPN como Tailscale ou ZeroTier põe os PCs dos amigos numa rede privada pela internet. O servidor da guerra trata jogadores nas faixas de endereço comuns de VPN como locais, então **LAN** basta: nenhuma porta para redirecionar e nenhum código de convite. O anúncio não atravessa a maioria das VPNs, então os jogadores digitam o endereço do anfitrião na VPN e a porta em **Em outra rede**, dentro de **Entrar em guerra**. Uma VPN é a escolha mais segura para jogar pela internet, já que só pessoas convidadas para a VPN conseguem alcançar a guerra.

## Internet {#internet}

**Share**, depois **Internet**. Share pergunta mais uma vez antes de abrir, já que a guerra passa a responder a quem vier de qualquer lugar com o código, e abrir para a internet também abre a LAN. Então o anfitrião:

1. Redireciona no roteador a porta TCP mostrada em Share para o PC do anfitrião. Os menus de roteador chamam isso de redirecionamento de porta ou servidor virtual.
2. Descobre o endereço público da conexão, na página de status do roteador ou em qualquer site de "qual é meu IP", aberto no PC do anfitrião.
3. Manda aos amigos o endereço com a porta, como `203.0.113.7:17494`, e o código de convite de Share.

Os amigos abrem **Entrar em guerra** no Obsidian, digitam o endereço e o código em **Em outra rede**, apertam **Encontrar guerras** e entram. O código fica guardado com a guerra naquele PC, então é digitado uma vez só.

Muitas conexões domésticas mudam o endereço público de tempos em tempos. Quando isso acontece, os amigos entram de novo com o novo endereço. Alguns provedores põem muitas casas atrás de um único endereço compartilhado (chamado CGNAT), e aí nenhum redirecionamento de porta chega ao PC do anfitrião. Um túnel ou uma VPN funciona nesse caso.

## Túnel {#tunnel}

Um serviço de túnel dá à guerra um endereço público sem mexer no roteador. Um programa do serviço roda no PC do anfitrião e repassa cada chamada a esse endereço para o servidor da guerra, então o roteador e o provedor não participam. playit.gg e ngrok são dois serviços assim, e qualquer serviço com túneis TCP funciona.

1. Ligue **Internet** em Share, como para um redirecionamento de porta.
2. No programa de túnel, crie um túnel TCP para `127.0.0.1` e a porta que Share mostra.
3. Mande aos amigos o endereço que o serviço dá, como `example.at.ply.gg:12345`, e o código de convite.

Os amigos digitam esse endereço e o código em **Em outra rede**, dentro de **Entrar em guerra**. Toda chamada por um túnel conta como chamada de fora, então o código é exigido mesmo para um amigo na mesma rede. O programa de túnel precisa estar rodando enquanto a guerra é jogada, e em alguns serviços o endereço muda cada vez que o programa reinicia.

## O código de convite {#the-invite-code}

Uma guerra aberta para a internet só responde a quem vem de fora da rede com o código, que tem a forma `K7M2P-9XQ4T`. Sem ele, ou com um errado, a guerra parece não existir. A guerra recebe um código na primeira vez que abre para a internet e o mantém entre reinícios. **Copy Code**, em Share, copia o código, e **New Code** o substitui, o que tranca para fora todos que têm o antigo até eles entrarem de novo com o novo código.

## O que é possível hoje {#what-is-possible-today}

Compartilhar uma guerra com amigos funciona hoje, numa LAN, por uma VPN, ou pela internet por um redirecionamento de porta ou um túnel com o código de convite. Os jogadores que entram veem as telas da guerra do anfitrião em modo somente leitura e jogam a guerra, e toda missão que eles concluem conta.

Uma guerra aberta que qualquer um pode encontrar e entrar ainda não é suportada. Publicar o endereço e o código transforma o código numa senha pública, e o servidor da guerra não tem defesa além dela:

- O servidor da guerra aceita sem prova o ID de jogador que cada jogo informa, então um estranho com o código poderia relatar missões com qualquer nome.
- O anfitrião não pode expulsar nem banir um jogador específico. **New Code** é a única tranca, e ela tranca todos para fora.
- A conexão é HTTP simples, legível no caminho.
- O servidor da guerra roda dentro do Obsidian no PC de um jogador. Ele não foi feito para rodar sozinho num servidor alugado.

Uma guerra comporta até 5000 jogadores. Os lobbies no jogo continuam com quatro jogadores, como sempre.
