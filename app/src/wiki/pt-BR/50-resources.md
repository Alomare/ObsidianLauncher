---
title: Substituir arquivos do jogo
group: Criar mods
summary: Mudar scripts e outros recursos do jogo pelo nome, sem distribuir bundles inteiros.
---

Além de rodar um script, um mod pode substituir recursos do jogo, ou adicionar novos, pela lista `resources`. O Obsidian coloca esses recursos nos patches ao aplicar e resolve os conflitos com outros mods recurso por recurso.

## Um script do jogo alterado {#a-changed-game-script}

Copie o script do jogo que você quer mudar para a pasta do mod, edite e liste:

```json
"resources": [
  { "type": "lua", "name": "lua/settings/perk_settings", "source": "res/perk_settings.lua" }
]
```

- `type` e `name` são o caminho do recurso dentro do jogo.
- `source` é Lua puro na pasta do mod. O Obsidian compila ao aplicar.
- `bytecode` pode substituir `source` para um arquivo que já é bytecode LuaJIT 2.0 compilado. O Obsidian confere o cabeçalho e recusa qualquer outra coisa.

> [!TIP]
> Um hook muda uma função e deixa o resto do script com o jogo, então dois mods podem mudar o mesmo script sem conflito. Prefira [hooks](wiki:loader-api#hooks) para código, e substitua um script inteiro só quando não houver outro jeito.

## Outros recursos {#other-resources}

Texturas, unidades, materiais e outros tipos usam `data`, o recurso bruto como o jogo guarda:

```json
{ "type": "texture", "name": "units/foo/bar", "data": "res/bar.texture" }
```

Mods antigos convertidos na importação usam essa forma. Criar esses arquivos do zero exige ferramentas que ainda não existem para o HD1.

## Bancos de som e vídeos {#streamed-resources}

Bancos de som (`timpani_bank`) e vídeos (`bik`) guardam a maior parte dos bytes no arquivo `.stream` de um bundle. Informe essa parte em `stream`, ao lado de `data`:

```json
{ "type": "timpani_bank", "name": "#e709d0c5697fb286", "bundle": "0e2ec9da60a0f53a",
  "data": "res/menu.timpani_bank", "stream": "res/menu.stream" }
```

Um recurso que o jogo tem em vários idiomas lista cada um em `variants`, com a flag de idioma do jogo em `lang`. Uma variante sem `lang` vale 0, a que todo idioma sem variante própria usa:

```json
{ "type": "timpani_bank", "name": "#f0b3f997d587fb77", "bundle": "f0b3f997d587fb77",
  "variants": [
    { "data": "res/voices.timpani_bank", "stream": "res/voices.stream" },
    { "lang": 256, "data": "res/voices.fr.timpani_bank", "stream": "res/voices.fr.stream" }
  ] }
```

| Idioma | `lang` |
|---|---|
| Todo idioma sem variante própria | 0 |
| Japonês | 1 |
| Francês | 256 |
| Espanhol (América Latina) | 512 |
| Espanhol (Espanha) | 1024 |
| Português (Brasil) | 2048 |
| Russo | 4096 |
| Italiano | 8192 |
| Alemão | 16384 |

No deploy, o Obsidian grava as partes em stream num arquivo `.stream` ao lado do patch (`<bundle>.patch_0.stream`), na ordem em que o patch as lista, como o jogo organiza as dele. Som muda só o que você ouve, então um mod de bancos de som pode ser um [mod de cliente](wiki:scopes).

## Campos {#fields}

| Campo | O que é |
|---|---|
| `type` | O tipo do recurso, como `lua` ou `texture`, ou `#` mais 16 dígitos hexadecimais para um tipo sem nome conhecido |
| `name` | O nome do recurso, como `lua/settings/perk_settings`, ou `#` mais 16 dígitos hexadecimais |
| `source` | Código Lua para compilar |
| `bytecode` | Bytecode LuaJIT 2.0 já compilado |
| `data` | Um recurso bruto de qualquer outro tipo |
| `stream` | Com `data`: a parte do recurso que o jogo lê do arquivo `.stream` |
| `variants` | No lugar de `data`: uma entrada por idioma, cada uma com `lang`, `data` e um `stream` opcional |
| `bundle` | O bundle em que o recurso entra, 16 dígitos hexadecimais. Só é preciso para um recurso que o jogo ainda não tem |
| `flags` | Um valor que os bundles guardam ao lado de cada recurso; deixe de fora, a não ser que um conversor tenha definido |
| `when` | Opções que precisam ter esses valores para o recurso ser usado ([Opções e teclas](wiki:options-and-keys#options-that-pick-files)) |

## Limites {#limits}

Um recurso substituído é estático: não dá para desligar no meio de uma partida. Num mod de sessão, use recursos para dados que o jogo lê ao abrir, e hooks para comportamento.
