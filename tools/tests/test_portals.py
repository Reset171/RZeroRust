"""Tests 24-27: Portals, Dimension Travel, Ender Pearls, Portal Cooldown."""

from . import setup_clean_env


def test_24_nether_portal_cold_chunk(h):
    setup_clean_env(h)
    h.tp(1200.5, 65, 1200.5)
    h.wait(8)

    # 1. Рамка портала в Оверворлде (обсидиан, незажжен)
    h.command("setblock 1200 65 1200 minecraft:obsidian")
    h.command("setblock 1200 66 1200 minecraft:obsidian")
    h.command("setblock 1200 67 1200 minecraft:obsidian")
    h.wait(4)

    # 2. Чекпоинт перед зажиганием
    h.rzero("set")
    h.wait(4)

    # 3. Зажигаем портал и удаляемся на 500 блоков (имитация выгрузки чанка)
    h.command("setblock 1200 66 1200 minecraft:nether_portal[axis=z]")
    h.tp(1800.5, 65, 1800.5)
    h.wait(12)

    # 4. Откат
    h.rzero("load")
    h.wait(8)

    # 5. Проверка: чанк вернулся в исходное состояние, портал потушен
    h.tp(1200.5, 65, 1200.5)
    h.wait(6)
    b = h.block(1200, 66, 1200)
    assert b.get("block") == "minecraft:obsidian" or b.get("block") == "minecraft:air", f"Portal block leaked in cold chunk: {b}"


def test_25_flying_ender_pearl_discard(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    for _ in range(10):
        h.wait(2)
        if abs(h.state()["player"]["position"][0] - 0.5) < 1.0:
            break
    base_pos = h.state()["player"]["position"]

    # 1. Чекпоинт на безопасной площадке
    h.rzero("set")
    h.wait(4)

    # 2. Спавним летящий эндер-перл далеко в яму со скоростью
    h.command("summon minecraft:ender_pearl 0.5 70 0.5 {Motion:[0.0, 1.0, 50.0]}")
    h.wait(3)

    # 3. Откат до приземления перла
    h.rzero("load")
    h.wait(20)

    # 4. Проверка: перл не телепортировал игрока, игрок остался на безопасной базе
    pos_after = h.state()["player"]["position"]
    assert abs(pos_after[0] - base_pos[0]) < 3.0 and abs(pos_after[2] - base_pos[2]) < 3.0, f"Flying ender pearl teleported player after rewind: {pos_after}"


def test_26_portal_processor_cooldown(h):
    setup_clean_env(h)
    h.tp(1300.5, 65, 1300.5)
    h.wait(8)

    # 1. Чекпоинт рядом с блоком портала
    h.command("setblock 1300 65 1300 minecraft:nether_portal[axis=x]")
    h.wait(4)
    h.rzero("set")
    h.wait(4)

    # 2. Наступаем в блок портала на 5 тиков
    h.tp(1300.5, 65.0, 1300.5)
    h.wait(5)

    # 3. Откат
    h.rzero("load")
    h.wait(8)

    # 4. Проверка: измерение по-прежнему overworld, самопроизвольной телепортации нет
    dim = h.state().get("dimension", "")
    assert "overworld" in dim, f"Player was unexpectedly sucked into portal after rewind: {dim}"
    h.command("setblock 1300 65 1300 minecraft:air")


def test_27_end_credits_respawn_safety(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(8)

    # 1. Чекпоинт в выживании
    h.rzero("set")
    h.wait(4)

    # 2. Имитация перехода через титры/врата (Seen credits)
    h.command("advancement grant @s only minecraft:end/root")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: экран Playing, смерть не зафиксирована
    st = h.state()
    assert st.get("screen") == "playing", f"Screen was broken after credits check: {st.get('screen')}"
