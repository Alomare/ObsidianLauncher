---
title: Mods
group: Conheça as abas
summary: A biblioteca de mods, a ordem de carregamento e os detalhes de cada mod.
---

A aba [Mods](app:mods) é a biblioteca: todos os mods que o Obsidian conhece, na ordem de carregamento, e quais o perfil ativo tem ligados.

## A fileira de cima {#the-top-row}

| Controle | O que faz |
|---|---|
| Botão do perfil | Mostra o [perfil](wiki:profiles) ativo, e troca para outro ou abre **Gerenciar perfis** |
| Ícone **Ordenar mods** | Ordena a lista pelo que os mods pedem ([Ordenar mods](wiki:load-order#sort-mods)) |
| Ícone **Procurar atualizações de todos** | Pergunta ao GitHub por novas versões de cada mod que indica um repositório ([Atualizar](wiki:adding-mods#updating)) |
| Ícone **Criar pasta** | Cria uma pasta na lista ([Pastas](wiki:load-order#folders)) |
| **Adicionar** | Importa um `.zip` ou uma pasta, ou abre a [Loja](app:store) |
| Botão dourado com um número | Aparece quando há atualizações, e instala todas |
| Caixa de busca | Mostra só os mods cujo nome, descrição, id ou autor contém o texto digitado |
| Ícone **Limpar** | Tira do jogo todos os arquivos do Obsidian ([Limpar](wiki:deploy-and-purge#purge)) |
| Ícone **Aplicar** | Coloca os mods ligados no jogo, e fica verde enquanto há mudanças pendentes ([Aplicar](wiki:deploy-and-purge#deploy)) |

## A lista {#the-list}

Cada linha tem uma caixa de marcar, o lugar na ordem de carregamento, um interruptor, o mod, o tamanho, o que a última aplicação pôs no jogo, a versão e o [escopo](wiki:scopes). Um clique com o botão direito no cabeçalho mostra ou esconde as colunas **#**, **Tamanho**, **Aplicado** e **Versão**.

- O interruptor liga ou desliga um mod no [perfil](wiki:profiles) ativo. Nada chega ao jogo até **Aplicar**.
- As linhas podem ser arrastadas para outro lugar na [ordem de carregamento](wiki:load-order), e pastas as agrupam.
- Um ícone de alerta ao lado de um nome quer dizer que o mod muda os mesmos arquivos que outro mod; um clique mostra qual.
- Uma versão dourada quer dizer que uma mais nova foi encontrada; um clique a instala.
- Mods marcados ganham uma barra embaixo para ligar ou desligar, procurar atualizações ou remover. O interruptor de um mod marcado liga ou desliga todos os mods marcados.

Teclas na lista, fora da caixa de busca e sem janela ou menu aberto:

| Tecla | O que faz |
|---|---|
| Ctrl+A | Marca todos os mods que a lista mostra, com a busca aplicada; de novo, desmarca |
| Ctrl+C | Copia os mods marcados, ou o aberto, como uma lista de nomes e versões |
| Ctrl+V | Marca os mods que uma lista colada cita, por nome ou id; caminhos de arquivo colados importam esses mods |
| Ctrl+F | Vai para a caixa de busca |
| Alt+Cima, Alt+Baixo | Move o mod ou a pasta em foco |

Um clique com o botão direito numa linha ou pasta abre um menu com todo o resto.

Mods também podem ser soltos na lista. [Adicionar mods](wiki:adding-mods) cobre os formatos que o Obsidian lê.

## Os detalhes de um mod {#the-details-of-a-mod}

Um clique num mod abre os detalhes à direita:

| Aba | O que mostra |
|---|---|
| Sobre | A descrição, os autores e os links |
| Versões | As releases no GitHub, por canal, e a troca para qualquer uma delas |
| Opções | As configurações do mod, salvas no perfil. Alguns mods também mostram configurações no jogo, em **Mods** no menu de pausa, e um valor mudado ali volta para esta aba depois que o jogo fecha |
| Teclas | As teclas e entradas de controle do mod. **Mudar atalhos** abre uma tabela para definir uma nova tecla ou botão do mouse, uma entrada de controle (de um a quatro botões segurados juntos) e um modo de entrada para cada uma |
| Arquivos | Os arquivos do mod e os conflitos com outros mods |
| Dependências | Os mods de que este precisa e os mods que precisam dele |
