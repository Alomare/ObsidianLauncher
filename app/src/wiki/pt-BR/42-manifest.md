---
title: O arquivo obsidian.json
group: Criar mods
summary: Cada campo do arquivo que descreve um mod.
---

`obsidian.json` fica no topo da pasta do mod, ou no topo do `.zip`. O Obsidian o confere na importação e diz qual campo está errado quando algo não se encaixa.

## Um exemplo completo {#a-full-example}

```json
{
  "obsidian": 1,
  "id": "yourname.better-camera",
  "version": "1.2.0",
  "name": { "en": "Better Camera", "pt-BR": "Câmera Melhor" },
  "description": { "en": "A closer camera with a toggle key." },
  "authors": ["Author Name"],
  "image": "image.png",
  "scope": "client",
  "entry": "main.lua",
  "loader_api": 1,
  "options": [
    { "id": "distance", "type": "number", "default": 5, "min": 1, "max": 12, "step": 0.5,
      "label": { "en": "Camera distance" } }
  ],
  "keybinds": [
    { "id": "toggle", "default": "f8", "label": { "en": "Toggle the camera" } }
  ],
  "resources": []
}
```

## Campos {#fields}

| Campo | Obrigatório | O que é |
|---|---|---|
| `obsidian` | sim | A versão do formato. Sempre `1` por enquanto. |
| `id` | sim | Id permanente: de 1 a 64 caracteres entre `a-z`, `0-9`, `.`, `_` e `-`, começando com letra ou dígito. Use `seunome.nome-do-mod`. |
| `version` | sim | Um número de versão como `1.2.0` (versionamento semântico). Uma versão maior substitui uma menor na importação. |
| `name` | sim | O nome do mod por idioma. Inglês (`en`) é obrigatório. |
| `description` | não | Uma ou duas frases por idioma, mostradas abaixo do nome. |
| `authors` | não | Uma lista de nomes. |
| `image` | não | Um PNG, JPG ou WebP quadrado dentro da pasta do mod, de até 4 MB. |
| `source` | não | O endereço web do mod, mostrado nos detalhes. Um repositório do GitHub como `https://github.com/author/mod` também deixa os jogadores atualizarem o mod pelo app ([Compartilhar um mod](wiki:sharing#updates)). |
| `scope` | não | `client`, `session`, `expansion` ou `library` ([Escopos](wiki:scopes)). Ausente ou desconhecido torna o mod não especificado, que roda como mod de cliente, ou como mod de sessão com `every_player`. |
| `every_player` | não | `true` para um mod que só funciona quando todos os jogadores do lobby o rodam ([Mods de que todos precisam](wiki:session-mods#every-player)). Não serve para mods de cliente nem expansões. |
| `secondary_scopes` | não | Código, opções e arquivos que rodam com um escopo diferente de `scope` ([Escopos Híbridos](#hybrid-scopes)). Não serve para bibliotecas. |
| `entry` | não | O script Lua que o carregador inicia. Mods que só substituem arquivos do jogo o deixam de fora. |
| `modules` | não | Mais arquivos `.lua` além de `entry`, que o mod carrega com `obsidian.module`. Caminhos dentro da pasta do mod. |
| `requires` | não | Ids de mods de que este precisa. Ligar o mod os liga, e eles começam antes. Os jogadores os veem na aba Dependências do mod. Uma aplicação para com uma mensagem se faltar algum. |
| `load` | não | Onde o mod pede para ficar na ordem de carregamento: `"first"`, `"last"`, ou um objeto com mais pedidos ([Pedidos de ordem de carregamento](#load-order-requests)). |
| `load_after` | não | Ids de mods depois dos quais este pede para carregar. O mesmo que `after` dentro de `load`, mantido para mods escritos antes dele. |
| `loader_api` | não | A versão da [Loader API](wiki:loader-api) de que o mod precisa. `1` quando omitido. |
| `min_obsidian` | não | O Obsidian mais antigo com que o mod funciona, como `"1.1.0"`. Um mais antigo recusa o mod e pede ao jogador para atualizar. O Obsidian antes da 1.1 não o lê. |
| `options` | não | Configurações que os jogadores mudam no app ([Opções e teclas](wiki:options-and-keys)). |
| `option_groups` | não | Títulos recolhíveis para agrupar as opções ([Grupos de opções](wiki:options-and-keys#option-groups)). |
| `option_previews` | não | Imagens dos valores das opções, a partir de modelos SVG ([Prévias de opções](wiki:options-and-keys#option-previews)). |
| `keybinds` | não | Teclas que os jogadores podem remapear no app. O `pad` de um atalho cita a opção de escolha com a entrada de controle para a mesma ação ([Entrada de controle](wiki:options-and-keys#controller-input)). |
| `resources` | não | Arquivos do jogo que o mod substitui ou adiciona ([Substituir arquivos do jogo](wiki:resources)). |
| `save_editor` | não | O que o mod mostra na aba Mods do [editor de saves](wiki:saves) ([O editor de saves](#save-editor)). |

## Pedidos de ordem de carregamento {#load-order-requests}

`load` aceita `"first"` ou `"last"` sozinhos, ou um objeto com qualquer um destes campos:

```json
"load": {
  "position": "last",
  "before": ["author.other-mod"],
  "after": ["author.base-mod"],
  "after_patches": true
}
```

| Campo | O que pede |
|---|---|
| `position` | `"first"` para o topo da ordem de carregamento, `"last"` para o fim. |
| `before` | Ids de mods antes dos quais este carrega. |
| `after` | Ids de mods depois dos quais este carrega, para que este mod vença nos arquivos que ambos substituem. |
| `before_patches` | `true` para carregar antes de todos os mods que substituem arquivos do jogo (têm `resources`). |
| `after_patches` | `true` para carregar depois de todos os mods que substituem arquivos do jogo. |

Um mod novo ocupa o lugar que pede quando entra num perfil. Depois disso, a ordem que o jogador definiu vale até ele clicar em **Ordenar mods** na aba Mods ([Ordenar mods](wiki:load-order#sort-mods)), que move só os mods que precisam. Ids de mods que o jogador não tem são ignorados, e Ordenar mods avisa.

Peça só o que o mod precisa. Dois mods que pedem para carregar por último, ou que pedem para carregar um depois do outro, não podem ser atendidos: Ordenar mods os deixa onde estão e diz ao jogador para ordená-los à mão.

O Obsidian confere `load` na importação: uma posição diferente de `first` ou `last`, um campo desconhecido, o id do próprio mod, ou o mesmo id em `before` e `after` são recusados. O Obsidian 1.3.1 e anteriores só leem `"last"`, e listam um mod com qualquer outro `load` como um que não pode ser lido.

## Escopos Híbridos {#hybrid-scopes}

`scope` é o escopo principal do mod. `secondary_scopes` lista os escopos com que parte do código, das opções e dos arquivos roda no lugar dele, cada um `client` ou `session` ([Escopos Híbridos](wiki:scopes#hybrid-scopes)). Um mod de sessão os usa para manter código pessoal rodando num lobby não liberado; uma expansão usa um escopo secundário de sessão para código que também roda fora do conteúdo, num lobby liberado.

```json
"scope": "session",
"secondary_scopes": [
  { "id": "view", "scope": "client",
    "name": { "en": "Camera and view" }, "description": { "en": "Keeps working in any lobby." } },
  { "id": "weapon_range", "scope": "session", "option": "max_weapon_range",
    "name": { "en": "Max weapon range" } }
]
```

| Campo | O que é |
|---|---|
| `id` | Letras minúsculas, dígitos e `_`, como um id de opção. |
| `scope` | `client` ou `session`. |
| `name`, `description` | Texto por idioma, mostrado nos detalhes do mod e no aviso dentro do jogo. |
| `option` | Uma opção liga/desliga do mod. O escopo secundário só fica ligado enquanto essa opção está ligada, e mudá-la exige uma aplicação. |

Código, opções e arquivos citam o escopo secundário deles: `opts.secondary_scope` na [Loader API](wiki:loader-api#hybrid-scopes), `secondary_scope` numa [opção](wiki:options-and-keys#options-in-a-secondary-scope) e num [recurso](wiki:resources#hybrid-scopes). Num mod de sessão, o que não tem marcação segue o escopo secundário mais restrito que está ligado; numa expansão, fica com a expansão. Um escopo secundário desligado não roda código, não aplica arquivos e não conta no que os jogadores comparam.

O escopo principal é o que um Obsidian mais antigo lê, e ele roda o mod inteiro com esse escopo, então declare ali o mais restrito: `session` para um mod de sessão com escopo secundário de sessão, `expansion` para uma expansão. O Obsidian recusa um id repetido, um escopo diferente de `client` ou `session`, um escopo secundário de sessão sob um escopo principal diferente de `session` ou `expansion`, uma `option` que não é liga/desliga, e uma marcação `secondary_scope` que não cita nenhum deles. O Obsidian 2.0.0-beta.1 e anteriores leem a mesma lista com o nome `parts`, com `part` como marcação; o Obsidian ainda lê esses nomes.

## Texto em vários idiomas {#text-in-several-languages}

Todo campo que os jogadores leem é um objeto com uma entrada por idioma, como `en`, `pt-BR`, `ru` e `zh-CN`, os idiomas do app. Inglês é sempre obrigatório e preenche qualquer idioma que falte.

O texto que o mod mostra no jogo fica numa pasta `locales` ao lado de `obsidian.json`, um arquivo por idioma do jogo (`locales/en.json`, `locales/fr.json` e assim por diante), cada um um objeto simples de chaves e textos. O mod os lê com `obsidian.text`, que lista os nomes dos arquivos ([Loader API](wiki:loader-api#obsidian-text-key)).

## O editor de saves {#save-editor}

`save_editor` adiciona uma seção do mod à aba Mods do [editor de saves](wiki:saves), para saves Sandbox. Os valores ficam nos dados que o mod lê com [`obsidian.save_data`](wiki:loader-api#save-data), sob o id do mod.

```json
"save_editor": {
  "items": [
    { "type": "stratagem", "name": "my_strike", "label": { "en": "Orbital Strike" }, "unlock": "manual" },
    { "type": "weapon", "name": "my_rifle", "label": { "en": "Long Rifle" }, "unlock": { "rank": 10 } }
  ],
  "fields": [
    { "id": "difficulty", "type": "choice", "label": { "en": "Difficulty" }, "choices": ["easy", "hard"] },
    { "id": "boost", "type": "number", "label": { "en": "Boost" }, "min": 0, "max": 5, "when": { "difficulty": "hard" } },
    { "id": "enabled", "type": "toggle", "label": { "en": "Enabled" } }
  ]
}
```

`items` lista o que o mod adiciona ao jogo. `type` é `stratagem`, `weapon` ou `perk`, e `unlock` diz como um save recebe o item: `"manual"` mostra um interruptor, `"free"` e `"base"` querem dizer que todo save o tem, e `{ "rank": n }` quer dizer que ele vem com aquela patente.

`fields` são valores que o jogador pode mudar. `type` é `number` (com `min` e `max`), `toggle` ou `choice` (com `choices`, cada um um id ou um objeto com `id` e `label`). `when` deixa o campo acinzentado a menos que os campos citados tenham os valores citados.

Enquanto o mod está desligado, a seção fica acinzentada e os dados ficam no save. Se o mod sai da biblioteca, os dados são listados sob o id do mod para o jogador apagar. `"type": "tree"` está reservado para o futuro.

## Campos que o Obsidian não conhece {#fields-obsidian-doesn-t-know}

Campos desconhecidos são mantidos e ignorados, então um mod escrito para uma versão futura do Obsidian ainda é importado.
