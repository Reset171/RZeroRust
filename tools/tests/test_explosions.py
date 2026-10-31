"""Tests 2 & 3: Massive TNT explosions and future ignition chain isolation."""

from . import setup_clean_env


def test_2_tnt_explosion_rollback(h):
    setup_clean_env(h)
    h.tp(200.5, 70, 200.5)
    h.wait(10)
    h.command("setblock 200 69 200 minecraft:stone")
    h.command("setblock 200 70 200 minecraft:air")
    h.wait(4)

    # 1. Чекпоинт до TNT
    h.rzero("set")
    h.wait(4)

    # 2. Создаем и поджигаем TNT
    h.command("fill 198 70 198 202 72 202 minecraft:tnt")
    h.wait(4)
    h.command("summon tnt 200 70 200")
    h.wait(6)

    # 4. Телепорт вдаль
    h.tp(99999.5, 120, 99999.5)
    h.wait(12)

    # 5. Откат
    h.rzero("load")
    h.wait(10)

    # Проверка базы: кратера нет, блок 200 69 200 на месте
    h.tp(200.5, 70, 200.5)
    h.wait(8)
    chk = h.block(200, 69, 200)
    assert "stone" in chk.get("block", ""), f"Crater detected, stone missing: {chk}"
    tnt_chk = h.block(200, 70, 200)
    assert "tnt" not in tnt_chk.get("block", ""), f"TNT blocks remained after rollback: {tnt_chk}"


def test_3_tnt_future_chain_isolation(h):
    setup_clean_env(h)
    h.tp(300.5, 70, 300.5)
    h.wait(10)
    h.command("setblock 300 69 300 minecraft:bedrock")
    h.command("fill 299 70 299 301 70 301 minecraft:tnt")
    h.wait(4)

    # 2. Сделать точку сохранения с готовым TNT
    h.rzero("set")
    h.wait(4)

    # 3. Поджечь
    h.command("summon tnt 300 71 300")
    h.wait(6)

    # 4. Телепорт в 99999
    h.tp(99999.5, 120, 99999.5)
    h.wait(12)

    # 5. Откат
    h.rzero("load")
    h.wait(10)

    # Проверка: блоки TNT стоят на месте нетронутыми
    h.tp(300.5, 70, 300.5)
    h.wait(8)
    chk = h.block(300, 70, 300)
    assert "tnt" in chk.get("block", ""), f"TNT was destroyed by future explosion: {chk}"
    h.command("fill 299 70 299 301 70 301 minecraft:air")
    h.wait(4)
