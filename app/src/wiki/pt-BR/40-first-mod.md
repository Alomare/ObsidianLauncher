---
title: O seu primeiro mod
group: Criar mods
summary: Um mod funcionando com dois arquivos, sem compilador nem outra ferramenta para instalar.
---

Um mod do Obsidian é uma pasta com um arquivo `obsidian.json`, que descreve o mod, e os arquivos que ele usa. Um mod de script precisa de um arquivo Lua. O Obsidian compila o Lua quando aplica, então você escreve texto puro e nunca mexe nas ferramentas do próprio jogo.

## 1. Crie a pasta {#1-make-the-folder}

Crie uma pasta em qualquer lugar, por exemplo `ola-helldiver`, com dois arquivos dentro:

```
ola-helldiver/
  obsidian.json
  main.lua
```

## 2. Descreva o mod {#2-describe-the-mod}

`obsidian.json`:

```json
{
  "obsidian": 1,
  "id": "seunome.ola-helldiver",
  "version": "1.0.0",
  "name": { "en": "Hello Helldiver", "pt-BR": "Olá Helldiver" },
  "description": { "en": "Writes a line to the log when the game starts.", "pt-BR": "Escreve uma linha no log quando o jogo abre." },
  "authors": ["Seu nome"],
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1
}
```

- `id` é permanente e precisa ser único. Comece pelo seu nome, para que dois autores nunca escolham o mesmo.
- `scope` é `client` porque este mod não muda nada que outros jogadores perceberiam ([Escopos](wiki:scopes)).
- `entry` é o script que o loader inicia.
- O nome em inglês (`en`) é obrigatório; os outros idiomas são opcionais.

## 3. Escreva o script {#3-write-the-script}

`main.lua`:

```lua
obsidian.log("Olá do meu primeiro mod")

obsidian.after_startup(function()
  obsidian.log("Todos os mods iniciaram")
end)
```

O loader dá a cada mod uma tabela global chamada `obsidian`, com as funções da [API do loader](wiki:loader-api).

## 4. Teste {#4-try-it}

1. Arraste a pasta para a lista de mods na aba [Mods](app:mods).
2. Ligue o mod e clique em **Aplicar**.
3. Abra o jogo e depois feche.
4. Abra `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`. `seunomeola-helldiver.log` tem as suas duas linhas, e o `loader.log` mostra o mod iniciando. O log de um mod tem o nome do id do mod, só com letras, dígitos, `_` e `-`.

Para mudar o mod, edite os arquivos e importe a pasta de novo: um mod com o mesmo id substitui a cópia antiga. Aumente o `version` quando compartilhar uma versão nova.

> [!TIP]
> Os mods que vêm com o Obsidian também são mods normais. O Obsidian Patches é um bom exemplo de mod de cliente com opções; os arquivos estão no [repositório dele](https://github.com/Alomare/ObsidianPatches).

## Próximos passos {#next-steps}

- [O arquivo obsidian.json](wiki:manifest) lista cada campo.
- [Opções e teclas](wiki:options-and-keys) deixa os jogadores configurarem o seu mod no app.
- [Hooks](wiki:loader-api#hooks) são o jeito de um mod mudar o que o jogo faz.
- [Escrever mods de sessão](wiki:session-mods) importa assim que o seu mod muda o jogo compartilhado.
