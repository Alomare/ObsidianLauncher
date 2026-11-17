---
title: Opções e teclas
group: Criar mods
summary: Configurações que os jogadores mudam no app e teclas que podem remapear, lidas pelo Lua.
---

As opções aparecem no painel à direita quando um jogador seleciona o mod. O Obsidian confere cada valor com a declaração, então o mod nunca recebe um valor fora do intervalo declarado. Os valores pertencem ao perfil, então dois perfis podem rodar o mesmo mod com configurações diferentes.

## Declarar opções {#declaring-options}

Cada opção tem um `id` (letras minúsculas, dígitos e `_`), um `type`, um `default` e um `label`. Um `description` adiciona o texto que os jogadores veem ao passar o mouse sobre a opção.

| Tipo | Valor | Campos extras |
|---|---|---|
| `toggle` | `true` ou `false` | |
| `number` | um número | `min`, `max`, `step` |
| `choice` | um dos ids listados | `choices`: uma lista de ids, ou de `{ "id": "…", "label": { … } }` |
| `key` | um nome de tecla como `f8` | |
| `color` | `#rrggbb` | |

```json
"options": [
  { "id": "enabled", "type": "toggle", "default": true, "label": { "en": "Show the compass" } },
  { "id": "style", "type": "choice", "default": "small",
    "choices": [ { "id": "small", "label": { "en": "Small" } }, { "id": "large", "label": { "en": "Large" } } ],
    "label": { "en": "Size" }, "visible_when": { "enabled": true } },
  { "id": "tint", "type": "color", "default": "#f0b429", "label": { "en": "Color" }, "scope": "client" }
]
```

### Mostrar opções só quando importam {#showing-options-only-when-they-matter}

`visible_when` esconde uma opção até outras opções terem os valores listados. Acima, **Size** só aparece enquanto **Show the compass** está ligado.

### Opções pessoais em mods de sessão {#personal-options-in-session-mods}

Os valores de opção de um mod de sessão ou de uma expansão contam quando o Obsidian compara os mods de dois jogadores, porque um controle de equilíbrio muda o jogo compartilhado. Marque uma opção com `"scope": "client"` quando o valor só muda o que este jogador vê, como uma cor ou a distância da câmera. Os jogadores podem então escolher valores diferentes e ainda bater ([Jogar online](wiki:multiplayer)).

### Opções num escopo secundário {#options-in-a-secondary-scope}

Num mod com [Escopos Híbridos](wiki:manifest#hybrid-scopes), `"secondary_scope": "<id>"` põe uma opção naquele escopo secundário. Uma opção de um escopo secundário de cliente conta como `"scope": "client"`, e uma opção de um escopo secundário desligado não conta. Num mod de sessão, uma opção sem marcação segue o escopo secundário mais restrito que está ligado. A opção liga/desliga que um escopo secundário cita em `option` o liga e desliga, então mudá-la exige uma aplicação.

## Grupos de opções {#option-groups}

`option_groups` organiza as opções sob títulos recolhíveis. Cada opção cita o título com `group`:

```json
"option_groups": [
  { "id": "idle", "label": { "en": "Idle crosshair" }, "description": { "en": "While not aiming." } },
  { "id": "aim", "label": { "en": "Aiming" }, "collapsed": true }
],
"options": [
  { "id": "idle_shape", "type": "choice", "default": "cross", "choices": ["cross", "dot"], "group": "idle",
    "label": { "en": "Shape" } }
]
```

Opções sem grupo vêm primeiro, depois cada grupo na ordem declarada. `collapsed` começa um grupo recolhido. O app lembra quais grupos o jogador abriu, para cada mod. `visible_when` funciona dentro dos grupos como em qualquer lugar. Um `group` que não cita nenhum grupo é recusado na importação. Um Obsidian mais antigo ignora os dois campos e lista as opções como antes.

## Prévias de opções {#option-previews}

`option_previews` mostra uma imagem dos valores, desenhada a partir de um arquivo SVG na pasta do mod e redesenhada conforme o jogador os muda:

```json
"option_previews": [
  { "id": "idle_preview", "group": "idle", "template": "preview/idle.svg", "width": 240, "height": 140,
    "label": { "en": "Preview" } }
]
```

A prévia aparece no topo do grupo que cita, ou no topo da aba Opções. O arquivo é SVG, de no máximo 64 KB, e é conferido na importação. Dentro dele:

- `{{expr}}` num valor de atributo ou num texto vira o valor da expressão.
- `data-if="expr"` em qualquer elemento remove o elemento enquanto a expressão é falsa.

Uma expressão usa números, `'texto'`, ids de opção (um número, `true` ou `false`, um id de escolha ou uma cor `#rrggbb`), `+ - * / %`, `== != < > <= >=`, `&& || !`, `a ? b : c`, parênteses e `min`, `max`, `abs`, `round`, `floor`, `ceil` e `clamp(x, lo, hi)`. `+` junta textos. Uma expressão com erro não dá nada, e um elemento cujo `data-if` tem erro é removido.

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="240" height="140">
  <rect width="240" height="140" fill="#1b1d1f"/>
  <g data-if="idle_shape == 'cross'" stroke="{{idle_color}}" stroke-width="{{idle_thickness}}">
    <line x1="{{120 - idle_size}}" y1="70" x2="{{120 + idle_size}}" y2="70"/>
    <line x1="120" y1="{{70 - idle_size}}" x2="120" y2="{{70 + idle_size}}"/>
  </g>
  <circle data-if="idle_shape == 'dot'" cx="120" cy="70" r="{{idle_size / 2}}" fill="{{idle_color}}"/>
</svg>
```

A imagem aparece como imagem: scripts, `foreignObject`, atributos `on...` e links para fora do arquivo são removidos.

## Ler opções no Lua {#reading-options-in-lua}

```lua
local get = obsidian.settings.get

if get("enabled") then
  local size = get("style")   -- "small" or "large"
end
```

`obsidian.settings.get` retorna o valor que o jogador escolheu, ou o padrão.

Uma opção ou tecla alterada chega ao jogo no próximo início do jogo: o Obsidian a escreve no arquivo de configurações que o carregador lê, sem aplicar. Estas opções exigem uma aplicação, e a dica de cada uma diz **Vale depois de Aplicar**:

- uma opção que escolhe arquivos com `when` (abaixo);
- a opção liga/desliga que controla um [escopo secundário](#options-in-a-secondary-scope);
- uma opção de um mod de sessão ou de uma expansão que conta quando os jogadores comparam conjuntos, ou seja, toda opção não marcada com `"scope": "client"` e fora de um escopo secundário de cliente;
- todas as opções de um mod sem script `entry`.

## Opções no menu de pausa {#options-in-the-pause-menu}

Um mod que precisa do Obsidian API também pode mostrar configurações no jogo, numa página em **Mods** no menu de pausa, com `api.mod_settings`. Uma linha criada com `option = true` é a opção de mesmo id no `obsidian.json`: ela começa do valor definido no app, e um valor mudado no jogo volta para o perfil ativo enquanto o jogo está fechado. A [referência do Obsidian API](https://github.com/Alomare/ObsidianAPI/blob/main/REFERENCE.md#your-settings-in-the-pause-menu-apimod_settings) lista as linhas.

## Opções que escolhem arquivos {#options-that-pick-files}

Um recurso pode depender de uma opção com `when`. O recurso só entra no jogo enquanto as opções têm esses valores:

```json
"resources": [
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_small.lua", "when": { "style": "small" } },
  { "type": "lua", "name": "lua/hud/compass", "source": "res/compass_large.lua", "when": { "style": "large" } }
]
```

Mudar uma opção assim exige uma nova aplicação.

## Teclas {#keys}

Declare cada tecla que o mod escuta, com um padrão:

```json
"keybinds": [ { "id": "toggle", "default": "f8", "label": { "en": "Turn the compass on or off" } } ]
```

Depois reaja a ela no Lua:

```lua
obsidian.keybind("toggle", function()
  obsidian.log("toggle pressed")
end)
```

A função roda uma vez a cada disparo do atalho. Os jogadores remapeiam teclas no app, onde os conflitos entre mods são resolvidos. Os nomes de tecla são os que o jogo usa, em minúsculas: `f8`, `k`, `insert`. Um jogador também pode atribuir um botão do mouse que não seja o esquerdo: o carregador lê `mouse_middle`, `mouse_right`, `mouse_extra_1` e `mouse_extra_2` do mouse.

### Modos de entrada {#input-modes}

Os jogadores também escolhem como cada atalho dispara, em **Mudar atalhos** na aba Teclas:

| Modo | Dispara |
|---|---|
| Pressionar (o padrão) | No quadro em que a tecla desce |
| Soltar | No quadro em que a tecla volta |
| Toque | Quando a tecla volta em menos de um quarto de segundo |
| Toque duplo | No segundo toque, até 0,3 segundo depois do primeiro |
| Segurar | Uma vez, depois de meio segundo segurando |

O carregador aplica o modo, então o mod não precisa de código para isso.

### Entrada de controle {#controller-input}

A aba Teclas tem uma coluna para mouse e teclado e uma para controle. Quando uma ação também tem uma entrada de controle, declare essa entrada como uma opção `choice` e cite a opção no `pad` do atalho:

```json
"options": [ { "id": "pad_toggle", "type": "choice", "default": "thumbs",
               "choices": [ { "id": "thumbs", "label": { "en": "Both sticks pressed in" } }, { "id": "off", "label": { "en": "Off" } } ] } ],
"keybinds": [ { "id": "toggle", "default": "f8", "pad": "pad_toggle", "label": { "en": "Turn the compass on or off" } } ]
```

O app então mostra a opção na linha do atalho, na coluna do controle, e a deixa fora da aba Opções. O mod lê a opção como qualquer outra. Para o modo de entrada valer também para o controle, passe a `obsidian.keybind` uma função que diz se a entrada de controle está pressionada, como `opts.pad`; o carregador então chama `fn` com `"pad"` quando ela dispara (`obsidian.capabilities.input_modes` é `true` num carregador que faz isso). Senão, o mod vigia o controle sozinho.

Os jogadores podem dar a qualquer atalho uma entrada de controle em **Mudar atalhos**, com ou sem opção `pad`: de um a quatro botões segurados juntos. O carregador então lê o controle para aquele atalho e chama `fn` com `"pad"`, no lugar de `opts.pad` (`obsidian.capabilities.pad_binds` é `true` num carregador que faz isso). O mod não precisa de código para isso. Um atalho sem `pad` mostra um traço nessa coluna. Versões do Obsidian sem a coluna do controle ignoram `pad` e listam a opção em Opções.
