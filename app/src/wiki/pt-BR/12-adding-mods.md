---
title: Adicionar e atualizar mods
group: Primeiros passos
summary: Importar um mod de um .zip ou de uma pasta, atualizar e remover.
---

## Importar {#importing}

Arraste a pasta de um mod ou um arquivo `.zip` para a lista de mods na aba [Mods](app:mods), ou clique no botão verde **Adicionar** acima da lista e escolha **Importar .zip** ou **Importar pasta**. Vários arquivos podem entrar de uma vez, e uma notificação diz o que aconteceu com cada um. A [Loja](app:store) instala mods sem download manual.

O Obsidian copia o mod para a biblioteca, então o download pode ser apagado depois. Importar não muda nada no jogo até a próxima aplicação.

### Mods feitos para o Obsidian {#mods-made-for-obsidian}

Um mod feito para o Obsidian tem um arquivo `obsidian.json` no topo da pasta ou do arquivo compactado. O Obsidian lê desse arquivo o nome, a versão, as opções e as teclas, e os mostra no painel à direita.

### Mods antigos {#older-mods}

A maioria dos mods de HD1 compartilhados hoje substitui bundles inteiros do jogo, com arquivos de nomes como `9e13b2414b41b842`. O Obsidian também importa esses mods. Ele compara cada bundle do mod com o do jogo e guarda só os recursos que o mod realmente muda, então dois mods antigos que mexem em coisas diferentes podem rodar juntos.

Alguns mods antigos trazem arquivos `.patch_0` ou `.patch_1`, com nomes como `9e13b2414b41b842.patch_0`, que o jogo lê por cima do bundle correspondente. O Obsidian também os importa, sem renomear à mão para o próximo número livre. Um mod que é só um preset do ReShade (um `ReShadePreset.ini`) não é um mod do jogo: ele precisa do ReShade na pasta do jogo, e a importação avisa.

Mods antigos entram com o escopo **não especificado**, porque o Obsidian não sabe o que eles mudam num jogo compartilhado. Eles rodam como mods de cliente, então nunca deixam um jogador fora de um lobby, e não podem ser desligados no meio de uma missão ([Escopos](wiki:scopes)). Um mod antigo que muda só bancos de som entra como mod de **cliente**, já que uma mudança de som fica num PC só.

Um patch para outro mod não traz esse mod junto. Importe os dois e ponha o patch mais abaixo na lista. Confira para qual versão o patch foi feito: os arquivos em comum vêm do patch, então um patch de uma versão antiga leva esses arquivos de volta àquela versão.

> [!NOTE]
> Comparar bundles exige os arquivos originais do jogo, então a importação para quando eles diferem. Um mod copiado à mão para a pasta do jogo causa isso: verifique os arquivos do jogo na Steam e importe de novo.

## Atualizar {#updating}

Importe a nova versão do mesmo jeito. Um mod com o mesmo id substitui o da biblioteca, e a notificação mostra o número da versão antiga e da nova. Valores de opções e teclas continuam, exceto em opções cujo tipo o autor mudou.

### Pelo GitHub {#from-github}

Um mod pode indicar um repositório do GitHub. O Obsidian pergunta ao GitHub por novas versões desses mods ao iniciar, a não ser que tenha perguntado nas últimas seis horas, e quando pedido:

- **Procurar atualizações de todos**, um ícone no topo da aba [Mods](app:mods), pergunta sobre todos esses mods.
- **Procurar atualizações**, na barra no pé da lista, pergunta sobre os mods marcados. A caixa de uma pasta marca todos os mods dela, e a do topo da lista marca todos.

Um mod com nova versão mostra a versão em dourado na coluna Versão, e **Atualizar para** nos detalhes do mod. Um clique em qualquer um mostra a nova versão e um link para o changelog, e pede confirmação. O botão dourado com um número, no topo da aba Mods, instala todas as atualizações pendentes de uma vez. Só releases estáveis com versão maior são oferecidas, e nada instala sem um clique. Aplique depois para pôr as novas versões no jogo.

A aba Versões nos detalhes de um mod assim lista todas as releases, por canal, com o que cada uma mudou, e instala qualquer uma, inclusive as mais antigas ([Canais](wiki:sharing#channels)).

## Remover {#removing}

Clique com o botão direito num mod e escolha **Remover**, ou use a lixeira abaixo do botão de fechar nos detalhes do mod. Para remover vários, marque-os e use **Remover** na barra no pé da lista.

## Obsidian Kit {#obsidian-kit}

Três mods vêm com o Obsidian, na pasta **Obsidian Kit** no topo da lista: Obsidian API, [Obsidian Trainer](wiki:trainer) e Obsidian Patches. O Obsidian Patches começa ligado, com a maioria das correções ativadas, e os outros dois ficam desligados até serem ligados.

Eles são mods como quaisquer outros: ligam e desligam, podem ser removidos e se atualizam pelo GitHub. A pasta começa trancada, então fica no lugar. Um clique no cadeado a destranca, e então os mods podem se mover e a pasta pode ser renomeada ou apagada. Um mod do Kit removido se instala de novo pela fileira Obsidian Kit no topo da [Loja](app:store).

## Dependências {#dependencies}

Alguns mods precisam de outros para funcionar. A aba **Dependências**, no painel à direita, mostra do que um mod precisa e quais mods precisam dele. Ligar um mod liga o que ele precisa, e desligá-lo desliga os mods que precisam dele. Quando um mod precisa de algo que falta na biblioteca, a aba avisa, e o mod fica desligado até esse mod ser adicionado.

## Imagens {#pictures}

Um mod pode trazer uma imagem quadrada. Para usar outra, abra o mod no painel à direita e clique na imagem. Os perfis recebem uma imagem do mesmo jeito.
