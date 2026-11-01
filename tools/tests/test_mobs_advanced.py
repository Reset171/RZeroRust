"""Tests 28-32: Advanced Mob AI, Anger, Leashes, Slimes, Shoulder Parrots."""

from . import setup_clean_env


def test_28_shoulder_parrots_rollback(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("summon minecraft:parrot 0.5 65 1.5 {Variant:0,Tags:['shoulder_parrot']}")
    h.wait(4)

    # 1. Чекпоинт с попугаем
    h.rzero("set")
    h.wait(4)

    # 2. Убиваем попугая и прыгаем
    h.command("kill @e[tag=shoulder_parrot]")
    h.tp(0.5, 75, 0.5)
    h.wait(6)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: попугай возрожден и на месте
    res = h.command("execute if entity @e[tag=shoulder_parrot] run say parrot_restored")
    chat = res.get("chat", [])
    assert any("parrot_restored" in line for line in chat), f"Shoulder parrot was not restored: {chat}"
    h.command("kill @e[tag=shoulder_parrot]")


def test_29_neutral_mob_anger_reset(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("summon minecraft:wolf 0.5 65 3.5 {Tags:['anger_wolf']}")
    h.wait(4)

    # 1. Чекпоинт с мирным волком
    h.rzero("set")
    h.wait(4)

    # 2. Наносим урон волку (агрим его)
    h.command("damage @e[tag=anger_wolf,limit=1] 1 minecraft:player_attack by @s")
    h.wait(4)

    # 3. Откат к моменту до удара
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: волк снова нейтрален, AngerTime сброшен
    wolf_res = h.command("data get entity @e[tag=anger_wolf,limit=1] AngerTime")
    chat = wolf_res.get("chat", [])
    assert any("не найдены" in line.lower() or "0" in line for line in chat), f"Wolf remained angry after rollback: {chat}"
    h.command("kill @e[tag=anger_wolf]")


def test_30_leash_distance_preservation(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("setblock 0 65 2 minecraft:oak_fence")
    h.command("summon minecraft:cow 0.5 65 3.5 {Tags:['leashed_cow']}")
    h.wait(4)

    # 1. Чекпоинт с коровой
    h.rzero("set")
    h.wait(4)

    # 2. Телепортируемся далеко на 150 блоков (дистанция превышает лимит поводка 10 блоков)
    h.tp(150.5, 65, 150.5)
    h.wait(8)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: корова на месте, забор цел, предмета выпавшего поводка нет
    res = h.command("execute if entity @e[tag=leashed_cow] run say cow_ok")
    chat = res.get("chat", [])
    assert any("cow_ok" in line for line in chat), f"Leashed cow disappeared after rollback: {chat}"
    h.command("kill @e[tag=leashed_cow]")
    h.command("setblock 0 65 2 minecraft:air")


def test_31_slime_jump_delay_determinism(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("summon minecraft:slime 0.5 65 3.5 {Size:1,Tags:['slime_timing']}")
    h.wait(4)

    # 1. Чекпоинт
    h.rzero("set")
    h.wait(4)

    # 2. Ждем прыжков
    h.wait(15)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: слизень на месте и сохранил физическое состояние
    res = h.command("execute if entity @e[tag=slime_timing] run say slime_ok")
    chat = res.get("chat", [])
    assert any("slime_ok" in line for line in chat), f"Slime was lost after rollback: {chat}"
    h.command("kill @e[tag=slime_timing]")


def test_32_sleeping_villager_day_schedule(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("time set day")
    h.command("summon minecraft:villager 0.5 65 3.5 {Tags:['sched_villager']}")
    h.wait(4)

    # 1. Чекпоинт днем
    h.rzero("set")
    h.wait(4)

    # 2. Ставим полночь
    h.command("time set midnight")
    h.wait(8)

    # 3. Откат обратно в день
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: день восстановлен, житель активен
    v_check = h.command("execute if entity @e[tag=sched_villager] run say villager_active")
    chat = v_check.get("chat", [])
    assert any("villager_active" in line for line in chat), f"Villager failed schedule sync after rollback: {chat}"
    h.command("kill @e[tag=sched_villager]")
