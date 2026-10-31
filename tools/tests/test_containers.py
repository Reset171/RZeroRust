"""Tests 14 & 15: Chest container inventory and piston redstone machine rollback."""

from . import setup_clean_env


def test_14_chest_container_rollback(h):
    setup_clean_env(h)
    h.tp(600.5, 65, 600.5)
    h.wait(8)
    h.command("setblock 600 65 600 minecraft:chest")
    h.wait(4)

    # 1. Чекпоинт с пустым сундуком
    h.rzero("set")
    h.wait(4)

    # 2. Наполняем сундук алмазами
    h.command("item replace block 600 65 600 container.0 with minecraft:diamond 64")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(8)

    # Проверка: в сундуке нет алмазов (откатился к пустому)
    res = h.command("data get block 600 65 600 Items")
    chat = res.get("chat", [])
    has_diamonds = any("diamond" in line.lower() for line in chat)
    assert not has_diamonds, f"Chest items were not rolled back: {chat}"
    h.command("setblock 600 65 600 minecraft:air")


def test_15_piston_redstone_machine(h):
    setup_clean_env(h)
    h.tp(650.5, 65, 650.5)
    h.wait(8)
    h.command("setblock 650 65 650 minecraft:piston[extended=false]")
    h.command("setblock 650 65 651 minecraft:redstone_wire[power=0]")
    h.wait(4)

    # Чекпоинт в выключенном состоянии
    h.rzero("set")
    h.wait(4)

    # Активируем редстоун блоком
    h.command("setblock 650 65 652 minecraft:redstone_block")
    h.wait(6)

    # Откат
    h.rzero("load")
    h.wait(8)

    b_piston = h.block(650, 65, 650)
    b_source = h.block(650, 65, 652)

    assert "extended=false" in b_piston.get("state", ""), f"Piston state not rolled back: {b_piston}"
    assert "redstone_block" not in b_source.get("block", ""), f"Redstone block remained: {b_source}"

    # Очистка
    h.command("fill 649 65 649 651 65 653 minecraft:air")
