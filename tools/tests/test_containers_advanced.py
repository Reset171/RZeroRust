"""Tests 33-36: Advanced Containers (Barrels, Hoppers), Armor Stands, Inventory."""

from . import setup_clean_env


def test_33_furnace_cook_and_burn_time(h):
    setup_clean_env(h)
    h.tp(1500.5, 65, 1500.5)
    h.command("setblock 1500 65 1500 minecraft:barrel{Items:[{Slot:0b,id:'minecraft:diamond',count:10}]}")
    h.wait(4)

    # 1. Чекпоинт с 10 алмазами в бочке
    h.rzero("set")
    h.wait(4)

    # 2. Заменяем алмазы на 64 изумруда
    h.command("setblock 1500 65 1500 minecraft:barrel{Items:[{Slot:0b,id:'minecraft:emerald',count:64}]}")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: в бочке снова 10 алмазов, а не изумруды
    data_res = h.command("data get block 1500 65 1500 Items")
    chat = data_res.get("chat", [])
    assert any("diamond" in line and "10" in line for line in chat), f"Barrel items not rolled back: {chat}"
    assert not any("emerald" in line for line in chat), f"Future emeralds leaked into barrel: {chat}"
    h.command("setblock 1500 65 1500 minecraft:air")


def test_34_hopper_transfer_cooldown(h):
    setup_clean_env(h)
    h.tp(1510.5, 65, 1510.5)
    h.command("setblock 1510 65 1510 minecraft:hopper{TransferCooldown:8,Items:[{Slot:0b,id:'minecraft:iron_ingot',count:5}]}")
    h.wait(4)

    # 1. Чекпоинт с 5 слитками в воронке
    h.rzero("set")
    h.wait(4)

    # 2. Опустошаем воронку
    h.command("setblock 1510 65 1510 minecraft:hopper{Items:[]}")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: в воронке ровно 5 слитков железа
    data_res = h.command("data get block 1510 65 1510 Items")
    chat = data_res.get("chat", [])
    assert any("iron_ingot" in line and "5" in line for line in chat), f"Hopper transfer items not rolled back: {chat}"
    h.command("setblock 1510 65 1510 minecraft:air")


def test_35_item_frame_duplication_safety(h):
    setup_clean_env(h)
    h.tp(1520.5, 65, 1520.5)
    h.command("summon minecraft:armor_stand 1520.5 65 1520.5 {Tags:['stand_hero'],ArmorItems:[{},{},{},{id:'minecraft:diamond_helmet',count:1}]}")
    h.wait(4)

    # 1. Чекпоинт со стойкой брони в алмазном шлеме
    h.rzero("set")
    h.wait(4)

    # 2. Уничтожаем стойку брони
    h.command("kill @e[tag=stand_hero]")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: стойка брони и алмазный шлем на месте
    data_res = h.command("data get entity @e[tag=stand_hero,limit=1] ArmorItems")
    chat = data_res.get("chat", [])
    assert any("diamond_helmet" in line for line in chat), f"ArmorStand helmet was not restored: {chat}"
    h.command("kill @e[tag=stand_hero]")


def test_36_creative_cursor_item_retention(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(4)

    # 1. Даем игроку золотой слиток и делаем чекпоинт
    h.command("give @s minecraft:gold_ingot 7")
    h.wait(4)
    h.rzero("set")
    h.wait(4)

    # 2. Выбрасываем все предметы
    h.command("clear @s")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: 7 слитков золота снова в инвентаре игрока
    inv_res = h.command("data get entity @s Inventory")
    chat = inv_res.get("chat", [])
    assert any("gold_ingot" in line for line in chat), f"Player gold ingot not retained after rollback: {chat}"
    h.command("clear @s")
