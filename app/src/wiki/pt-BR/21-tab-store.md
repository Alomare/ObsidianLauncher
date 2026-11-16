---
title: Loja
group: Conheça as abas
summary: Mods prontos para instalar com um clique, e como colocar um mod na Loja.
---

A [Loja](app:store) lista mods que se instalam com um clique, os mais novos primeiro. O **Obsidian Kit** fica numa fileira no topo: Obsidian API, Obsidian Patches e Obsidian Trainer, os mods em torno dos quais o Obsidian foi feito.

## Encontrar um mod {#finding-a-mod}

A fileira acima dos cards filtra por [escopo](wiki:scopes), ordena a lista e mostra só os mods instalados. A caixa de busca procura em nomes e descrições.

Cada card mostra a imagem, o nome, uma descrição curta e as etiquetas:

| Etiqueta | Significado |
|---|---|
| Quadrado prateado | **Verificado pelo Obsidian**: o mod é feito e mantido pela equipe do Obsidian, ou por alguém que ganhou a confiança dela |
| Quadrado colorido | O [escopo](wiki:scopes) do mod |
| **EA** (acesso antecipado) | Uma versão abaixo de 1.0, ainda em desenvolvimento, em que versões futuras podem mudar muito. Mods em repositórios privados também são de acesso antecipado e só aparecem para uma conta do GitHub que pode lê-los ([Conta](wiki:tab-account)) |

Um canto verde marca um mod instalado, e um canto amarelo um mod com versão mais nova na Loja.

## A página de um mod {#the-page-of-a-mod}

Um clique num card abre a página do mod: a descrição, as versões com o que cada uma mudou, as opções, os mods de que ele precisa ou que precisam dele, e os problemas abertos no GitHub. Os painéis à direita abrem com um clique no título.

**Instalar** baixa do GitHub a versão mais nova, com todos os mods de que ela precisa, para a biblioteca. O mod chega desligado: ligue-o na aba [Mods](app:mods) e aplique. **Atualizar** troca uma cópia instalada pela versão mais nova, e **Desinstalar** remove o mod da biblioteca.

Com login no GitHub ([Conta](wiki:tab-account)), a página de um mod também pode:

- dar estrela no mod no GitHub;
- **Relatar um problema**: um formulário que abre uma issue no repositório do mod, com a conta do GitHub, com o relatório de problema do Obsidian anexado se desejado. As respostas aparecem na aba Relatos da [Conta](app:account).

## Colocar um mod na Loja {#adding-a-mod-to-the-store}

A Loja lê uma lista pública, o [Obsidian Store Index](https://github.com/Alomare/ObsidianStoreIndex). Um mod entra quando o repositório dele está nessa lista:

1. Publique o mod num repositório público do GitHub, com `obsidian.json` na raiz (veja [O manifesto](wiki:manifest)).
2. Faça releases com a tag `v` mais a versão, como `v1.0.0`, com o zip do mod anexado. A Loja instala a release estável mais nova.
3. Se quiser, adicione `thumbnail.jpg` (16:9) na raiz do repositório e um ícone indicado por `image` no manifesto. O README vira a descrição na página do mod.
4. Clique em **Enviar mod** na Loja, com login na conta do GitHub dona do repositório. O Obsidian confere o repositório, lista qualquer problema e então abre um pull request no Store Index para revisão. Um pull request ou uma issue aberta à mão no Store Index também serve.

O índice é refeito a cada hora a partir das releases, então uma release nova aparece na Loja em até uma hora. O Store Index dá o Verificado pelo Obsidian às contas do GitHub em que confia: a etiqueta segue a conta dona do repositório, nunca os autores que um manifesto cita.
