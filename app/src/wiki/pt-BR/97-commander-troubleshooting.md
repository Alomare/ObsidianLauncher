---
title: Solução de problemas
group: Ajuda
summary: Guerras que não aparecem, endereços que não respondem, o jogo na guerra errada, logs e remover o Commander.
---

## Nenhuma guerra em Entrar em guerra {#no-war-under-join-war}

Confira estes pontos primeiro no PC do anfitrião. **Share**, na tela da guerra, mostra **LAN** ligado: o interruptor desliga sempre que o Obsidian fecha, então precisa ser ligado depois de cada início. O firewall do Windows deixou o servidor da guerra passar em redes privadas, e o Windows trata a rede como privada. Os dois PCs estão na mesma rede, e não um deles num Wi-Fi de convidados.

Quando tudo isso está certo e a lista continua vazia, a rede pode estar impedindo os dispositivos de se verem. Digite o endereço local do anfitrião em **Em outra rede**, dentro de **Entrar em guerra**, como `192.168.1.20:17494`, usando o endereço IPv4 que `ipconfig` mostra no PC do anfitrião e a porta que Share mostra ([LAN](wiki:commander-sharing#lan)).

Um aviso do firewall respondido com Cancelar bloqueia o servidor da guerra até a regra mudar: na Segurança do Windows, abra **Firewall e proteção de rede**, depois **Permitir um aplicativo pelo firewall**, e marque **Privada** para `commander-server.exe`.

## "Nenhuma guerra respondeu nesse endereço" {#no-war-answered-at-that-address}

**Encontrar guerras**, em **Em outra rede** dentro de **Entrar em guerra**, não trouxe nenhuma guerra. No lado do anfitrião, confira se **Internet** está ligado em Share (ele também desliga quando o Obsidian fecha), se o roteador redireciona para o PC do anfitrião a porta TCP que Share mostra, e se o endereço é o público atual. No lado do jogador, confira a porta depois dos dois-pontos e o código de convite: depois de **New Code**, só o código novo funciona.

O redirecionamento de porta falha sem erro quando o provedor divide um endereço público entre muitas casas (CGNAT). O endereço público na página de status do roteador então difere do que um site de "qual é meu IP" mostra. Um [túnel](wiki:commander-sharing#tunnel) ou uma [VPN](wiki:commander-sharing#vpn) funciona nessas conexões. Por um túnel, confira se o programa de túnel está rodando no PC do anfitrião, aponta para `127.0.0.1` e a porta que Share mostra, e ainda tem o endereço que os amigos digitaram.

## O jogo mostra a guerra como offline {#the-game-shows-the-war-as-offline}

O jogo alcança a guerra pelo Obsidian no mesmo PC, que então alcança o anfitrião. Confira se o Obsidian está aberto neste PC e no PC do anfitrião, e se a guerra ainda responde: a aba da guerra em que se entrou, na aba Guerras, mostra as telas da guerra do anfitrião enquanto ela responde. Depois de um **New Code** no anfitrião, entre de novo com o novo código.

## O jogo joga a guerra oficial {#the-game-plays-the-official-war}

O jogo só joga uma guerra personalizada quando a guerra está selecionada, um save Sandbox que a guerra aceita é o save ativo, e o carregador do Obsidian está no jogo. Abra o jogo com **Jogar** no Obsidian, depois que a guerra mostrar **Selected**. Um jogo aberto pela Steam sem nada aplicado, ou aberto antes de a guerra ser selecionada, joga a guerra oficial no save Oficial.

## O pacote não instala {#the-package-won-t-install}

Um pacote baixado só instala com uma assinatura válida. Quando o download falha na verificação, tente **Atualizar pacote** de novo mais tarde, e relate no GitHub se continuar falhando. Um build local instala com **Instalar de arquivo**.

## Logs {#logs}

O servidor da guerra escreve `%LOCALAPPDATA%\ObsidianLauncher\commander\server\logs\server.log` no PC do anfitrião. As linhas de guerra do jogo, em todo PC de jogador, ficam em `loader.log`, com os outros logs listados na [Solução de problemas](wiki:troubleshooting#logs) da Wiki do Launcher. **Copiar relatório**, em **Relatório de problema** na aba Sistema das [Configurações](app:settings), reúne as versões e o fim do `loader.log` para um relato de bug.

## Remover o Commander {#removing-commander}

**Remover todos os dados do Commander**, no fim do menu do pacote, remove o pacote, as guerras hospedadas e as em que se entrou, os dados do servidor da guerra e o arquivo de nomes de regiões. A janela lista primeiro o que encontrou. Apagar os saves de guerra é uma escolha ali, e remover as regras do Firewall do Windows para o servidor da guerra também, o que exige permissão de administrador e abre um aviso do Windows. O servidor da guerra para primeiro, e o próximo início joga a guerra oficial.

Exporte as guerras que quiser guardar antes de remover, já que as guerras não podem voltar depois. **Remover pacote**, no mesmo menu, leva só o servidor da guerra e as telas da guerra, e mantém as guerras e os saves de guerra.
