---
title: Opções e teclas
group: Criar mods
summary: Configurações que os jogadores mudam no app, e teclas que podem trocar, lidas pelo Lua.
---

As opções aparecem no painel da direita quando um jogador seleciona o seu mod. O Obsidian confere cada valor contra o que você declarou, então o mod nunca recebe um valor fora da faixa definida. Os valores pertencem ao perfil, então dois perfis podem rodar o mesmo mod com configurações diferentes.

## Declarar opções {#declaring-options}

Cada opção tem um `id` (letras minúsculas, dígitos e `_`), um `type`, um `default` e um `label`. Um `description` acrescenta o texto que aparece quando o jogador passa o mouse na opção.

| Tipo | Valor | Campos extras |
|---|---|---|
| `toggle` | `true` ou `false` | |
| `number` | um número | `min`, `max`, `step` |
| `choice` | um dos ids listados | `choices`: uma lista de ids, ou de `{ "id": "…", "label": { … } }` |
| `key` | um nome de tecla como `f8` | |
| `color` | `#rrggbb` | |

```json
"options": [
  { "id": "enabled", "type": "toggle", "default": true, "label": { "en": "Show the compass", "pt-BR": "Mostrar a bússola" } },
  { "id": "style", "type": "choice", "default": "small",
    "choices": [ { "id": "small", "label": { "en": "Small", "pt-BR": "Pequena" } }, { "id": "large", "label": { "en": "Large", "pt-BR": "Grande" } } ],
    "label": { "en": "Size", "pt-BR": "Tamanho" }, "visible_when": { "enabled": true } },
  { "id": "tint", "type": "color", "default": "#f0b429", "label": { "en": "Color", "pt-BR": "Cor" }, "scope": "client" }
]
```

### Mostrar opções só quando importam {#showing-options-only-when-they-matter}

`visible_when` esconde uma opção até que outras opções tenham os valores listados. No exemplo, **Tamanho** só aparece enquanto **Mostrar a bússola** está ligada.

### Opções pessoais em mods de sessão {#personal-options-in-session-mods}

Os valores de opção de um mod de sessão ou de uma expansão contam quando o Obsidian compara os mods de dois jogadores, porque um controle de balanceamento muda o jogo compartilhado. Marque uma opção com `"scope": "client"` quando o valor só mudar o que este jogador vê, como uma cor ou a distância da câmera. Assim os jogadores podem escolher valores diferentes e ainda bater ([Jogar online](wiki:multiplayer)).

## Ler opções no Lua {#reading-options-in-lua}

```lua
local get = obsidian.settings.get

if get("enabled") then
  local size = get("style")   -- "small" ou "large"
end
```

`obsidian.settings.get` devolve o valor que o jogador escolheu, ou o padrão. Uma mudança no app vale na próxima vez que o jogo abrir, sem aplicar de novo.

## Opções que escolhem arquivos {#options-that-pick-files}

Um recurso pode depender de uma opção com `when`. O recurso só entra no jogo enquanto as opções tiverem esses valores:

```json
"resources": [
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_small.lua", "when": { "style": "small" } },
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_large.lua", "when": { "style": "large" } }
]
```

Mudar uma opção assim exige aplicar de novo, e o app deixa o botão Aplicar verde para avisar.

## Teclas {#keys}

Declare cada tecla que o mod escuta, com um padrão:

```json
"keybinds": [ { "id": "toggle", "default": "f8", "label": { "en": "Turn the compass on or off", "pt-BR": "Ligar ou desligar a bússola" } } ]
```

Depois reaja a ela no Lua:

```lua
obsidian.keybind("toggle", function()
  obsidian.log("toggle apertado")
end)
```

A função roda uma vez a cada vez que a tecla desce. Os jogadores trocam as teclas no app, que é onde os choques entre mods se resolvem. Os nomes de tecla são os que o jogo usa, em minúsculas: `f8`, `k`, `insert`.
