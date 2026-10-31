"""Tests 4, 5, 16: Fishing rod bobbers, boat mounts, flying projectiles."""

from . import setup_clean_env


def test_4_fishing_bobber_preservation(h):
    setup_clean_env(h)
    h.tp(400.5, 65, 400.5)
    h.wait(8)
    h.command("setblock 400 64 402 minecraft:water")
    h.command("give @s minecraft:fishing_rod 1")
    h.raw_send("select 1")
    h.wait(4)

    # 1. Точка сохранения с удочкой
    h.rzero("set")
    h.wait(4)

    # 2. Очищаем инвентарь и телепортируемся вдаль
    h.command("clear @s")
    h.tp(99999.5, 120, 99999.5)
    h.wait(12)

    # 3. Откат
    h.rzero("load")
    h.wait(10)

    # Проверка: игрок вернулся на координаты озера и удочка восстановлена
    st = h.state()
    p = st.get("player", {})
    assert abs(p.get("position", [0, 0, 0])[0] - 400.5) < 2.0, f"Position not restored: {p}"
    assert h.inventory().get("counts", {}).get("minecraft:fishing_rod") == 1, "Fishing rod was not restored"


def test_5_boat_mount_rollback(h):
    setup_clean_env(h)
    h.tp(500.5, 65, 500.5)
    h.wait(8)
    h.command("kill @e[type=oak_boat,distance=..32]")
    h.command("summon oak_boat 500.5 65.0 500.5")
    h.wait(4)

    # 2. Чекпоинт
    h.rzero("set")
    h.wait(4)

    # 3. Ломаем лодки вокруг
    h.command("kill @e[type=oak_boat,distance=..10]")
    h.wait(4)
    assert not h.has_entity("@e[type=oak_boat,distance=..10]"), "Boat was not killed"
    h.tp(99999.5, 120, 99999.5)
    h.wait(10)

    # 5. Откат
    h.rzero("load")
    h.wait(12)

    assert h.has_entity("@e[type=oak_boat,distance=..16]"), "Boat was not restored after rollback"


def test_16_flying_arrow_projectile(h):
    setup_clean_env(h)
    h.tp(750.5, 65, 750.5)
    h.wait(8)
    h.command("kill @e[type=arrow,distance=..32]")

    # Чекпоинт без стрел
    h.rzero("set")
    h.wait(4)

    # Спавним летящую стрелу
    h.command("summon arrow 750.5 70.0 750.5 {Motion:[0.0,1.0,0.0]}")
    h.wait(4)
    assert h.has_entity("@e[type=arrow,distance=..16]"), "Arrow was not spawned"

    # Откат
    h.rzero("load")
    h.wait(6)

    # Стрелы из будущего нет
    assert not h.has_entity("@e[type=arrow,distance=..16]"), "Future arrow remained after rollback"
