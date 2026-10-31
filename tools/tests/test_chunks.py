"""Test 1: Unloaded chunks isolation and time persistence."""

import time
from . import setup_clean_env


def test_1_unloaded_chunks(h):
    setup_clean_env(h)
    h.command("setblock 0 64 0 minecraft:stone")
    h.command("setblock 0 65 0 minecraft:air")
    h.command("setblock 0 66 0 minecraft:air")
    h.tp(0.5, 65, 0.5)
    h.wait(8)

    offset = int(time.time()) % 1000
    ax = 90000 + offset * 16
    bx = 80000 + offset * 16

    # 1. Точка сохранения 1
    h.rzero("set")
    h.wait(4)

    # 2. Не загруженный чанк A
    h.tp(ax + 0.5, 120, ax + 0.5)
    h.wait(20)
    h.command(f"setblock {ax} 119 {ax} minecraft:diamond_block")
    h.wait(6)
    b1 = h.block(ax, 119, ax)
    assert "diamond_block" in b1.get("block", ""), f"Diamond block not set: {b1}"

    # 4. Прийти обратно
    h.tp(0.5, 65, 0.5)
    h.wait(10)

    # 5. Сделать точку сохранения 2
    h.rzero("set")
    h.wait(4)

    # 6. Не загруженный чанк B
    h.tp(bx + 0.5, 120, bx + 0.5)
    h.wait(20)
    h.command(f"setblock {bx} 119 {bx} minecraft:emerald_block")
    h.wait(6)
    b2 = h.block(bx, 119, bx)
    assert "emerald_block" in b2.get("block", ""), f"Emerald block not set: {b2}"

    # 8. Уйти на базу
    h.tp(0.5, 65, 0.5)
    h.wait(10)

    # 9. Сделать откат к точке 2
    h.rzero("load")
    h.wait(10)

    # Проверка: в чанке A алмазный блок остался
    h.tp(ax + 0.5, 120, ax + 0.5)
    h.wait(15)
    chk_diamond = h.block(ax, 119, ax)
    assert "diamond_block" in chk_diamond.get("block", ""), f"Chunk A lost changes: {chk_diamond}"

    # Проверка: в чанке B изумрудный блок откатился
    h.tp(bx + 0.5, 120, bx + 0.5)
    h.wait(15)
    chk_emerald = h.block(bx, 119, bx)
    assert "emerald_block" not in chk_emerald.get("block", ""), f"Chunk B was not rolled back: {chk_emerald}"

    # Очистка
    h.command(f"setblock {ax} 119 {ax} minecraft:air")
    h.command(f"setblock {bx} 119 {bx} minecraft:air")
    h.tp(0.5, 65, 0.5)
    h.wait(6)
