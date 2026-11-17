---
title: Compartilhar um mod
group: Criar mods
summary: Empacotar uma release, versões, imagens e traduções.
---

## Empacotar {#packing}

Compacte a pasta do mod de modo que `obsidian.json` fique no topo do arquivo, ou dentro de uma pasta no topo. Os jogadores soltam o `.zip` na lista de mods, e essa é a instalação inteira.

Deixe de fora os arquivos que o mod não usa. Mantenha os caminhos em `obsidian.json` relativos à pasta do mod, com barras normais.

## Versões {#versions}

Use um número de versão como `1.4.2`:

- aumente o último número para correções;
- aumente o do meio para novos recursos e opções;
- aumente o primeiro quando o mod mudar de um jeito que os jogadores gostariam de saber antes de atualizar.

Um jogador que importa uma versão maior mantém os valores de opção e as teclas, exceto em opções que mudaram de tipo. Mantenha os ids de opções e teclas iguais entre releases.

> [!WARNING]
> Nunca mude o `id` de um mod já lançado. O Obsidian veria um mod diferente, e os jogadores acabariam com os dois.

## Atualizações {#updates}

Com um repositório do GitHub indicado em `obsidian.json`, os jogadores atualizam pelo app:

```json
"source": "https://github.com/author/mod"
```

Para cada nova versão:

1. Aumente `version` em `obsidian.json`.
2. Faça uma release no GitHub cuja tag seja essa versão, como `v1.4.2`.
3. Anexe o `.zip` à release. Se a release tiver vários, ponha o id do mod no nome do certo, como `author.mod-name-1.4.2.zip`.

O Obsidian oferece só releases estáveis como atualizações, e ignora rascunhos. Ele recusa um `.zip` com outro id ou com uma versão que não é maior.

### Canais {#channels}

Uma versão de pré-lançamento, como `1.5.0-beta.1`, pertence a um canal com o nome da primeira palavra da parte de pré-lançamento: `beta` aqui. A aba Versões nos detalhes do mod lista uma aba por canal, ao lado de **Estável**, e instala qualquer versão a partir dali. Uma release marcada como pré-lançamento no GitHub com uma versão simples vai para um canal chamado `pre`. O canal `ci` fica escondido, para builds de teste feitos por workflows.

Um mod num repositório privado, listado no Store Index, é de acesso antecipado: ele só aparece na Loja para jogadores cuja conta do GitHub pode ler o repositório e que permitiram acesso a repositórios privados ([Conta](wiki:tab-account)).

## Imagens {#pictures}

Adicione uma `image.png` quadrada (ou JPG ou WebP, de até 4 MB) e cite-a em `obsidian.json` com `"image": "image.png"`. 512 por 512 pixels é mais que suficiente.

## Traduções {#translations}

- Ponha o nome, a descrição e os rótulos das opções em todos os idiomas disponíveis, em `name`, `description` e `label`.
- Ponha o texto que o mod mostra no jogo em `locales/<idioma>.json`.
- O inglês é sempre a reserva, então uma tradução que falta mostra inglês em vez de um espaço vazio.

## Antes de uma release {#before-you-release}

1. Importe o próprio `.zip` no Obsidian, não a pasta, para testar o que os jogadores recebem.
2. Aplique, jogue uma missão e leia o log do mod.
3. Para um mod de sessão, teste com um amigo que não o roda: o mod deve desligar ([Escrever mods de sessão](wiki:session-mods)).
4. Escreva na descrição o que o mod muda e se os outros jogadores notam.
