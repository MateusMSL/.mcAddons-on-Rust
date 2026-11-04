# Bedrock add-ons: o que falta

Alvo: ler `.mcpack`, `.mcaddon` e pastas de qualquer autor, sem ajuste por pack. A&S 1.13 é só o pack de teste
(checklist em [`bedrock-validation.md`](bedrock-validation.md)). Matriz tecnica em [`bedrock-coverage.md`](bedrock-coverage.md).

Legenda: `[x]` feito e validado · `[~]` feito, validação pendente ou com bug aberto · `[ ]` não feito.

Atualizar a cada build: marcar o item, anotar a build (`ENGINE_BUILD`) e o resultado da validação.

## 1. Formatos e instalação

- [x] `.mcpack`, zip, pasta com `manifest.json`, `.brarchive`
- [x] Subpacks (`subpacks`) e `settings` (toggle, dropdown, multiselect, slider), tela **Settings**, `config/horizon-addons.json`
- [x] `texts/*.lang`, `splashes.json`
- [~] `.mcaddon` (vários packs num zip): resource pack lido, behavior pack ignorado. Verificar com um `.mcaddon` real
- [ ] `dependencies` entre packs (ordem e aviso de pack faltando)
- [ ] Vários packs Bedrock ativos ao mesmo tempo (merge por ordem de prioridade; hoje cada um vira um pack Java)
- [ ] `.mctemplate` / `.mcworld` (packs embutidos no mundo)
- [ ] Packs criptografados (`contents.json` com chave): recusar com mensagem clara
- [ ] `min_engine_version` / `format_version` novos: aviso no log em vez de falha silenciosa

## 2. Entidades (client entity)

- [x] Geometria, texturas, materiais, render controllers, animações e controllers, Molang, partículas, sons
- [x] Variantes de bebê, nome (`name tag`), bioma, mundo (`relative_block_has_any_tag`, `time_of_day`)
- [x] Jogador na skin do jogo (modelo wide e slim); `HORIZON_BEDROCK_PLAYERS=off` desliga
- [~] Animações de andar/correr do jogador (`ground_speed` por deslocamento do tick, v24)
- [~] Preview do jogador no inventário (`q.is_in_ui`, `v.is_paperdoll`): estado separado do mundo (v26)
- [~] Nado rápido (`v.jbawka` com `asin` fora do domínio): NaN vira 0 (v26)
- [~] Linhas nas bordas das faces (villager): UV 0,05 texel para dentro (v26: 0,01 não bastou)
- [~] Slot `body` (arreio do happy ghast, armadura de cavalo/lobo): attachable desenhado (v27)
- [~] Capa do jogador (variáveis calculadas por `q.position` e `q.life_time`; laje projetada em v24)
- [x] Geometrias vanilla embutidas: `humanoid.custom`, `customSlim`, `cape`, `elytra`, `shield`, `evocation_fang` (com `animation.evocation_fang.bite`: mandíbulas abrem e fecham, coluna sobe; renderizada offline), `bed` (UV da cama não conferida)
- [~] Projéteis e itens soltos: render offline de `minecart`, `splash_potion`, `snowball`, `egg`, `xp_orb`, `evocation_fang` ok; `arrow` e `fishing_hook` usam textura do jogo (`java_texture`), só conferem no jogo
- [~] Entidades montadas: `q.ride_body_y_rotation`, `ride_head_y_rotation`, `ride_head_x_rotation` valem a pose do veículo no passageiro (v29); `rider_*` (veículo lê o passageiro) seguem 0

## 3. Attachables (itens na mão e vestidos)

- [x] `parent_setup` roda na entidade que segura (v24)
- [x] Item na mão em 3ª pessoa: osso `rightitem`/`leftitem`, `binding`, filhos seguem o pai
- [~] Armadura vestida: ossos de mesmo nome seguem o osso do portador; camadas sobrepostas deslocadas (v25)
- [~] `texture_meshes` com sprite do jogo (`mesh_variant`): item plano e `use_pixel_depth`
- [ ] **1ª pessoa**: `c.is_first_person=1`, mão e item no espaço da câmera. Hoje só o sprite novo do item aparece
- [ ] Item no inventário, hotbar, GUI e no chão (`is_in_ui`, `is_paperdoll`)
- [ ] Poção (frasco e líquido, textura original escondida)
- [ ] Elytra, escudo, tridente, arco/besta em uso, balde, bússola/relógio animados
- [ ] Cabeças e `carved_pumpkin` no slot `head`
- [ ] Trims e tinta de armadura de couro

## 4. Materiais e desenho

- [x] Alpha test, blend, aditivo, dupla face, emissivo, `_glow`, stencil, `USE_UV_ANIM`, glint, inversão de culling
- [~] `DisableDepthWrite`: pipeline próprio (blend, teste de profundidade, sem escrita) (v29)
- [~] `DisableDepthTest`: pipeline sem teste nem escrita, desenhado sobre o resto do mundo; com transparência melhorada ligada entra no estágio dela e mantém o teste (v29)
- [~] `MASKED_MULTITEXTURE`/`*_multitexture*`: 2ª textura do render controller vira camada por cima da 1ª, com blend e sem escrita de profundidade (v29); texturas extras de material comum seguem desenhadas como antes
- [ ] `.material.bin` e shaders compilados: fora de escopo

## 5. Recursos que não são entidade

- [x] Texturas de blocos e itens (`textures/blocks`, `items`) escritas como recurso Java; céu, color maps
- [ ] `blocks.json`, `terrain_texture.json`, `item_texture.json`, `flipbook_textures.json` (animação de textura)
- [ ] `biomes_client.json`, fogs, `sky`, água e folhagem por bioma
- [ ] JSON UI (`ui/`): título, menus, HUD
- [ ] `sounds.json` de blocos e UI (sons de entidade já lidos)
- [ ] Partículas: auditoria de semântica com um pack que não é o A&S

## 6. Behavior pack (depois)

- [ ] Leitor de `entities/`, `items/`, `blocks/`, `recipes/`, `loot_tables/`, `functions/`, `trading/`
- [ ] Importador para `.hmod` (comportamento em Rust/Java do Horizon, sem rodar script do pack)
- [ ] Scripting API (`@minecraft/server`): fora de escopo

## 7. Ferramentas e validação

- [x] `audit_pack`, `scan_pack`, `convert_pack`, `inspect_entity`, `render_entity` (variáveis `Q=`, `WALK=`, `BONES=`, `DESCRIBE=`), `probe_attach`
- [x] Log `HORIZON_BEDROCK_DEBUG=1`, dump de queries e variáveis
- [ ] Captura offline de attachable (`render_attachable`) para ver item na mão sem abrir o jogo
- [ ] Teste de regressão com pack sintético por recurso (um pack mínimo por item deste roadmap)
- [ ] Segundo pack de referência (pack pequeno de terceiros) para separar bug do engine de bug do A&S

## Histórico de builds

| Build | Mudança | Validação |
| --- | --- | --- |
| v23 | texturas de mundo/itens, glint, culling, queries documentadas | itens ok (sprites), bebê ok |
| v24 `parent-setup` | `parent_setup` no portador, ossos de mesmo nome, `head_y_rotation` absoluto, `ground_speed` | F5 gira certo; item 1ª pessoa, poção, armadura com bug |
| v25 `layer-bias` | camadas de attachable deslocadas, olho da abelha, `is_in_water` | abelha ok; armadura ainda com bug; preview girando |
| v26 `gui-uv-nan` | preview de tela com estado próprio e `is_in_ui`/`is_paperdoll`=1; `Math.*` sem resultado vale 0 (nado rápido de cabeça para baixo); UV de cada face 0,01 texel para dentro (linhas nas bordas do villager) | pendente |
| v27 `body-slot-swim` | slot `body` (arreio do ghast, armadura de cavalo) vira attachable; `is_swimming` do jogador = pose de nado; `ground_speed` 3D no nado (inclinação do corpo = pitch); UV 0,05 texel | pendente |
| v28 `swim-diag` | `is_swimming` volta a valer "na água" (nado sem correr) ou pose de nado; dump ganha `ground_speed`, `vertical_speed`, `is_swimming`... | pendente |
| v29 `materials-entities` | `DisableDepthWrite`/`DisableDepthTest` e multitexture desenhados; `geometry.evocation_fang` + `bite` e `geometry.bed` embutidas; `ride_*` | pendente (jogo) |
