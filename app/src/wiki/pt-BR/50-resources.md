---
title: Substituir arquivos do jogo
group: Criar mods
summary: Mudar scripts e outros recursos do jogo pelo nome, sem distribuir bundles inteiros.
---

Além de rodar um script, um mod pode substituir recursos do jogo, ou adicionar novos, pela lista `resources`. O Obsidian os coloca nos patches ao aplicar e resolve conflitos com outros mods recurso por recurso.

## Um script do jogo alterado {#a-changed-game-script}

Copie para a pasta do mod o script do jogo que vai mudar, edite-o e liste-o:

```json
"resources": [
  { "type": "lua", "name": "lua/settings/perk_settings", "source": "res/perk_settings.lua" }
]
```

- `type` e `name` são o caminho do recurso no jogo.
- `source` é Lua comum na pasta do mod. O Obsidian o compila ao aplicar.
- `bytecode` pode substituir `source` para um arquivo que já é bytecode LuaJIT 2.0 compilado. O Obsidian confere o cabeçalho e recusa qualquer outra coisa.

> [!TIP]
> Um gancho muda uma função e deixa o resto do script para o jogo, então dois mods podem mudar o mesmo script sem conflito. Prefira [ganchos](wiki:loader-api#hooks) para código, e substitua um script inteiro só quando for preciso.

## Outros recursos {#other-resources}

Texturas, unidades, materiais e outros tipos usam `data`, o recurso bruto como o jogo o guarda:

```json
{ "type": "texture", "name": "units/foo/bar", "data": "res/bar.texture" }
```

Mods antigos convertidos na importação usam esta forma. Criar esses arquivos do zero exige ferramentas que ainda não existem para HD1.

## Bancos de som e vídeos {#streamed-resources}

Bancos de som (`timpani_bank`) e vídeos (`bik`) guardam a maior parte dos bytes no arquivo `.stream` de um bundle. Passe essa parte como `stream`, ao lado de `data`:

```json
{ "type": "timpani_bank", "name": "#e709d0c5697fb286", "bundle": "0e2ec9da60a0f53a",
  "data": "res/menu.timpani_bank", "stream": "res/menu.stream" }
```

Um recurso que o jogo tem em vários idiomas lista cada um em `variants`, com a marca de idioma do jogo em `lang`. Uma variante sem `lang` é 0, a usada por todo idioma que não tem variante própria:

```json
{ "type": "timpani_bank", "name": "#f0b3f997d587fb77", "bundle": "f0b3f997d587fb77",
  "variants": [
    { "data": "res/voices.timpani_bank", "stream": "res/voices.stream" },
    { "lang": 256, "data": "res/voices.fr.timpani_bank", "stream": "res/voices.fr.stream" }
  ] }
```

| Idioma | `lang` |
|---|---|
| Todo idioma sem variante | 0 |
| Japonês | 1 |
| Francês | 256 |
| Espanhol (América Latina) | 512 |
| Espanhol (Espanha) | 1024 |
| Português (Brasil) | 2048 |
| Russo | 4096 |
| Italiano | 8192 |
| Alemão | 16384 |

Ao aplicar, o Obsidian escreve as partes em stream num arquivo `.stream` ao lado do patch (`<bundle>.patch_0.stream`), na ordem em que o patch as lista, como o jogo organiza as originais. Uma mudança de som fica num PC só, então um mod de bancos de som pode ser um [mod de cliente](wiki:scopes).

## Clipes isolados de um banco de som {#sound-bank-clips}

Um banco inteiro pode passar de um gigabyte somando todos os idiomas. Um mod que muda poucos sons pode trazer só esses clipes em `clips`. Ao aplicar, o Obsidian aponta a lista de sons do jogo (`timpani_master`, no bundle de boot) para novos ids desses clipes, e põe os clipes com os novos ids num pequeno banco do Obsidian ao lado do banco que eles substituem. Os bancos do jogo ficam como estão:

```json
{ "type": "timpani_bank", "name": "#e0f28740d6c19769",
  "clips": [
    { "clip": "#1111111111111111", "data": "res/rock.clip", "stream": "res/rock.stream" },
    { "clip": "#1111111111111111", "lang": 256, "data": "res/rock.fr.clip", "stream": "res/rock.fr.stream" },
    { "clip": "#2222222222222222", "data": "res/short.clip" }
  ] }
```

- `clip` é o id do clipe, `#` mais 16 dígitos hexadecimais. `lang` é a marca de idioma da tabela acima, 0 quando omitida.
- `data` é o que vai no banco: o cabeçalho de 68 bytes do clipe e a primeira parte do áudio. `stream` é o resto, se houver.
- Um recurso com `clips` não tem `bundle`: aquele pequeno banco vai para todo bundle que contém o banco do jogo e carrega junto com ele, no idioma em que o jogo está. Clipes e idiomas omitidos tocam como o jogo os tem.
- Duas linhas podem usar o mesmo arquivo. Mods podem mudar clipes diferentes de um banco, e quando dois mudam o mesmo clipe, vence o que está mais abaixo na ordem de carregamento. Por cima de um mod que substitui o banco inteiro, esse banco continua sendo aplicado, e os idiomas que não forem listados tocam o clipe como aquele mod o tem.
- A aplicação recusa um clipe ou idioma que o banco não tem, e um arquivo cujo cabeçalho não bate com o tamanho do arquivo.

Uma atualização do jogo conta como mudança pendente, e uma nova aplicação lê o jogo de novo. O patch aplicado guarda a lista de sons (cerca de 0,2 MB) e os novos clipes, com o clipe original de cada idioma deixado de fora onde aquele idioma toca outro áudio: um passo em inglês americano, que fica em dois bancos, ocupa cerca de 4 MB de disco.

## Campos {#fields}

| Campo | O que é |
|---|---|
| `type` | O tipo do recurso, como `lua` ou `texture`, ou `#` mais 16 dígitos hexadecimais para um tipo sem nome conhecido |
| `name` | O nome do recurso, como `lua/settings/perk_settings`, ou `#` mais 16 dígitos hexadecimais |
| `source` | Código-fonte Lua para compilar |
| `bytecode` | Bytecode LuaJIT 2.0 compilado |
| `data` | Um recurso bruto de qualquer outro tipo |
| `stream` | Com `data`: a parte do recurso que o jogo lê do arquivo `.stream` |
| `variants` | No lugar de `data`: uma entrada por idioma, cada uma com `lang`, `data` e um `stream` opcional |
| `clips` | No lugar de `data`: os clipes de um banco de som a mudar, cada um com `clip`, `lang`, `data` e um `stream` opcional |
| `bundle` | O bundle em que o recurso entra, 16 dígitos hexadecimais. Necessário para todo recurso, exceto um com `clips` |
| `flags` | Um valor que os bundles guardam ao lado de cada recurso; deixe de fora a menos que um conversor o tenha definido |
| `when` | Opções que precisam ter estes valores para o recurso ser usado ([Opções e teclas](wiki:options-and-keys#options-that-pick-files)) |
| `secondary_scope` | O [escopo secundário](wiki:manifest#hybrid-scopes) a que o recurso pertence (`part` antes de os Escopos Híbridos terem esse nome) |

## Arquivos num escopo secundário {#hybrid-scopes}

Num mod com [Escopos Híbridos](wiki:manifest#hybrid-scopes), `"secondary_scope": "<id>"` põe um recurso naquele escopo secundário. Um recurso de um escopo secundário desligado não é aplicado, como com `when`. Um recurso de um escopo secundário de cliente fica com o jogador que o roda: ele não mantém jogadores vanilla fora de um lobby público, como fazem os arquivos de um mod de sessão. Num mod de sessão, um recurso sem escopo secundário segue o mais restrito que está ligado, então num mod de sessão ou híbrido ele conta como conteúdo de sessão.

## Limites {#limits}

Um recurso substituído é estático: não pode ser desligado no meio de uma partida. Num mod de sessão, use recursos para dados que o jogo lê na inicialização e ganchos para comportamento. Os Escopos Híbridos não mudam isso: os arquivos de um escopo secundário de sessão ficam no jogo em todo lobby, e só a aplicação decide quais entram.
