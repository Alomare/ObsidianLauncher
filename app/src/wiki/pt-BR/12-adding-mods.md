---
title: Adicionar e atualizar mods
group: Primeiros passos
summary: Importar um mod de um .zip ou de uma pasta, atualizar e remover.
---

## Importar {#importing}

Arraste uma pasta de mod ou um arquivo `.zip` para a lista de mods na aba [Mods](app:mods), ou clique no **+** verde ao lado de Criar pasta e escolha um. Dá para importar vários de uma vez, e uma notificação diz o que aconteceu com cada um.

O Obsidian copia o mod para a biblioteca, então você pode apagar o download depois. Importar não muda nada no jogo até você aplicar.

### Mods feitos para o Obsidian {#mods-made-for-obsidian}

Um mod feito para o Obsidian tem um arquivo `obsidian.json` no topo da pasta ou do arquivo compactado. O Obsidian lê dele o nome, a versão, as opções e as teclas, e mostra tudo no painel da direita.

### Mods mais antigos {#older-mods}

A maioria dos mods de HD1 que circulam hoje substitui bundles inteiros do jogo, com arquivos de nomes como `9e13b2414b41b842`. O Obsidian importa esses também. Ele compara cada bundle do mod com o do seu jogo e guarda só os recursos que o mod de fato altera, então dois mods antigos que mexem em coisas diferentes agora rodam juntos.

Alguns mods antigos trazem arquivos `.patch_0`, `.patch_1`, com nomes como `9e13b2414b41b842.patch_0`, que o jogo lê por cima do próprio bundle. O Obsidian importa esses também, então não é preciso renomeá-los à mão para o próximo número livre. Um mod que é só um preset do ReShade (um `ReShadePreset.ini`) não é um mod do jogo: ele precisa do ReShade na pasta do jogo, e a importação avisa isso.

Mods antigos entram com o escopo **não especificado**, porque o Obsidian não tem como saber o que eles mudam num jogo compartilhado. Eles rodam como mods de cliente, então nunca tiram você de um lobby ([Escopos](wiki:scopes)). Um mod antigo que só muda bancos de som entra como mod de **cliente**, já que som muda só o que você ouve. Eles também não podem ser desligados no meio de uma missão.

Um patch para outro mod não traz esse mod junto. Importe os dois e deixe o patch mais abaixo na lista. Confira para qual versão o patch foi feito: os arquivos que os dois têm em comum vêm do patch, então um patch para uma versão antiga volta esses arquivos para aquela versão.

> [!NOTE]
> Para comparar os bundles, os arquivos do jogo precisam ser os originais, então a importação para se não forem. Se você copiou um mod para a pasta do jogo à mão, verifique os arquivos do jogo na Steam e importe de novo.

## Atualizar {#updating}

Importe a versão nova do mesmo jeito. Um mod com o mesmo id substitui o que está na biblioteca, e a notificação mostra a versão antiga e a nova. Os valores das opções e as teclas continuam, exceto em opções cujo tipo o autor mudou.

### Pelo GitHub {#from-github}

Um mod pode indicar o repositório dele no GitHub. Quando indica, aparece um botão com duas setas na linha dele na aba [Mods](app:mods), entre o interruptor e o nome, que pergunta ao GitHub se há uma versão nova daquele mod. O mesmo botão numa pasta pergunta por todos os mods dela, e o do topo da lista pergunta por todos. Um mod com versão nova mostra um botão **Atualizar para** ao lado do nome. Clique nele e depois aplique ao jogo.

O Obsidian fala com o GitHub ao abrir e quando você clica, e só oferece uma versão mais alta do mesmo mod. Nada é instalado até você escolher. Um mod com versão nova mostra o número da versão numa caixa amarela com uma seta. Clique nela para ver a versão nova e um link para as novidades do mod, e depois confirme. **Atualizar todos** instala de uma vez todas as atualizações pendentes.

A aba Sobre, nos detalhes de um mod assim, lista o que mudou em cada versão, em **Notas de versão**. Clique numa versão para ler as mudanças; **Carregar mais** traz as mais antigas.

## Mods verificados {#verified-mods}

Uma marca redonda com um visto depois do nome de um mod quer dizer que o mod é confiável e recomendado pela equipe do Obsidian. A marca só aparece quando o mod vem do repositório do GitHub que a equipe recomendou, então uma cópia com o mesmo id vinda de outro lugar fica sem ela. Os mods que vêm com o Obsidian e o Better HUD têm a marca. A marca não muda mais nada: mods verificados são aplicados e rodam como qualquer outro.

## Remover {#removing}

Clique com o botão direito num mod e escolha **Remover**, ou use a lixeira abaixo do botão de fechar nos detalhes do mod. Para remover vários, marque os mods e use **Remover** na barra no pé da lista.

## Obsidian Kit

Três mods vêm com o Obsidian, na pasta **Obsidian Kit**, no topo da lista: Obsidian API, [Obsidian Trainer](wiki:trainer) e Obsidian Patches. O Obsidian Patches começa ligado, com a maioria das correções ativas. Os outros dois ficam desligados até você ligar.

Eles são mods como quaisquer outros: você pode ligar, desligar, remover e atualizar os mods pelo GitHub. A pasta começa trancada, então fica onde está. Clique no cadeado para destrancar, e aí você pode mover os mods e renomear ou apagar a pasta. Para deixar tudo como era, use **Restaurar Obsidian Kit** em [Configurações](app:settings). Isso traz de volta os mods apagados e a pasta, e volta as configurações deles ao padrão no perfil ativo.

## Dependências {#dependencies}

Alguns mods precisam de outros para funcionar. A aba **Dependências**, no painel à direita, mostra do que um mod precisa e quais mods precisam dele. Ligar um mod liga o que ele precisa, e desligar desliga os mods que precisam dele.

Se um mod precisa de algo que não está na sua biblioteca, a aba avisa, e o mod não liga até você adicionar o que falta.

## Imagens {#pictures}

Um mod pode trazer uma imagem quadrada. Para usar uma sua, abra o mod no painel da direita e clique na imagem. Perfis aceitam imagem do mesmo jeito.
