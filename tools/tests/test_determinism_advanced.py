"""Tests 37-40: Advanced Determinism, Height Limits (-64 & 318), Loot Tables, Raids, Maps."""

from . import setup_clean_env


def test_37_raid_determinism_and_coords(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(4)

    # 1. Чекпоинт без эффекта рейда
    h.rzero("set")
    h.wait(4)

    # 2. Накладываем эффект Дурного знака (Bad Omen)
    h.command("effect give @s bad_omen 60 1")
    h.wait(4)
    eff_res = h.command("data get entity @s active_effects")
    chat = eff_res.get("chat", [])
    assert any("bad_omen" in line for line in chat), f"Bad omen effect was not applied: {chat}"

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: эффект дурного знака сброшен
    eff_res_after = h.command("data get entity @s active_effects")
    chat_after = eff_res_after.get("chat", [])
    assert not any("bad_omen" in line for line in chat_after), f"Bad omen effect leaked into restored past: {chat_after}"


def test_38_chest_loot_table_determinism(h):
    setup_clean_env(h)
    h.tp(1600.5, 65, 1600.5)
    h.command("setblock 1600 65 1600 minecraft:chest{LootTable:'minecraft:chests/simple_dungeon'}")
    h.wait(4)

    # 1. Чекпоинт с сундуком со статической таблицей лута
    h.rzero("set")
    h.wait(4)

    # 2. Ломаем сундук и ставим туда грязь
    h.command("setblock 1600 65 1600 minecraft:dirt")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: сундук восстановлен и сохранил свою таблицу лута
    loot_res = h.command("data get block 1600 65 1600 LootTable")
    chat = loot_res.get("chat", [])
    assert any("simple_dungeon" in line for line in chat), f"Chest loot table was not preserved: {chat}"
    h.command("setblock 1600 65 1600 minecraft:air")


def test_39_explored_map_fog_of_war(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(4)

    # 1. Чекпоинт с чистым инвентарем без карт
    h.command("clear @s")
    h.wait(2)
    h.rzero("set")
    h.wait(4)

    # 2. Выдаем заполненную карту
    h.command("give @s minecraft:filled_map 1")
    h.wait(4)
    inv_res = h.command("data get entity @s Inventory")
    assert any("filled_map" in line for line in inv_res.get("chat", [])), "Map was not given"

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: карта исчезла из инвентаря
    inv_after = h.command("data get entity @s Inventory")
    assert not any("filled_map" in line for line in inv_after.get("chat", [])), "Explored map leaked into checkpoint"


def test_40_subchunk_palette_expansion_bounds(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(4)

    # 1. Чекпоинт
    h.rzero("set")
    h.wait(4)

    # 2. Ставим блоки на предельных границах мира: Y=318 (верх неба) и Y=-50 (глубинный сланец)
    h.command("setblock 0 318 0 minecraft:stone")
    h.command("setblock 0 -50 0 minecraft:gold_block")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: блок на Y=318 снова воздух, блок на Y=-50 вернулся в исходное состояние
    b_top = h.block(0, 318, 0)
    assert b_top.get("block") == "minecraft:air", f"Top sky block leaked at Y=318: {b_top}"
    b_bot = h.block(0, -50, 0)
    assert b_bot.get("block") != "minecraft:gold_block", f"Bottom deep block leaked at Y=-50: {b_bot}"
