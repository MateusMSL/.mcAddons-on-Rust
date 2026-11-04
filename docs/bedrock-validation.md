# Actions & Stuff 1.13: checklist de validação

Pack de teste do engine Bedrock. Cada linha é um recurso do A&S que o engine já usa.
Rodar com `HORIZON_BEDROCK_DEBUG=1`; marcar aqui após validar em jogo.

`[x]` validado em jogo · `[~]` aparece mas com defeito (nota) · `[ ]` pendente

Números do pack: 128 client entities, 1642 attachables, 3492 animações, 693 geometrias, 84 render controllers,
495 partículas, 4445 texturas, 18 settings, 3 subpacks.

## Instalação

- [ ] Pack aparece na lista e ativa sem erro
- [ ] Subpacks: *Custom Experience*, *Vanilla Experience*, *Full Experience* trocam o visual
- [ ] Tela **Settings** com os 18 itens (10 dropdowns/multiselects e 8 toggles) e **Done** recarrega
- [ ] `config/horizon-addons.json` guarda a escolha
- [ ] Log sem `Bedrock file ... does not parse`

## Mobs (128 client entities)

Marcar por grupo; anotar a entidade que falhar.

- [~] Passivos: pig, cow, sheep, chicken, rabbit, fox, wolf, cat, ocelot, panda, polar_bear, goat, frog, turtle, armadillo, sniffer, camel, horse/donkey/mule, llama (bebê ok; adultos a conferir)
- [x] Abelha: bebê e adulta ok (v25)
- [ ] Abelha: asas; ferrão
- [x] Allay: olho visível
- [ ] Aquáticos: cod, salmon, pufferfish, tropicalfish, squid, glow_squid, dolphin, axolotl, guardian, elder_guardian, nautilus
- [ ] Mortos-vivos: zombie, husk, drowned, skeleton, stray, bogged, parched, wither_skeleton, zombie_villager, zombie_nautilus, camel_husk, zombie_horse, skeleton_horse
- [ ] Hostis: creeper (carregado), spider, cave_spider, enderman, endermite, silverfish, slime, magma_cube, blaze, ghast, happy_ghast, phantom, vex, witch, pillager, vindicator, evoker, ravager, shulker, warden, breeze, creaking
- [ ] Bosses: ender_dragon, wither, ender_crystal
- [ ] Nether: piglin, piglin_brute, hoglin, zoglin, strider, zombie_pigman
- [ ] Villager e wandering_trader (profissões, níveis), iron_golem, snow_golem, copper_golem
- [ ] Armor stand, cushion, sulfur_cube
- [~] Happy ghast: arreio (`slot.armor.body`), v27
- [~] Projéteis (offline v29: snowball, egg, xp_orb, evocation_fang ok; arrow e fishing_hook dependem da textura do jogo): arrow, snowball, egg, fireball, small_fireball, dragon_fireball, wind_charge, shulker_bullet, wither_skull, fishing_hook, ender_pearl, eye_of_ender, fireworks_rocket, thrown_trident
- [~] Poções e frascos (offline v29: splash_potion ok): splash_potion, lingering_potion, xp_bottle, xp_orb
- [~] Minecarts (offline v29: minecart ok): minecart, chest, hopper, tnt, command_block

## Comportamento visual dos mobs

- [~] Bebê: escala e `baby_transform` (ok em abelha, vaca, zumbi)
- [x] Piscar: abelha
- [ ] Piscar: villager, gato, raposa
- [~] Villager: linhas finas nas bordas das faces (UV 0,05 texel para dentro, v27)
- [ ] Morte (giro e queda orientados à câmera)
- [ ] Dano (flash vermelho) e fogo (`on_fire_color`)
- [ ] Nome (`name tag`) e variantes por bioma
- [ ] Partículas ligadas ao mob (fogo, poeira, nectar da abelha)
- [ ] Sons dos eventos `oreville_ans:*`

## Jogador

- [x] Skin do jogo (wide e slim) com o modelo do pack
- [x] F5: corpo e cabeça giram com a câmera (v24)
- [~] Andar e correr: braços e pernas (`ground_speed`; revalidar em v25)
- [x] Nadar sem correr (animação, restaurada na v28)
- [~] Nadar correndo (Ctrl): ficava de cabeça para baixo; v26 troca NaN por 0
- [~] Preview no inventário/pausa girando sem parar; v26 (estado próprio, `is_in_ui`=1)
- [~] Idle: perna balançando e braços abertos (investigar com `HORIZON_BEDROCK_DUMP`)
- [~] Óculos do arreio do ghast e brilho de olhos (aranha, ghast) borrados/coloridos: causa não achada
- [~] Villager (roupas, profissão) e olho do mercador: linhas finas nas bordas; causa não achada
- [ ] Agachar, escalar, planar, dormir, montar
- [ ] Atacar (`attack_time`) e usar item (comer, arco, escudo)
- [~] Capa: laje projetada quando o elytra está equipado
- [ ] Emotes e animações de ocioso

## Itens na mão (1642 attachables)

- [x] Textura nova do item (`textures/items`)
- [~] 3ª pessoa: espada e lança aparecem na mão; posição bugada depois da v24
- [ ] **1ª pessoa**: item e mão com o modelo do pack (hoje só o sprite novo)
- [ ] Pose do braço segurando (`oreville_mainhand_custom_pos`, `hold_up`)
- [ ] Mão esquerda (`leftitem`) e off hand
- [ ] Pickaxe, axe, shovel, hoe, bow, crossbow, trident, shield, fishing rod, bucket, shears
- [ ] Comida e bebida (animação de usar)
- [ ] Poção na mão (frasco mal posicionado, textura original ainda aparece)
- [ ] Item em zumbi, esqueleto e drowned (espada, tridente)

## Armadura e vestíveis

- [~] Armadura vestida no jogador: sprites bugados ao equipar (v25 desloca camadas; revalidar)
- [ ] Capacete, peitoral, calça, bota: couro, ferro, ouro, diamante, netherite, cobre, cota de malha
- [ ] Brilho de encantamento sem pontilhado
- [ ] Trims
- [ ] Armadura em zumbi, esqueleto, piglin, armor stand
- [~] Elytra: aparece só no peito; cor e forma a conferir (pink na v24)
- [ ] Cabeças (skulls), abóbora esculpida, turtle helmet

## Blocos e itens retexturizados

- [x] Sprites de itens e blocos trocados (vanilla → `textures/blocks`, `items`)
- [ ] Texturas animadas (flipbook): água, lava, fogo, portal
- [ ] Céu, nuvens, sol e lua
- [ ] Folhagem e grama por bioma
- [ ] Texto de `splashes.json` no título

## Partículas (495 efeitos)

- [ ] Ataque (critical, sweep), dano, cura
- [ ] Andar (poeira, folhas)
- [ ] Ambiente de bioma
- [ ] Efeitos de poção e de encantamento

## Materiais

- [x] Alpha test e emissivo (`_glow`): olhos do allay, aranha, enderman
- [ ] Blend e aditivo: creeper carregado, vidro, água
- [ ] Stencil (contorno)
- [ ] Glint em item e armadura
- [ ] `DisableDepthWrite`/`Test` e multitexture (desenhados em v29; conferir no jogo: xp_orb do A&S usa depth write off)

## Como registrar um bug

1. `HORIZON_BEDROCK_DEBUG=1`; anotar build (`bedrock: engine build vNN`).
2. Print + trecho do log.
3. Linha deste checklist com `[~]` e nota curta.
4. Ao corrigir: build, o que mudou, resultado.
