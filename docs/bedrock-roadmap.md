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
- [~] **1ª pessoa** (v70): cópia do jogador com `first_person` avalia o attachable do pack com `c.is_first_person=1` / `v.is_first_person` e o desenha na matriz da mão do jogo (`item_pose`), no lugar do modelo vanilla; `bedrock-debug.json`: `first_person_off`, `fp_offset` (blocos), `fp_rotate` (graus x,y,z), `fp_scale`. Falta: braço do pack (animações `first_person_*`), posição exata do osso `rightitem` em 1ª pessoa
- [ ] Item no inventário, hotbar, GUI e no chão (`is_in_ui`, `is_paperdoll`)
- [ ] Poção (frasco e líquido, textura original escondida)
- [~] Ferramentas (espada/machado dourados) invisiveis na mao: textura `dim` 1024x1024 fora do atlas (limite 1024x512); v45 reduz no conversor; falta confirmar no jogo
- [x] Cavalo, burro, coelho, axolote (v50, validado no jogo)
- [~] Blaze: animação de ataque do pacote (v51); jogador montado segue as animações da montaria (barco, vagoneta, cavalo, burro; v51)
- [ ] Barco no mundo com o modelo do pack: o A&S só traz `geometry.boat` nos attachables dos itens (`oak_boat`...), sem entidade de barco; usar esse modelo na entidade nativa
- [~] Jogador montado (cavalo, burro, vagoneta): pose por tipo da montaria (v51) e sem passo de caminhada (v53); falta ver assento/posição no jogo
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

## 8. Inventário A&S: o que falta integrar

Fonte: A&S 1.1.68 (`entity` 128, `attachables` 1642, `animations` 3492 + `animation_controllers` 948, `models` 693, `particles` 495, `render_controllers` 84, `textures` 4441) e o `.mcpack` 1.1.24 (subpacks `0_e`/`1_e`/`2_e`; o `2_e` leva 7552 texturas: blocks 5026, items 884, entity 767, ui 677, trims 53, gui 37, painting 23, `blocks.json`, `terrain_texture.json`, `flipbook_textures.json`). `scan_pack`: 4483 JSON, 199430 expressões Molang, 0 falhas; `audit_pack`: 0 chaves ignoradas. O que falta é semântica, não leitura.

### 8.1 Consultas Molang fixas em 0 (pack usa; motor responde constante)

- [ ] `is_owner_identifier_any` (5388 usos, 1642 attachables): o attachable herda do dono (`c.owning_entity->v.*`) só quando o dono é `player`/`armor_stand`; com 0 o ramo nunca roda (`v.nszoco=1` fixo); dono = tipo da entidade que segura (player, armor_stand, sulfur_cube, villager, piglin, allay, witch, copper_golem, pig, boat)
- [ ] `is_local_player` (175; jogador local em 1ª pessoa, `cslzxk`...), `is_spectator` (92), `is_emoting` (23), `cooldown_time`/`cooldown_time_remaining` (27), `relative_block_has_all_tags` (10)
- [ ] `hurt_direction` (430 usos; armor_stand, attachables): direção do golpe
- [ ] Estados de mob sem resposta: `is_sleeping` (gato, raposa, aldeão), `is_shaking` (107), `is_avoiding_mobs`/`is_avoiding_block` (aldeão, piglin, hoglin, esqueleto), `is_charging` (vex), `is_sonic_boom`, `is_emerging`, `is_digging` (warden, sniffer), `is_searching`, `is_sniffing`, `is_stalking` (raposa), `is_admiring` (piglin), `is_dancing` (allay), `is_resting` (morcego), `is_playing_dead` (axolote), `is_croaking`, `is_eating_mob` (sapo), `is_interested`, `is_shaking_wetness` (lobo), `is_casting`, `is_shield_powered` (wither), `has_dash_cooldown` (nautilus), `is_jump_goal_jumping` (breeze), `roll_counter`/`lie_amount`/`sneeze_counter` (panda), `tail_angle`, `body_x_rotation` (golfinho), `heartbeat_phase`
- [ ] `property`/`has_property` (87 usos): só `minecraft:armadillo_state` responde; faltam `climate_variant` (vaca, porco, galinha: 26), `chest_interaction` (22), `can_move` (9), `oxidation_level` (copper_golem), `has_nectar`/`has_flower` (abelha), `creaking_state`, `creaking_swaying_ticks`, `sulfur_cube_archetype`, `variant`
- [ ] `surface_particle_*` (28; partículas de bloco do warden), `texture_frame_index`, `mark_variant` fora do aldeão, `get_animation_frame` fora do relógio (bússola, bússola de recuperação)
- [ ] `is_in_ui` do motor lê só `is_in_gui`: ferramenta no inventário, hotbar e item caído (11747 usos) sem pose própria

### 8.2 Variáveis do jogo que o pack lê e o motor não define

- [ ] `v.stand_anim`, `v.raise_arms`, `v.gliding_speed_value` (elytra), `v.is_holding_spyglass`, `v.should_bow_head`, `v.use_blinking_animation`, `v.ischarging`, `v.lieonplayer` (gato), `v.decortextureindex`, `v.trim_path`, `v.offer_flower_tick` (golem de ferro), `v.is_patterned`, `v.glow_alpha`, `v.ground_particle_color`
- [ ] `v.is_enchanted` fixo em 0 (232 usos): brilho de encantamento nos itens
- [ ] `v.map_face_icon`, `v.has_trim`, `v.bob_animation`, `v.short_arm_offset_*`, `v.rail_rotation`, `v.liedownamount`: definidos em 0, sem valor do jogo

### 8.3 Animações

- [ ] Ids que o pack nomeia sem definir e o motor não traz: `animation.axolotl.play_dead`, `animation.ravager.idle_mouth`, `animation.player.sleeping`, `animation.player.look_at_target.ui`, `animation.armor.helmet.offset`, `controller.animation.persona.blink`
- [ ] Ids ofuscados sem definição no pack lido: `animation.iubeld` (evoker), `controller.animation.qqjxbi` (blaze); conferir no `2_e`/`0_e`
- [ ] 74 animações/controllers definidos e nunca ligados a entidade ou attachable do pack lido (provável conteúdo dos subpacks)
- [ ] Dependem das consultas de 8.1: jogador (729 animações; emote, `is_local_player`, recarga de item), warden (62), aldeão (82), piglin (110), lobo (94), panda (53), vex, golem de cobre (85)
- [ ] Elytra, escudo, arco/besta em uso, balde, bússola/relógio (seção 3)
- [ ] 1ª pessoa: ver "refino" no histórico (v75)

### 8.4 Texturas de item e bloco não exportadas

`convert_world` escreve `textures/items|blocks/<nome>` pelo nome Java; `java_item` cobre `wood_`/`gold_` das 5 ferramentas e `crossbow_standby`; `java_block` ~20 nomes. No `2_e`: 430 de 739 itens e 740 de 3014 blocos (sem contar `_mer`/`_mers`/`_normal`/`_heightmap`, PBR fora de escopo) caem fora.

- [ ] Itens por nome Bedrock: `record_*` → `music_disc_*`, `egg_*` → `*_spawn_egg`, `dye_powder_*` → `*_dye`, `beef_cooked`/`beef_raw`/`porkchop_*`/`mutton_*`/`chicken_*`/`rabbit_*`/`potato_baked` → `cooked_*`/`*`, `fish_*` → `cod`/`salmon`/`cooked_*`/`tropical_fish`/`pufferfish`, `bucket_*` → `*_bucket`, `boat_*`/`door_*`/`sign_*`/`bed_*` → `*_boat`/`*_door`/`*_sign`/`*_bed`, `minecart_*`, `book_*`, `map_*`, `redstone_dust` → `redstone`, `netherbrick` → `nether_brick`, `fireball` → `fire_charge`, `melon` → `melon_slice`, `reeds` → `sugar_cane`, `seeds_*` → `*_seeds`, `gold_*` armadura/cavalo → `golden_*`, `bow_standby`/`fishing_rod_uncast`, `apple_golden`, `carrot_golden`, `chain` → `iron_chain`, `bundle_*`, `quiver`
- [ ] Itens sem equivalente 1:1: `potion_bottle_*` (Java tinge `potion`), `clock_item`/`compass_item`/`recovery_compass_*`/`lodestonecompass_*` (atlas de 64 frames; Java: `clock_00`..`clock_63`, `compass_00`..`compass_31`), `tipped_arrow_*` (20 efeitos referenciados por attachables), `light_block_*`, `empty_armor_slot_*`
- [ ] Blocos por nome Bedrock: `planks_*`, `log_*`/`log_*_top`, `leaves_*_opaque`, `sapling_*`, `door_*_lower|upper`, `glass_<cor>`, `glass_pane_top_*`, `glazed_terracotta_*`, `concrete_powder_*`, `shulker_top_*`, `stonebrick*`, `grass_top`/`grass_side_*`, `flower_*`, `tallgrass`, `deadbush`, `coral_*`, `*_stage_N` (colheitas), `furnace_*`/`blast_furnace_*`/`smoker_*`, `chest_*`/`ender_chest_*`, `command_block_*`, `farmland_*`, `rail_*`, `bookshelf_*`, `*_ore_N`, `*_3d`, `hopper_*`, `cave_vines_*`, `creaking_heart_*`, `dried_ghast_state_*`, `copper_*_chain`/`chest_inventory_*`, `water_*_grey`, `quartz_block_*`
- [ ] Entradas de `terrain_texture.json` (chave Bedrock) entram por `java_block(chave)`: a chave também é nome Bedrock
- [ ] Pastas do `2_e` sem conversão: `ui` (677), `gui` (37), `trims` (53), `painting` (23), `map` (11), `font` (3), `misc`, `particles`, `caustic`, `models` (16), `environment` além de `sun`/`moon_phases`/`clouds`/`rain`/`snow` (`weather`, `destroy_stage_0..9`, `end_sky`, `end_portal_colors`), `colormap` além de `grass`/`foliage` (`birch`, `evergreen`, `swamp_*`, `dry_foliage`, `mangrove_swamp_foliage`), `blocks.json`
- [ ] Entidade/attachable aponta para textura do jogo que não existe em Java: `entity/spyglass`, `entity/loyalty_rope`, `entity/iron_golem/cracked_none`, `entity/wolf/wolf_armor_cracked_*`, `entity/cushion/*` e `entity/spear/*` (vêm do subpack)

### 8.5 Mobs por etiqueta (`q.is_name_any`)

4565 nomes em 4358 usos. Exato e com caixa (o pack lista `baby`/`Baby`); o motor casa `custom_name` por igualdade.

- [~] Mobs por nome: `baby`/`foal`/`calf`/`fry`/`pup`, `halloween` (50 entidades), `cherry`, `frosty`, `snowy`, `dappled`, `pale`, `festive`, `sulfur`, `skull`, `snowman`, `bunny`/`killer bunny`/`thumper`, `wendigo foal`, `baby zombified`/`baby skeleton`/`baby executioner`, `rumble`, `villager`, `brian`, `glare`, `green`/`beanie`/`magician` (enderman), `blossom`/`poplar` (creeper), `pup` — validar um a um no jogo; ligados ao mob que o jogo conhece pelo nome (`custom_name`), não à tag `/tag`
- [ ] Props e itens por nome: armor stand ou cubo de enxofre chamado `Prop`/`prop`/`§§` ou `Item`/`item`/`§§§§` vira vitrine de qualquer bloco/item (1738 attachables: `v.eclqlw` e `v.jvsiul`); exige `is_owner_identifier_any` (8.1) e armor stand vestido pelo pack (8.6)
- [ ] `get_name` (12 usos) devolve `custom_name`; nome como componente com cor/estilo não passa pela igualdade

### 8.6 Entidades do pack sem desenho do pack

Só `Mob` e jogador passam por `has_entity`. Fora do caminho:

- [ ] `minecraft:armor_stand` (76 animações, props por nome), `minecart` + `chest`/`hopper`/`tnt`/`command_block`, barcos (só attachables dos itens), `arrow`, `thrown_trident`, `fireball`/`small_fireball`/`dragon_fireball`, `wither_skull(_dangerous)`, `snowball`, `egg`, `ender_pearl`, `eye_of_ender_signal`, `fireworks_rocket`, `xp_orb`, `xp_bottle`, `splash_potion`, `lingering_potion`, `wind_charge_projectile`, `shulker_bullet`, `evocation_fang`, `ender_crystal`, `fishing_hook` (Java `fishing_bobber` fora de `BEDROCK_IDS`), `cushion`
- [ ] Item caído, item em moldura, item no chão e GUI com attachable do pack (`is_in_ui`, `is_paperdoll`)
- [ ] Subpacks: `n13p` não traz `subpacks/*`; conferir qual subpack o jogo carrega (`SP0` custom, `SP1` vanilla, `SP2` full) e rodar `scan_pack` com o subpack

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
| v50 `defs-rabbit` | várias definições da mesma entidade no pack (um arquivo por `min_engine_version`, cavalo e burro têm 3): vale a de maior versão; `this` de `color` do render controller = 1 (cavalo saía com alfa 0 offline); `q.vertical_speed` parado no chão = 0 (era −1,57 pela gravidade e prendia o estado `fall` de cavalo, burro, axolote); escala embutida do coelho 0,6 (bebê 0,4); blaze: `q.is_charged` = fogo do blaze (flag 16 bit 0 do servidor, `extra.flag` local) e `q.facing_target_to_range_attack` com alvo | validado (cavalo, burro, coelho, axolote) |
| v51 `mount-blaze` | `q.is_riding_any_entity_of_type` responde pelo tipo do veículo (barco, barco com baú, vagoneta, mobs; `riders::set_mount_kinds` a cada tick, `set_player_mount` para o jogador); blaze de servidor: `has_target`/`facing_target_to_range_attack` ficam 120 ticks após a carga (sem alvo no cliente o controller voltava a `default` a cada 0,1 s) | pendente (jogo) |
| v52 `blaze-volley` | blaze: `is_charged` só nos 60 ticks iniciais da carga (a rajada com a flag ainda ligada é o ataque do pacote, `!is_charged && facing_target`); alvo/`facing_target_to_range_attack` por 100 ticks após a flag cair (`nether::set_blaze_charged`, `blaze_charging`, `blaze_attacking`) | pendente (jogo) |
| v53 `rider-limbs` | passageiro: `modified_move_speed`, `ground_speed`, `modified_distance_moved` = 0 (o jogador montado balançava braços/tronco com a velocidade da montaria) | pendente (jogo) |
| v54 `cart-seat` | jogador na vagoneta: `previous_position` = assento do tick anterior da vagoneta (`c.previous`), não a posição já com a gravidade do tick (o modelo tremia para cima e para baixo) | pendente (jogo) |
| v55 `rider-seat` | jogador montado (cavalo, burro, etc., local e servidor): `previous_position` = último assento (`rider_seat`) e não a posição do jogador já com a gravidade do tick (o modelo tremia para cima e para baixo parado) | pendente (jogo) |
| v56 `player-ground` | modelo do jogador: `on_ground` do movimento (antes sempre falso → `vertical_speed` = gravidade → estado "queda" do pack, no jogador e no axolote do balde); passageiro com `vertical_speed` 0 | pendente (jogo) |
| v57 `rider-trace` | `bedrock-debug.json` `trace_rider: true`: escreve `rider-trace.log` (pasta do jogo, não o console): `rider-trace-frame` (alturas/yaws da montaria e do modelo do jogador por quadro), `rider-trace-tick` (assento por tick) e, em v58, `rider-trace-pose` (ossos da montaria e do rider por quadro). Medido: parado, altura/assento estáveis em 18 476 de 18 521 quadros (só o embarque varia) | validado (medição) |
| v58 `rider-body` | corpo do jogador sobre montaria viva segue o corpo da montaria (até 85° da cabeça; além, acompanha a cabeça + 0,2), como `LivingEntityRenderer`; antes ficava 50° fora (`body_diff` = 50 no trace) | validado (jogo) |
| v59 `bone-damp` | `pre_animation` lê `q.bone_origin` do quadro anterior pela média dos dois últimos quadros: `root` do rider = altura das pernas (`rdabcy`: `max(v.odekyc, v.ektjzx)`) realimentava o próprio deslocamento com sinal trocado e alternava ~2 px por quadro (trace: 0,81 ↔ 2,92). `rider-trace.log` passa para a pasta `logs` | validado (jogo) |
| v60 `trace-mount` | `trace_rider`: `rider-trace-pose` também da montaria (antes só quando `rider` setado: camelo/husk ficavam de fora). Camelo: assento sobe de 0,63 a 1,40 em ~37 ticks (= `getBodyAnchorAnimationYOffset` do vanilla, `pose_time` 15→52) | pendente (jogo) |
| v61 `enderman-angry` | `q.is_angry` do enderman = flag creepy (alvo/encarar); camelo: `q.is_sitting`/sit_amount seguem `camel::sitting` (animações deitar/levantar) | pendente (jogo) |
| v62 `pass-bias` | passes de material dentro de uma camada (olho emissivo vs pálpebra) empurravam 0,002 bloco (> 0,0016 entre pálpebra z-4 e pupila z-3,975): agora 0,0002 por passe; 0,002 só entre camadas | pendente (jogo) |
| v63 `camel-seat` | assento do camelo = altura em pé + deslocamento de `root`+`body` do modelo (log: assento vanilla subia em 52 ticks, animação do pack em ~23 e 0,09 mais alto → jogador dentro do corpo); sem modelo de pack segue o vanilla | pendente (jogo) |
| v64 `glow-bias` | segundo desenho `_glow` (texels emissivos) subia 0,004 bloco (> 0,0014 entre pálpebra e pupila): a pupila do enderman aparecia por cima da pálpebra na piscada; agora sobe só 0,0002 sobre a própria face | pendente (jogo) |
| v65 `camel-standing` | assento do camelo sentado ficava sob o corpo: altura em pé usava a caixa atual (−1,43 sentado); agora usa a caixa fixa de 2,375 + queda do osso `root`/`body` | pendente (jogo) |
| v66 `cart-seat` | jogador no minecart ficava afundado (queixo na borda); assento sobe 0,25 bloco (v66), 0,35 (v67: pés vazavam sob o piso), 0,47 (v68) | pendente (jogo) |
| v69 `arm-join` | braços do jogador com 1 px de folga no tronco (`humanoid.custom`/`customSlim` com origem em ±5/−9, ±8): agora encostados (esq. 4; dir. −8 clássico, −7 magro; mangas junto) | pendente (jogo) |
| v70 `first-person` | item na mão em 1ª pessoa: attachable do pack (`c.is_first_person=1`) na pose de mão do jogo; ajuste por `fp_offset`/`fp_rotate`/`fp_scale`; `first_person_off` volta ao modelo do jogo | pendente (jogo) |
| v71 `first-person-angle` | item em 1ª pessoa deitado para a frente (modelado no quadro do braço pendente): padrão `fp_rotate` 60° em x e `fp_scale` 0,6 | pendente (jogo) |
| v72 `first-person-upright` | ferramenta longa e chata (espada, tridente, lança) desenha uma vez para achar os eixos e fica em pé, face para a câmera, ponta inclinada 20°; `fp_scale` padrão 0,8; `fp_rotate` vira ajuste extra (padrão 0). Bedrock mostra espada e tridente de frente, o pack as modela deitadas no quadro do braço | pendente (jogo) |
| v73 `first-person-log` | a pose em pé ignora vértices aditivos (plano de brilho do pack) ao achar os eixos; log da 1ª pessoa repete quando o tamanho muda e diz se ficou em pé | pendente (jogo) |
| v74 `first-person-hand` | padrões da mão em 1ª pessoa tirados dos prints (Bedrock x Horizon): `fp_offset` [-0,12, -0,12, 0], `fp_rotate` [0, 0, -14] (ferramenta em pé inclinava para a esquerda) | pendente (jogo) |
| v75 `first-person-pack-roll` | 1ª pessoa sem reorientar por item: só pitch fixo (ponta para cima, 20° de inclinação); giro do item vem das animações do pacote (tridente de frente, espada e lança de lado); `fp_rotate` padrão [0, 0, -6] | pendente (jogo) |
| v75 (melhoria) `first-person-refino` | 1ª pessoa: v75 próximo do Bedrock, mas não ok; pausado. Refinar: inclinação por item (espada ~11° à direita, tridente/lança ~4°), posição, escala, mão/braço do pacote (`first_person_*`), mão secundária | aberto |

- [x] Enderman (v62): o ponto central do olho (camada de olho aberto) fica uma camada à frente dos olhos fechados; ao piscar/expressar, o fechado não o cobre (reportado v59)
