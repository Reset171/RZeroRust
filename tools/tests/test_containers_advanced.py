"""Tests 33-36: Advanced Containers (Furnace, Hopper), Item Frame & ArmorStand, Cursor Inventory."""

from . import setup_clean_env


def test_33_furnace_cook_and_burn_time(h):
    setup_clean_env(h)
    h.tp(1500.5, 65, 1500.5)
    h.command(
        "setblock 1500 65 1500 minecraft:furnace[lit=true]{Items:[{Slot:0b,id:'minecraft:raw_iron',count:10},{Slot:1b,id:'minecraft:coal',count:4},{Slot:2b,id:'minecraft:iron_ingot',count:2}],lit_time_remaining:120s,cooking_time_spent:50s,cooking_total_time:200s}"
    )
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command(
        "setblock 1500 65 1500 minecraft:furnace[lit=false]{Items:[{Slot:2b,id:'minecraft:iron_ingot',count:12}],lit_time_remaining:0s,cooking_time_spent:0s}"
    )
    h.wait(3)
    items_mut = h.data_get_block(1500, 65, 1500, "Items")
    assert "raw_iron" not in items_mut, f"Furnace raw_iron was not consumed in future state: {items_mut}"

    h.rzero("load")
    h.wait(4)

    items_restored = h.data_get_block(1500, 65, 1500, "Items")
    assert "raw_iron" in items_restored and "coal" in items_restored, f"Furnace slots not rolled back: {items_restored}"
    b_furnace = h.block(1500, 65, 1500)
    assert "lit=true" in b_furnace.get("state", ""), f"Furnace lit state not rolled back: {b_furnace}"
    h.command("setblock 1500 65 1500 minecraft:air")


def test_34_hopper_transfer_cooldown(h):
    setup_clean_env(h)
    h.tp(1510.5, 65, 1510.5)
    h.command(
        "setblock 1510 65 1510 minecraft:hopper{TransferCooldown:8,Items:[{Slot:0b,id:'minecraft:iron_ingot',count:5}]}"
    )
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("setblock 1510 65 1510 minecraft:hopper{Items:[]}")
    h.wait(3)
    empty_check = h.data_get_block(1510, 65, 1510, "Items")
    assert "iron_ingot" not in empty_check, f"Hopper was not emptied before rollback: {empty_check}"

    h.rzero("load")
    h.wait(4)

    restored = h.data_get_block(1510, 65, 1510, "Items")
    assert "iron_ingot" in restored and "5" in restored, f"Hopper transfer items not rolled back: {restored}"
    h.command("setblock 1510 65 1510 minecraft:air")


def test_35_item_frame_duplication_safety(h):
    setup_clean_env(h)
    h.tp(1520.5, 65, 1520.5)
    h.command("kill @e[tag=frame_hero]")
    h.command("kill @e[tag=stand_hero]")
    h.command("setblock 1520 65 1522 minecraft:stone")
    h.command(
        "summon minecraft:item_frame 1520 65 1521 {Facing:2b,Tags:['frame_hero'],Item:{id:'minecraft:diamond_sword',count:1}}"
    )
    h.command(
        "summon minecraft:armor_stand 1521.5 65 1520.5 {Tags:['stand_hero'],ArmorItems:[{},{},{},{id:'minecraft:diamond_helmet',count:1}]}"
    )
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("kill @e[tag=frame_hero]")
    h.command("kill @e[tag=stand_hero]")
    h.command("summon minecraft:item 1520.5 65 1521.5 {Item:{id:'minecraft:diamond_sword',count:1},Tags:['duped_drop']}")
    h.wait(3)
    assert not h.has_entity("@e[tag=frame_hero]"), "ItemFrame was not killed"

    h.rzero("load")
    h.wait(4)

    assert not h.has_entity("@e[tag=duped_drop]"), "Dropped item from broken frame leaked into past (duplication!)"
    frame_item = h.data_get_entity("@e[tag=frame_hero,limit=1]", "Item")
    assert "diamond_sword" in frame_item, f"ItemFrame sword was not restored: {frame_item}"
    stand_armor = h.data_get_entity("@e[tag=stand_hero,limit=1]", "ArmorItems")
    assert "diamond_helmet" in stand_armor, f"ArmorStand helmet was not restored: {stand_armor}"
    h.command("kill @e[tag=frame_hero]")
    h.command("kill @e[tag=stand_hero]")
    h.command("setblock 1520 65 1522 minecraft:air")


def test_36_creative_cursor_item_retention(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("give @s minecraft:gold_ingot 7")
    h.wait(3)

    h.open_inventory()
    h.wait(2)
    scr = h.screen()
    gold_slot = next((s["slot"] for s in scr.get("slots", []) if s.get("item") == "minecraft:gold_ingot"), 0)
    h.click_slot(gold_slot)
    h.wait(2)
    carried_before = h.inventory().get("carried")
    assert carried_before and carried_before.get("item") == "minecraft:gold_ingot", (
        f"Gold ingot was not picked up onto cursor: {carried_before}"
    )

    h.click_slot(gold_slot)
    h.close_screen()
    h.wait(2)

    h.rzero("set")
    h.wait(2)

    h.command("clear @s")
    h.wait(3)
    assert h.inventory().get("counts", {}).get("minecraft:gold_ingot", 0) == 0, "Gold ingot was not cleared"

    h.rzero("load")
    h.wait(4)

    assert h.inventory().get("counts", {}).get("minecraft:gold_ingot") == 7, (
        f"Player gold ingots not retained after rollback: {h.inventory()}"
    )
    h.command("clear @s")