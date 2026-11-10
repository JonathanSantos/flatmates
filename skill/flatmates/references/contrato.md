# Contrato do JSON

`flatmates catalog` é a fonte da verdade para os valores. Este arquivo explica o que cada campo faz.

## Cena

| Campo | O que faz |
|---|---|
| `size` | `[largura, altura]` em pixels. Padrão `[1200, 630]`. |
| `theme` | `day`, `pastel`, `sunset`, `mint`, `night`. |
| `background` | Hex de fundo próprio; blobs e chão derivam dele. Fundo escuro clareia os neutros. |
| `palette` | 2 a 8 cores que substituem os acentos (roupas, móveis, confete). |
| `shading` | `false` desliga a sombra tonal das formas grandes. |
| `haze` | `false` desliga a névoa de profundidade (o fundo puxa para a cor do tema). |
| `camera` | `front`, `three-quarter`, `isometric`, `high` ou `{"yaw": 30, "pitch": 20}`. |
| `seed` | Muda o sorteio de cores, cabelos e confete. |
| `title` | Título no topo; quebra em até 3 linhas sozinho. |
| `backdrop` | Liga ou desliga `blob`, `shapes`, `confetti`, `ground`, `grain`. |
| `cast` | Personagens nomeados: `{"ana": {"hair": "bun", ...}}`. |
| `defs` | Objetos próprios (veja objetos.md). |

## Item

| Campo | O que faz |
|---|---|
| `id` | Nome para outros itens apontarem. |
| `kind` | `person`, um objeto do catálogo, um nome de `defs`, ou `custom`. |
| `at` | `[x, z]` em metros. |
| `near` | `{"id", "side": left/right/front/behind, "gap"}`. |
| `on` | Id do apoio; `at` passa a ser relativo a ele. |
| `facing` / `face_to` | Graus, ou id para o qual virar. |
| `scale`, `color`, `content`, `lift` | Tamanho, cor, conteúdo de tela e altura. |

## Pessoa

| Campo | O que faz |
|---|---|
| `who` | Nome em `cast`; o que o item disser vale por cima. |
| `pose` | Pose parada. `sitting` e `typing` pedem `sits_on`; `floor-sitting` e `lying` não. |
| `motion` | Movimento cíclico para `flatmates animate`. |
| `sits_on`, `seat` | Assento (e qual lugar do sofá: 0 ou 1). |
| `uses` | Mãos no objeto: as duas no laptop, uma em caneca, quadro, caixa, livros, mesa. |
| `holds` | `phone`, `mug`, `book`, `tablet`, `box`. |
| `look_at`, `points_at` | Cabeça e braço miram o item. |
| `outfit`, `bottom`, `hair`, `skin`, `face`, `beard`, `accessories`, `build`, `height` | Aparência. O que faltar é sorteado pela `seed`. |
| `top_color`, `bottom_color`, `shoe_color`, `hair_color` | Nome de cor ou hex. |

`skin`: `fantasy` (padrão, tons de fantasia), `real` (tons realistas), um nome ou hex.

## Saída de `explain`

`items[].box` é `[x0, y0, x1, y1]` em pixels. `pixels_per_meter` diz o tamanho final. `warnings` repete os avisos do stderr.
