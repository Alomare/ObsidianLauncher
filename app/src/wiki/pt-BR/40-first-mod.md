---
title: Um primeiro mod
group: Criar mods
summary: Um mod que funciona, com dois arquivos, sem compilador nem outras ferramentas para instalar.
---

Um mod do Obsidian é uma pasta com um arquivo `obsidian.json` que descreve o mod, mais os arquivos que ele usa. Um mod de script precisa de um arquivo Lua. O Obsidian compila o Lua quando aplica, então um mod é texto simples, e as ferramentas do jogo nunca são necessárias.

## 1. Criar a pasta {#1-make-the-folder}

Crie uma pasta em qualquer lugar, por exemplo `hello-helldiver`, com dois arquivos:

```
hello-helldiver/
  obsidian.json
  main.lua
```

## 2. Descrever o mod {#2-describe-the-mod}

`obsidian.json`:

```json
{
  "obsidian": 1,
  "id": "author.hello-helldiver",
  "version": "1.0.0",
  "name": { "en": "Hello Helldiver" },
  "description": { "en": "Writes a line to the log when the game starts." },
  "authors": ["Author Name"],
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1
}
```

- `id` é permanente e precisa ser único. Comece com o nome do autor, para que dois autores nunca escolham o mesmo.
- `scope` é `client` porque este mod não muda nada que outros jogadores notariam ([Escopos](wiki:scopes)).
- `entry` é o script que o carregador inicia.

## 3. Escrever o script {#3-write-the-script}

`main.lua`:

```lua
obsidian.log("Hello from a first mod")

obsidian.after_startup(function()
  obsidian.log("Every mod has started")
end)
```

O carregador dá a cada mod uma tabela global chamada `obsidian` com as funções da [Loader API](wiki:loader-api).

## 4. Testar {#4-try-it}

1. Arraste a pasta para a lista de mods na aba [Mods](app:mods).
2. Ligue o mod e clique em **Aplicar**.
3. Abra o jogo e depois feche.
4. Abra `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`. `authorhello-helldiver.log` tem as duas linhas, e `loader.log` mostra o mod começando. O log de um mod tem o nome do id do mod, mantendo só letras, dígitos, `_` e `-`.

Para mudar o mod, edite os arquivos e importe a pasta de novo: um mod com o mesmo id substitui a cópia antiga. Aumente `version` a cada nova release compartilhada.

> [!TIP]
> Os mods que vêm com o Obsidian também são mods normais. O Obsidian Patches é um bom exemplo de mod de cliente com opções, e os arquivos estão no [repositório ObsidianPatches](https://github.com/Alomare/ObsidianPatches).

## Próximos passos {#next-steps}

- [O arquivo obsidian.json](wiki:manifest) lista todos os campos.
- [Opções e teclas](wiki:options-and-keys) deixa os jogadores configurarem o mod no app.
- [Ganchos](wiki:loader-api#hooks) são como um mod muda o que o jogo faz.
- [Escrever mods de sessão](wiki:session-mods) importa assim que o mod muda o jogo compartilhado.
