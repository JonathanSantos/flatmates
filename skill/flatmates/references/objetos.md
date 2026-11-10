# Objetos próprios

Defina em `defs` (o nome vira um `kind`) ou direto no item com `kind: "custom"` e `parts`.

Coordenadas locais em metros: x direita, y cima, z frente. O chão é y = 0.

| `shape` | Campos | Uso |
|---|---|---|
| `box` | `at` (centro), `size` [l, a, p], `yaw`, `round` | móveis, caixas, telas grossas |
| `cylinder` | `at` (centro da base), `r`, `r_top`, `h` | postes, vasos, copos, cones |
| `sphere` | `at`, `r` | bolas, lâmpadas, botões |
| `hull` | `points` [[x, y, z, raio], ...] | formas orgânicas: asa, ponta, almofada |
| `chain` | `points` [[x, y, z, raio], ...] | tubos curvos: cabo, alça, chama, rabo |
| `panel` | `at` (centro), `size` [l, a], `content`, `yaw` | placa, tela, quadro |

`color` em cada forma: `accent` (cor do item), `accent2`, `steel`, um nome (`coral`, `blue`, `wood`, `white`...) ou hex.

Pontos de contato, todos opcionais:

- `top`: altura onde outros objetos se apoiam com `on`.
- `seat`: [x, y, z] do assento, para `sits_on`.
- `use`: [x, y, z] onde as duas mãos trabalham, para `uses`.
- `touch`: [x, y, z] que uma mão toca, para `uses`.
- `shadow`: [meia-largura, meia-profundidade] da sombra no chão.

Use de 3 a 12 formas. Poucas formas grandes leem melhor que muitas pequenas.

```json
"defs": {
  "banco": {
    "parts": [
      { "shape": "box", "at": [0, 0.42, 0], "size": [1.3, 0.07, 0.42], "color": "wood" },
      { "shape": "box", "at": [-0.52, 0.2, 0], "size": [0.07, 0.4, 0.36], "color": "steel" },
      { "shape": "box", "at": [0.52, 0.2, 0], "size": [0.07, 0.4, 0.36], "color": "steel" }
    ],
    "top": 0.455,
    "seat": [0.25, 0.455, 0.02],
    "shadow": [0.7, 0.28]
  }
}
```
