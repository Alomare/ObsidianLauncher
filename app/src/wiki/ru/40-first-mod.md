---
title: Первый мод
group: Создание модов
summary: Рабочий мод из двух файлов, без компилятора и других инструментов.
---

Мод для Obsidian представляет собой папку с файлом `obsidian.json`, который описывает мод, и файлами, которые он использует. Скриптовому моду нужен один файл Lua. Obsidian компилирует Lua при применении, поэтому мод состоит из обычного текста, и инструменты разработки игры не нужны.

## 1. Создайте папку {#1-make-the-folder}

Создайте где угодно папку, например `hello-helldiver`, с двумя файлами:

```
hello-helldiver/
  obsidian.json
  main.lua
```

## 2. Опишите мод {#2-describe-the-mod}

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

- `id` постоянен и должен быть уникальным. Начинайте его с имени автора, чтобы два автора никогда не выбрали одинаковый.
- `scope` равен `client`, потому что мод не меняет ничего, что заметили бы другие игроки ([Области](wiki:scopes)).
- `entry` является скриптом, который запускает загрузчик.

## 3. Напишите скрипт {#3-write-the-script}

`main.lua`:

```lua
obsidian.log("Hello from a first mod")

obsidian.after_startup(function()
  obsidian.log("Every mod has started")
end)
```

Загрузчик даёт каждому моду глобальную таблицу `obsidian` с функциями [Loader API](wiki:loader-api).

## 4. Проверьте {#4-try-it}

1. Перетащите папку на список модов во вкладке [Моды](app:mods).
2. Включите мод и нажмите **Применить**.
3. Запустите игру, затем закройте её.
4. Откройте `%APPDATA%\Arrowhead\Helldivers\ObsidianLauncher\logs`. В `authorhello-helldiver.log` будут две строки, а `loader.log` покажет запуск мода. Журнал мода называется по id мода, в котором оставлены только буквы, цифры, `_` и `-`.

Чтобы изменить мод, отредактируйте файлы и импортируйте папку снова: мод с тем же id заменит старую копию. Увеличивайте `version` для каждого нового опубликованного релиза.

> [!TIP]
> Моды, которые поставляются с Obsidian, тоже обычные моды. Obsidian Patches является хорошим примером клиентского мода с параметрами, а его файлы лежат в [репозитории ObsidianPatches](https://github.com/Alomare/ObsidianPatches).

## Что дальше {#next-steps}

- [Файл obsidian.json](wiki:manifest) перечисляет все поля.
- [Параметры и клавиши](wiki:options-and-keys) позволяют игрокам настраивать мод в приложении.
- [Хуки](wiki:loader-api#hooks) являются способом, которым мод меняет поведение игры.
- [Как писать сессионные моды](wiki:session-mods) становится важным, как только мод меняет общую игру.
