"""Tests 37-40: Advanced Determinism, Height Limits (-64 & 318), Loot Tables, Raids, Maps."""

from . import setup_clean_env


def test_37_raid_determinism_and_coords(h):
    setup_clean_env(h)
    h.rzero("set")
    h.wait(2)

    h.command("effect give @s bad_omen 60 1")
    h.wait(3)
    eff_chat = h.data_get_entity("@s", "active_effects")
    assert "bad_omen" in eff_chat, f"Bad omen effect was not applied: {eff_chat}"

    h.rzero("load")
    h.wait(4)

    eff_after = h.data_get_entity("@s", "active_effects")
    assert "bad_omen" not in eff_after, f"Bad omen effect leaked into restored past: {eff_after}"


def test_38_chest_loot_table_determinism(h):
    setup_clean_env(h)
    h.tp(1600.5, 65, 1600.5)
    h.command("setblock 1600 65 1600 minecraft:chest{LootTable:'minecraft:chests/simple_dungeon',LootTableSeed:424242L}")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("setblock 1600 65 1600 minecraft:dirt")
    h.wait(3)
    assert "dirt" in h.block(1600, 65, 1600).get("block", ""), "Chest was not replaced with dirt"

    h.rzero("load")
    h.wait(4)

    loot_chat = h.data_get_block(1600, 65, 1600, "LootTable")
    assert "simple_dungeon" in loot_chat, f"Chest loot table was not preserved: {loot_chat}"
    h.command("setblock 1600 65 1600 minecraft:air")


def test_39_explored_map_fog_of_war(h):
    setup_clean_env(h)
    h.command("item replace entity @s hotbar.0 with minecraft:filled_map[minecraft:map_id=7]")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("item replace entity @s hotbar.0 with minecraft:filled_map[minecraft:map_id=99]")
    h.command("give @s minecraft:compass 1")
    h.wait(3)
    inv_mut = h.data_get_entity("@s", "Inventory")
    assert "99" in inv_mut, f"Map id was not mutated to 99: {inv_mut}"

    h.rzero("load")
    h.wait(4)

    inv_after = h.data_get_entity("@s", "Inventory")
    assert "7" in inv_after and "99" not in inv_after, f"Map component state was not restored: {inv_after}"
    assert "compass" not in inv_after, "Future compass item leaked into past"
    h.command("clear @s")


def test_40_subchunk_palette_expansion_bounds(h):
    setup_clean_env(h)
    h.rzero("set")
    h.wait(2)

    h.command("setblock 0 318 0 minecraft:stone")
    h.command("setblock 0 -50 0 minecraft:gold_block")
    h.wait(3)
    assert h.block(0, 318, 0).get("block") == "minecraft:stone", "Block at Y=318 was not placed"
    assert h.block(0, -50, 0).get("block") == "minecraft:gold_block", "Block at Y=-50 was not placed"

    h.rzero("load")
    h.wait(4)

    b_top = h.block(0, 318, 0)
    assert b_top.get("block") == "minecraft:air", f"Top sky block leaked at Y=318: {b_top}"
    b_bot = h.block(0, -50, 0)
    assert b_bot.get("block") != "minecraft:gold_block", f"Bottom deep block leaked at Y=-50: {b_bot}"