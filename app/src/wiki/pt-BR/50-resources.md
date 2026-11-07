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

## Clipes de um banco de som {#sound-bank-clips}

Um banco inteiro pode passar de um gigabyte somando os idiomas. Um mod que muda poucos sons pode trazer só esses clipes em `clips`. Ao aplicar, o Obsidian aponta a lista de sons do jogo (`timpani_master`, no bundle de boot) para ids novos desses clipes, e coloca os clipes, com os ids novos, num pequeno banco do Obsidian ao lado do banco que eles substituem. Os bancos do jogo ficam como estão:

```json
{ "type": "timpani_bank", "name": "#e0f28740d6c19769",
  "clips": [
    { "clip": "#1111111111111111", "data": "res/rock.clip", "stream": "res/rock.stream" },
    { "clip": "#1111111111111111", "lang": 256, "data": "res/rock.fr.clip", "stream": "res/rock.fr.stream" },
    { "clip": "#2222222222222222", "data": "res/short.clip" }
  ] }
```

- `clip` é o id do clipe, `#` mais 16 dígitos hexadecimais. `lang` é o valor de idioma da tabela acima, 0 se você não informar.
- `data` é o que vai no banco: o cabeçalho de 68 bytes do clipe e a primeira parte do áudio. `stream` é o resto, se houver.
- Um recurso com `clips` não tem `bundle`: esse pequeno banco vai para todo bundle que contém o banco do jogo e carrega junto com ele, no idioma em que o jogo está configurado. Clipes e idiomas que você não listar tocam como no jogo.
- Duas linhas podem usar o mesmo arquivo. Mods podem mudar clipes diferentes do mesmo banco, e quando dois mudam o mesmo clipe, vale o último na ordem de carregamento. Sobre um mod que substitui o banco inteiro, esse banco continua sendo aplicado, e os idiomas que você não listar tocam o clipe como esse mod o tem.
- Aplicar recusa um clipe ou idioma que o banco não tem, e um arquivo cujo cabeçalho não bate com o tamanho.

Uma atualização do jogo conta como mudanças esperando, e aplicar de novo lê o jogo outra vez. O patch aplicado guarda a lista de sons (cerca de 0,2 MB) e os clipes novos, com o clipe original do jogo para cada idioma que você deixar de fora onde esse idioma toca outro áudio: um passo em inglês americano, que fica em dois bancos, ocupa cerca de 4 MB de disco.

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
| `clips` | No lugar de `data`: os clipes de um banco de som a mudar, cada um com `clip`, `lang`, `data` e um `stream` opcional |
| `bundle` | O bundle onde o recurso entra, 16 dígitos hexadecimais. Necessário em todo recurso, menos os que têm `clips` |
| `flags` | Um valor que os bundles guardam ao lado de cada recurso; deixe de fora, a não ser que um conversor tenha definido |
| `when` | Opções que precisam ter esses valores para o recurso ser usado ([Opções e teclas](wiki:options-and-keys#options-that-pick-files)) |

## Limites {#limits}

Um recurso substituído é estático: não dá para desligar no meio de uma partida. Num mod de sessão, use recursos para dados que o jogo lê ao abrir, e hooks para comportamento.
