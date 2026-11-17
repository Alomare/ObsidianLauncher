---
title: Conta
group: Conheça as abas
summary: O login do GitHub, os acessos que o Obsidian tem, relatos de problemas e sincronização entre PCs.
---

A aba [Conta](app:account), o ícone de pessoa na ponta direita da barra de cima, liga o Obsidian a uma conta do GitHub. Todo o resto do Obsidian funciona sem uma. Uma marca de confirmação no ícone quer dizer que há uma conta conectada.

## GitHub {#github}

**Entrar com o GitHub** mostra um código e abre a página de login do GitHub. Digite o código ali para terminar. O Obsidian nunca vê a senha do GitHub.

A lista **Acesso** mostra o que o GitHub permite que o Obsidian faça, e **Permitir** pede ao GitHub mais um tipo de acesso:

| Acesso | Para que serve |
|---|---|
| Acesso a repositórios públicos | Relatar problemas, responder a relatos, dar estrela em mods e enviar um mod para a Loja |
| Acesso a Gists | Sincronizar entre PCs |
| Acesso a repositórios privados | Mods de acesso antecipado em repositórios privados que a conta pode ler. O GitHub concede isso como acesso de leitura e escrita a todos os repositórios que a conta alcança, então o Obsidian só o pede quando permitido aqui |

**Sair** faz o Obsidian esquecer a conta neste PC. O acesso continua concedido no GitHub até **Revogar no GitHub** removê-lo lá.

## Relatos {#reports}

Os problemas enviados com **Relatar um problema** na página de um mod na [Loja](wiki:tab-store) são listados aqui, com o mod, o estado (aberto, fechado ou corrigido) e a data. Um número marca respostas novas. Um clique num relato mostra a conversa, onde uma resposta pode ser escrita e enviada. **Abrir no GitHub** abre a issue no navegador, e **Remover da lista** a tira desta lista sem fechá-la no GitHub.

## Sincronizar {#sync}

A sincronização mantém os perfis e as configurações de aparência iguais em vários PCs. Ela os guarda num gist secreto na conta do GitHub, o que exige acesso a Gists.

- **Enviar** salva no gist os perfis e as configurações de aparência deste PC.
- **Buscar** os lê em outro PC. Perfis com o mesmo nome são substituídos, e mods que os perfis usam e faltam na biblioteca são instalados dos repositórios deles no GitHub. Mods sem repositório precisam ser adicionados à mão, e o Obsidian diz quais são.
- **Abrir o gist** mostra o arquivo guardado no GitHub.
