---
name: flatmates
description: Gera ilustrações Corporate Memphis (pessoas, escritório, pets, objetos) em SVG e PNG a partir de um JSON. Use quando pedirem ilustração, hero image, imagem de onboarding, avatar de equipe ou cena flat com personagens. Generates Corporate Memphis illustrations from JSON.
---

# Flatmates

Você descreve **o que** aparece e **como as coisas se relacionam**. O motor decide proporções, perspectiva, oclusão, cores e enquadramento. Não calcule pixels.

## Fluxo

1. Escreva a cena num arquivo JSON.
2. Rode `scripts/flatmates explain cena.json` e leia `warnings` e as caixas (`box`) de cada item.
3. Corrija o que os avisos apontarem e repita.
4. Rode `scripts/flatmates render cena.json -o cena.svg --png cena.png --scale 2`.
5. Abra o PNG e confira antes de entregar.

`scripts/flatmates catalog` lista todos os valores aceitos. `scripts/flatmates example office` imprime um exemplo completo.

## Como pensar a cena

- O mundo é em **metros**. O chão é o plano x (direita) e z (em direção à câmera). Uma pessoa tem cerca de 1,7 m; uma mesa, 1,5 m de largura.
- Prefira **relações** a coordenadas. Elas nunca erram o encaixe:
  - `"sits_on": "cadeira"` senta no assento.
  - `"on": "mesa"` apoia em cima (vale em vários níveis).
  - `"near": {"id": "mesa", "side": "left", "gap": 0.3}` põe ao lado.
  - `"face_to": "quadro"` vira o corpo; `"look_at"` vira a cabeça; `"points_at"` estica o braço.
  - `"uses": "laptop"` leva as mãos ao objeto.
- Use `at: [x, z]` só para ancorar um ou dois itens; o resto vem por relação.
- `facing` em graus: 0 olha para a câmera frontal, 90 para a direita, 180 de costas.
- Mantenha a cena compacta (até uns 5 m de largura). Cena espalhada deixa todo mundo pequeno, e o `explain` avisa.

## Exemplo

```json
{
  "theme": "mint",
  "camera": "three-quarter",
  "title": "Planejamento da semana",
  "items": [
    { "id": "mesa", "kind": "desk", "at": [0, 0] },
    { "id": "note", "kind": "laptop", "on": "mesa", "at": [0, -0.16], "facing": 180 },
    { "id": "cadeira", "kind": "chair", "at": [0, -0.6] },
    { "id": "ana", "kind": "person", "sits_on": "cadeira", "uses": "note", "hair": "bun", "outfit": "sweater" },
    { "kind": "person", "near": { "id": "mesa", "side": "right", "gap": 0.3 }, "face_to": "ana", "look_at": "ana", "pose": "pointing", "points_at": "note" },
    { "kind": "plant", "near": { "id": "mesa", "side": "left", "gap": 0.4 } }
  ]
}
```

## Receitas

- **Trabalho remoto:** sofá, pessoa sentada com `holds: "tablet"`, gato perto, `card` flutuando, luminária e planta nas pontas.
- **Colaboração:** `board` no centro, uma pessoa com `pose: "pointing"` e `points_at`, outra `thinking` com `look_at` na primeira, uma terceira chegando com `walking`.
- **Escritório:** câmera `three-quarter` ou `isometric`, mesa, laptop `on` a mesa, cadeira atrás, pessoa com `sits_on` + `uses`.
- **Mesa de pé:** `desk` com `scale: 1.35` e pessoa com `pose: "desk-typing"` + `uses`.
- **Conversa:** `says` em quem fala, `look_at` em quem ouve. Um balão por cena lê melhor que vários.
- **Sala:** `rug` no chão sob o sofá dá um plano de base à cena.
- **Marca do cliente:** `palette` com 3 a 5 hex da marca e `background` com a cor de fundo. Tudo se recolore.
- **Série com o mesmo personagem:** defina em `cast` e use `who` em cada cena.

## Quando o catálogo não tem o objeto

Descreva com formas simples em metros (veja [references/objetos.md](references/objetos.md)). Ele ganha oclusão, três tons e sombra como os nativos.

## Animação

Dê `motion` (`walk`, `run`, `wave`, `jump`, `type`) a quem se move e rode:

```
scripts/flatmates animate cena.json -o cena-animada.svg --frames 12 --fps 12
```

`--png-dir pasta` grava os quadros em PNG.

## Erros comuns

- Pessoa de costas para o que usa: troque `facing` ou use `face_to`.
- Mão que não alcança: aproxime com `near` e `gap` menor.
- Dois itens no mesmo lugar: dê `at` diferente ou use `near`.
- Laptop: a tela olha para o lado de `facing`. Quem usa fica desse lado.

Mais opções em [references/contrato.md](references/contrato.md).
