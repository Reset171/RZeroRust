"""Test 17: End Dragon bossbar and cross-dimension rollback."""

from . import setup_clean_env


def test_17_end_dragon_bossbar_rollback(h):
    setup_clean_env(h)
    h.tp(0.5, 65.0, 0.5)
    h.wait(8)

    # 1. Чекпоинт в Оверворлде (боссбара нет)
    h.rzero("set")
    h.wait(4)

    # 2. ТП в Энд
    h.command("execute in minecraft:the_end run tp @s 0.5 70.0 0.5")
    h.wait(14)
    st_end = h.state()
    assert st_end.get("dimension") == "minecraft:the_end", f"Not in End: {st_end.get('dimension')}"

    # 3. Откат к чекпоинту в Оверворлде
    h.rzero("load")
    h.wait(14)

    st_back = h.state()
    assert st_back.get("dimension") == "minecraft:overworld", f"Dimension not rolled back: {st_back.get('dimension')}"
    assert abs(st_back["player"]["position"][0] - 0.5) < 2.0, f"Position not rolled back: {st_back['player']['position']}"
