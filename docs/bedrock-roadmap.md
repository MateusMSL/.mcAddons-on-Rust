# Bedrock add-ons: o que falta

Alvo: ler `.mcpack`, `.mcaddon` e pastas de qualquer autor, sem ajuste por pack. A&S 1.13 é só o pack de teste
(checklist em [`bedrock-validation.md`](bedrock-validation.md)). Matriz tecnica em [`bedrock-coverage.md`](bedrock-coverage.md).

Legenda: `[x]` feito e validado · `[~]` feito, validação pendente ou com bug aberto · `[ ]` não feito.

Atualizar a cada build: marcar o item, anotar a build (`ENGINE_BUILD`) e o resultado da validação.

## 1. Formatos e instalação

- [x] `.mcpack`, zip, pasta com `manifest.json`, `.brarchive`
- [x] Subpacks (`subpacks`) e `settings` (toggle, dropdown, multiselect, slider), tela **Settings**, `config/horizon-addons.json`
- [x] `texts/*.lang`, `splashes.json`
- [x] `.mcaddon`: resource packs lidos (pastas e `.mcpack` aninhados), behavior pack ignorado (`validate_install`: 7 contêineres idênticos à pasta; Naturalist `.mcaddon` real: RP lido, BP ignorado)
- [x] `dependencies` entre packs: ordem de carga e aviso de pack faltando no log
- [x] Vários packs Bedrock ativos: dados de cada pack em `assets/horizon_bedrock/<uuid>/` (`materials/entity.material` e `sounds.json` de um pack não apagam os do outro); `order.txt` do overlay lista os packs do mais baixo ao mais alto, e o de cima vence o identificador que os dois definem; `settings` por pack (revisão 7, v33)
- [x] `.mctemplate` / `.mcworld`: `resource_packs/*` lidos
- [x] Packs criptografados (`contents.json` cifrado): recusados com erro `Encrypted` nomeado no log
- [x] `min_engine_version` / `format_version` novos: aviso no log

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

- [x] Molang: parênteses com lista de comandos (`( v.a = 4; v.b = 2; v.a * v.b; )`, estilo Blockbench) valem o último comando; 7911 expressões do Naturalist parseiam (v31)

## 3. Attachables (itens na mão e vestidos)

- [x] `parent_setup` roda na entidade que segura (v24)
- [x] Item na mão em 3ª pessoa: osso `rightitem`/`leftitem`, `binding`, filhos seguem o pai
- [~] Item na mão: modelado ao redor da origem, que cai no osso `rightitem` inteiro (pivô incluído), com o recuo de 24 do modelo cancelado (`held_anchor`); peças vestidas seguem só a variação (v33; conferir no jogo, F5)
- [~] Tridente: log mostra controlador do player e ossos `rightarm`/`rightitem` corretos; causa: o osso preso (`pole`, pivô em y 24) ancorava a origem do modelo na mão e subia 24 px; v43 ancora o pivô do osso preso na mão; falta confirmar no jogo
- [~] Brilho billboard (`oreville_auulis`: disco `cim` da aranha, enderman...): aditivo (v37) e olhos do `.tga` sólidos (v38); em certos ângulos o disco aparece de lado (linha fina) ou como mancha no chão; números de `hcmvys`/`rdmkec`/`habcyx` batem com o esperado, falta referência do Bedrock; `probe_facing` (offline) confirma que o giro e a inclinação do disco seguem o mesmo sentido da cabeça em todos os ângulos; falta referência do Bedrock (prints) para achar a diferença restante
- [~] Armadura vestida: ossos de mesmo nome seguem o osso do portador; camadas sobrepostas deslocadas (v25)
- [~] `texture_meshes` com sprite do jogo (`mesh_variant`): item plano e `use_pixel_depth`
- [x] Mace, besta, tridente, bola de vento, espada e ovos de spawn na mão (v49 `held_anchor: auto`, validado no jogo; osso sem pai → origem do modelo, com pai → pivô)
- [ ] **1ª pessoa**: `c.is_first_person=1`, mão e item no espaço da câmera. Hoje só o sprite novo do item aparece
- [ ] Item no inventário, hotbar, GUI e no chão (`is_in_ui`, `is_paperdoll`)
- [ ] Poção (frasco e líquido, textura original escondida)
- [~] Ferramentas (espada/machado dourados) invisiveis na mao: textura `dim` 1024x1024 fora do atlas (limite 1024x512); v45 reduz no conversor; falta confirmar no jogo
- [~] Cavalo, burro, coelho, blaze, axolote (v50): ver a linha v50 da tabela; conferir no jogo (cavalo/burro animam, coelho com o tamanho do vanilla ×1,08, axolote em terra sem ficar esticado, blaze com fogo e ataque do pacote)
- [~] Validacao de mobs (A&S, v45): ovelha com olho bugado (1 olho com pupila, outro castanho); burro e cavalo sem animacao; coelho gigantesco (escala adulta 1.8 do pacote, geometria 18 px); axolote invocado em terra fica de pe e inclinado (na agua ok); baiacu sem forma inflada; allay e vex com bordas das camadas externas bugadas; cubo de enxofre invocado afunda na terra; boneco de neve sem variacoes; lhama do mercador sem manta; chapeu do aldeao fazendeiro com a cabeca sobre o chapeu; cabeca do tatu piscando; cavalo esqueleto com sela sem sela (`q.is_saddled`); breeze perde o tornado depois de segundos e nao volta; fumaca do phantom colada nele; besta do pillager mal posicionada (provavel no player tambem); galinhas do stun do ravager mal posicionadas; blaze sem fogo proprio e sem animacao de ataque; ghast sem animacao de ataque e sem bola de fogo da textura; shulker preta (talvez textura do pacote); golem de cobre e creaking ok, so o glow dos olhos
- [x] `dump` v46 loga tambem is_saddled, is_baby, is_tamed, has_rider, is_riding, is_charged, is_angry, is_sheared, is_sleeping, is_sitting, is_alive, is_on_fire, is_attached, variant, mark_variant, skin_id, body rotations, yaw_speed, life_time
- [ ] Elytra, escudo, tridente, arco/besta em uso, balde, bússola/relógio animados
- [~] Cabeças e `carved_pumpkin` no slot `head`: `HOLD=... SLOT=head` no `render_entity` (osso `head`, mudança desde o repouso); offline o A&S não desenha (a escala lê `q.is_pack_setting_selected`, sem resposta fora do jogo); falta ver no jogo
- [ ] Trims e tinta de armadura de couro

## 4. Materiais e desenho

- [x] Alpha test, blend, aditivo, dupla face, emissivo, `_glow`, stencil, `USE_UV_ANIM`, glint, inversão de culling
- [~] `DisableDepthWrite`: pipeline próprio (blend, teste de profundidade, sem escrita) (v29)
- [~] `DisableDepthTest`: pipeline sem teste nem escrita, desenhado sobre o resto do mundo; com transparência melhorada ligada entra no estágio dela e mantém o teste (v29)
- [~] `MASKED_MULTITEXTURE`/`*_multitexture*`: 2ª textura do render controller vira camada por cima da 1ª, com blend e sem escrita de profundidade (v29); texturas extras de material comum seguem desenhadas como antes
- [ ] `.material.bin` e shaders compilados: fora de escopo

## 5. Recursos que não são entidade

- [x] Texturas de blocos e itens (`textures/blocks`, `items`) escritas como recurso Java; céu, color maps
- [~] `terrain_texture.json` e `item_texture.json` (arquivo que o bloco/item usa, com qualquer nome) e `flipbook_textures.json` (`.png.mcmeta` com `frametime`, `frames`, `interpolate`): v48, conversor revisão 11, testado no pack sintético; falta ver água/lava/fogo animados no jogo
- [ ] `blocks.json` (som, brilho e textura por bloco)
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
- [x] Captura offline de attachable: `render_entity` com `HOLD=<attachable>` (`SLOT=`, `ANCHOR=auto|pivot|frame`) desenha o jogador com o item na mão e imprime onde o item fica em relação à mão (v48)
- [~] Teste de regressão com pack sintético: `tests/synthetic_pack.rs` escreve uma pasta de pack e passa por abrir, converter, carregar e rodar (entidade, geometria, render controller, animação, controller, attachable, controller e animação com o mesmo nome curto); falta um pack mínimo para cada item restante do roadmap
- [x] Segundo pack de referência: o resource pack vanilla do Mojang (`Mojang/bedrock-samples`, sem texturas) passa no `scan_pack` sem falha (v48 corrigiu: blocos Molang em várias linhas de um array, controller que toca animação de mesmo nome curto, `scaleX/Y/Z` em maiúsculas)

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
| v30 `emissive-solid` | material emissivo (`entity_emissive*`, `EMISSIVE`): o alfa da textura é brilho, não transparência; `_solid.png` gerado na conversão (revisão 5) e usado no corpo do modelo (charged creeper translúcido, lentes do óculos); item na mão com attachable do pack não desenha o modelo do jogo (3ª pessoa) | pendente (jogo) |
| v31 `install-matrix` | `.mcworld`/`.mctemplate`, `.mcpack` aninhado em `.mcaddon`, nomes com `\\`, dependências, aviso de engine nova, recusa de pack criptografado, `settings` por pack (revisão 6); exemplo `validate_install` | offline ok; jogo pendente |
| v32 `held-origin` | item na mão (`binding` do slot) usa a pose inteira do osso do portador (antes: variação do repouso, item caía nos pés); Molang com lista de comandos entre parênteses; `HORIZON_BEDROCK_HIDE=<texturas>` esconde camadas para diagnóstico | pendente (jogo) |
| v33 `pack-folders` | vários packs Bedrock ativos não se sobrescrevem (pasta por UUID + `order.txt`); causa do casco ciano do creeper carregado: o `entity.material` do Naturalist substituía o do A&S; item na mão ancorado no osso com o recuo de 24 do modelo (`held_anchor`) | pendente (jogo) |
| v34 `held-blocks` | item na mão: o recuo de 24 do modelo estava em unidades erradas (24 blocos em vez de 24/16: arco do esqueleto 22 blocos acima); teste com o `posed` real e sonda `sword_probe` (espada de A&S no jogador: 0,85 a 2,47 acima dos pés, perto da mão); log de debug dá a caixa do item em blocos desde os pés e a posição do osso da mão | pendente (jogo) |
| v35 `glow-alpha` | `_solid` só para material emissivo sem blend nem aditivo (casco do creeper); brilho que mistura ou soma (olhos, aura do enderman/aranha) volta a usar o alfa da textura (v30 o deixava opaco: quadrados e discos chapados) | pendente (jogo) |
| v36 `debug-file` | `bedrock-debug.json` recarregado em jogo a cada segundo (hide, dump, dump_vars, flags de material por nome, queries fixas; chave errada = erro no log, valor antigo mantido); modelo criado com `HORIZON_BEDROCK_DEBUG=1`; configurações de execução do IntelliJ em `.run/` (jogo, exemplos, testes); exemplo `list_glow` | offline ok |
| v37 `glow-additive` | material com `blendSrc SourceAlpha`/`blendDst One` (`oreville_auulis`: brilho de aranha, enderman, zumbi, wither...) soma `cor × alfa` ao fundo em pipeline próprio (bit `ADDITIVE`, material 12 do lote, nos dois modos de transparência), em vez de cobrir com a textura por mistura de alfa; camada `_glow` extra não é mais desenhada para ela | pendente (jogo) |
| v38 `tga-faint` | textura `.tga` com texel de alfa 1 a 8 (olhos da aranha do A&S, alfa 3) agora vira sólido como em `.png` (antes sumia no jogo e o disco de brilho aparecia por trás); revisão do conversor 8 | pendente (jogo) |
| v39 `faint-filler` | textura com mais texels de alfa 1 a 8 que sólidos (roupas de profissão do vilager em `.tga`: branco com alfa 1 como fundo) não é mais levantada; só o olho (poucos texels fracos) vira sólido; `list_glow` lista também attachables (arnês do ghast feliz usa `oreville_auulis`); revisão do conversor 9 | pendente (jogo) |
| v40 `secondary-texture` | 2ª textura de um render controller só desenha com material multitexture (ferramentas na mão desenhavam `dpw`/`black` por cima do sprite e ficavam escuras) | pendente (jogo) |
| v41 `alpha-phantom` | material sem blend, soma nem teste de alfa ignora o alfa da textura (`_solid`): creeper carregado fica opaco com rosto branco (`cro`: corpo alfa 157, rosto alfa 2); phantom de pacote perde o deslocamento de 1,3125 do modelo Java (ficava um bloco abaixo); `list_glow` mostra `at` | pendente (jogo) |
| v42 `dump-bones` | `dump_bones` (`HORIZON_BEDROCK_DUMP_BONES`) lista a pose dos ossos pelo nome (ate 80), mesmo parados: tridente (`rightarm`, `rightitem`, `pole`) | pendente (jogo) |
| v43 `held-pivot` | item na mão: o pivô do osso preso cai na mão (`held_anchor_at`, `rest_pivot`); osso com pivô na origem mantém a âncora anterior; tridente (pivô y 24) deixa de aparecer 24 px acima | pendente (jogo) |
| v44 `held-skips` | debug: camada de item em mao descartada (geometria ausente, alfa 0, textura fora do atlas, 2a textura) entra no log; ferramentas invisiveis em v43 | pendente (jogo) |
| v45 `atlas-fit` | conversor (rev 10): textura acima de 1024x512 (limite do atlas, que a descartava calada) reduzida por fator de 2 com media; espada/machado dourados (`dim` 1024x1024) voltam; 17 texturas do A&S | pendente (jogo) |
| v46 `dump-queries` | `dump` loga mais queries (sela, bebe, domado, cavaleiro, variante...) para a lista de bugs de mobs | pendente (jogo) |
| v47 `dump-pose` | `dump` loga `touched` (ossos tocados pelas animacoes) e `max_rot`: a maioria dos mobs do A&S mostra `bones:` vazio no jogo mesmo andando (cavalo, burro, coelho, ovelha, lhama, breeze, blaze, axolote); separa animacao que nao roda de animacao sem o que mover | pendente (jogo) |
| v48 `held-frame` | `held_anchor` no `bedrock-debug.json` (mace/besta ~1,4 bloco acima da mão com o pivô na mão); `HOLD=` no `render_entity`; `tests/synthetic_pack.rs`; vanilla do Mojang como 2º pack: Molang em várias linhas, controller que toca animação de mesmo nome curto (antes recursão infinita no `scan_pack`), `scaleX/Y/Z`, corte de controller que toca a si mesmo ; conversor 11: `terrain_texture.json`, `item_texture.json`, `flipbook_textures.json` | pendente (jogo) |
| v49 `held-auto` | `held_anchor: auto` (padrão): osso preso sem pai → origem do modelo na mão, com pai → pivô; corrige ovos de spawn; `ANCHOR=auto` no `render_entity` | validado |
| v50 `defs-rabbit` | várias definições da mesma entidade no pack (um arquivo por `min_engine_version`, cavalo e burro têm 3): vale a de maior versão; `this` de `color` do render controller = 1 (cavalo saía com alfa 0 offline); `q.vertical_speed` parado no chão = 0 (era −1,57 pela gravidade e prendia o estado `fall` de cavalo, burro, axolote); escala embutida do coelho 0,6 (bebê 0,4); blaze: `q.is_charged` = fogo do blaze (flag 16 bit 0 do servidor, `extra.flag` local) e `q.facing_target_to_range_attack` com alvo | pendente (jogo) |
