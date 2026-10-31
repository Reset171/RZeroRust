"""Test 12: Selective rollback policies (keep inventory while rewinding world)."""

from . import setup_clean_env


def test_12_policy_selective_inventory(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(8)

    # Отключаем откат инвентаря
    h.rzero("policy set player_inventory false")
    h.rzero("set")
    h.wait(4)

    # Выдаем алмазы
    h.command("give @s minecraft:diamond 16")
    h.wait(4)
    inv_before = h.inventory().get("counts", {})
    assert inv_before.get("minecraft:diamond") == 16, f"Diamonds not given: {inv_before}"

    # Перемещаемся и откатываемся
    h.tp(30.5, 65, 30.5)
    h.wait(6)
    h.rzero("trigger rewind")
    h.wait(8)

    # Координаты откатились на базу
    pos_after = h.state()["player"]["position"]
    assert abs(pos_after[0] - 0.5) < 1.5, f"Position did not rewind: {pos_after}"

    # Инвентарь СОХРАНИЛСЯ (16 алмазов)
    inv_after = h.inventory().get("counts", {})
    assert inv_after.get("minecraft:diamond") == 16, f"Selective inventory policy failed: {inv_after}"

    # Сброс политики
    h.rzero("policy reset")
    h.command("clear @s")
    h.wait(4)
