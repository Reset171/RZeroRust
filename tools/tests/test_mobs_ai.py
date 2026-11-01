"""Tests 6, 7, 8: Remote chunk mobs cleanup, villager AI determinism, monster spawner."""

from . import setup_clean_env


def test_6_remote_chunks_mob_cleanup(h):
    setup_clean_env(h)
    h.tp(1111.5, 120, 1111.5)
    h.wait(8)
    h.command("kill @e[type=zombie,distance=..64]")
    h.command("kill @e[type=cow,distance=..64]")
    h.command("kill @e[type=oak_boat,distance=..64]")
    h.command("setblock 1111 119 1111 minecraft:air")
    h.wait(4)

    h.tp(0.5, 65, 0.5)
    h.wait(8)

    # 1. Точка сохранения
    h.rzero("set")
    h.wait(4)

    # 2. Телепорт в 1111 120 1111 и спавн мобов/лодок
    h.tp(1111.5, 120, 1111.5)
    h.wait(15)
    h.command("setblock 1111 119 1111 minecraft:stone")
    h.command("summon zombie 1111.5 120 1111.5")
    h.command("summon cow 1111.5 120 1111.5")
    h.command("summon oak_boat 1111.5 120 1111.5")
    h.wait(6)

    # 4. Телепорт в 2222
    h.tp(2222.5, 120, 2222.5)
    h.wait(15)

    # 5. Откат
    h.rzero("load")
    h.wait(10)

    # 6. Возвращение в 1111
    h.tp(1111.5, 120, 1111.5)
    h.wait(15)
    assert not h.has_entity("@e[type=zombie,distance=..32]"), "Zombies remained in remote chunk after rollback"
    assert not h.has_entity("@e[type=cow,distance=..32]"), "Cows remained in remote chunk after rollback"
    assert not h.has_entity("@e[type=oak_boat,distance=..32]"), "Boats remained in remote chunk after rollback"


def test_7_villager_ai_determinism(h):
    setup_clean_env(h)
    h.command("weather clear")
    h.command("kill @e[type=villager,distance=..64]")
    h.tp(700.5, 65, 700.5)
    h.wait(8)
    h.command("fill 695 64 695 705 64 705 minecraft:stone")
    h.command("fill 695 65 695 705 66 705 minecraft:air")
    h.command("summon villager 698.5 65 700.5 {NoAI:0b}")
    h.command("summon villager 702.5 65 700.5 {NoAI:0b}")
    h.wait(6)

    # Замораживаем мир ДО чекпоинта, чтобы тики шли строго по шагам
    h.command("tick freeze")
    h.wait(4)

    # 1. Чекпоинт в замороженном состоянии
    h.rzero("set")
    h.wait(4)

    # 2. Замер 1: откат в замороженном мире + строго 10 тиков шага
    h.rzero("load")
    h.wait(4)
    h.command("tick step 10")
    h.wait(15)
    ents_1 = {e["id"]: (round(e["pos"][0], 3), round(e["pos"][2], 3)) for e in h.entities(radius=10, entity_type="villager").get("entities", [])}

    # 3. Замер 2: повторный откат в замороженном мире + строго 10 тиков шага
    h.rzero("load")
    h.wait(4)
    h.command("tick step 10")
    h.wait(15)
    ents_2 = {e["id"]: (round(e["pos"][0], 3), round(e["pos"][2], 3)) for e in h.entities(radius=10, entity_type="villager").get("entities", [])}

    h.command("tick unfreeze")
    h.wait(4)

    assert ents_1 and ents_2, "Villagers missing in test area"
    for vid, p1 in ents_1.items():
        assert vid in ents_2, f"Villager {vid} disappeared in run 2"
        p2 = ents_2[vid]
        dx = abs(p1[0] - p2[0])
        dz = abs(p1[1] - p2[1])
        assert dx < 0.15 and dz < 0.15, f"Villager {vid} AI diverged: {p1} vs {p2}"

    h.command("kill @e[type=villager,distance=..16]")
    h.wait(4)


def test_8_spawner_determinism(h):
    setup_clean_env(h)
    h.command("kill @e[type=pig]")
    h.tp(800.5, 65.0, 804.5)
    h.wait(8)
    h.command("kill @e[type=pig]")
    h.command("fill 795 64 795 805 64 805 minecraft:grass_block")
    h.command("fill 795 65 795 805 68 805 minecraft:air")
    h.command("setblock 800 65 800 minecraft:spawner{Delay:15s,MinSpawnDelay:40s,MaxSpawnDelay:40s,SpawnCount:1s,SpawnData:{entity:{id:\"minecraft:pig\"}}}")
    h.wait(6)

    # Чекпоинт
    h.rzero("set")
    h.wait(4)

    # Ждем спавна (15 тиков + запас)
    h.wait(22)
    assert h.has_entity("@e[type=pig,x=800,y=65,z=800,distance=..5]"), "Pig did not spawn from spawner"

    # Откат
    h.rzero("load")
    assert not h.has_entity("@e[type=pig,x=800,y=65,z=800,distance=..5]"), "Pig remained immediately after rollback"

    # Повторное ожидание спавна (40 тиков)
    h.wait(45)
    assert h.has_entity("@e[type=pig,x=800,y=65,z=800,distance=..5]"), "Pig did not spawn after rollback"

    # Очистка
    h.command("setblock 800 65 800 minecraft:air")
    h.command("kill @e[type=pig,distance=..16]")
    h.wait(4)
