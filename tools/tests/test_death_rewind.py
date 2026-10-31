"""Tests 9 & 10: Fatal fall Return by Death and multiple fall cycles (anti-invulnerability)."""

from . import setup_clean_env


def test_9_return_by_death_fatal_fall(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(8)
    base_pos = h.state()["player"]["position"]

    # Чекпоинт
    h.rzero("set")
    h.wait(4)

    # Смертельное падение со 120 блоков
    h.tp(base_pos[0], 120.0, base_pos[2])

    rewound = False
    for _ in range(30):
        h.wait(3)
        st = h.state()
        p = st.get("player", {})
        if p.get("on_ground") and abs(p.get("position", [0, 0, 0])[1] - base_pos[1]) < 2.0:
            rewound = True
            break
    h.wait(6)

    st_final = h.state()
    assert rewound, "Player did not rewind on death"
    assert st_final["player"]["health"] >= 19.0, f"Health not restored to 20: {st_final['player']['health']}"
    assert st_final.get("screen") == "playing", f"Death screen was not suppressed: {st_final.get('screen')}"


def test_10_multiple_fall_cycles(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(8)
    base_pos = h.state()["player"]["position"]

    # 1. Первый прыжок
    h.tp(base_pos[0], 120.0, base_pos[2])
    for _ in range(30):
        h.wait(3)
        p = h.state().get("player", {})
        if p.get("on_ground") and abs(p.get("position", [0, 0, 0])[1] - base_pos[1]) < 2.0:
            break
    h.wait(6)
    assert h.state()["player"]["health"] >= 19.0, "First fall recovery failed"

    # 2. Второй прыжок (проверка, что урон рассчитывается и неуязвимость не залипла)
    h.tp(base_pos[0], 120.0, base_pos[2])
    rewound_2 = False
    for _ in range(30):
        h.wait(3)
        p = h.state().get("player", {})
        if p.get("on_ground") and abs(p.get("position", [0, 0, 0])[1] - base_pos[1]) < 2.0:
            rewound_2 = True
            break
    h.wait(6)
    assert rewound_2, "Second fall did not trigger rewind"
    assert h.state()["player"]["health"] >= 19.0, "Second fall health not recovered"


def test_18_drowning_and_damage_after_rewind(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(8)
    base_pos = h.state()["player"]["position"]

    # 1. Чекпоинт в выживании
    h.rzero("set")
    h.wait(4)

    # 2. Фатальное падение со 120 блоков
    h.tp(base_pos[0], 120.0, base_pos[2])
    for _ in range(30):
        h.wait(3)
        if h.state()["player"]["on_ground"]:
            break
    h.wait(6)

    # 3. Проверка: после отката игрок получает урон (неуязвимость снята)
    dmg = h.command("damage @s 4 minecraft:generic")
    chat = dmg.get("chat", [])
    assert not any("невосприимчива" in line.lower() or "invulnerable" in line.lower() for line in chat), f"Player remained invulnerable after rewind: {chat}"

    # 4. Проверка погружения в воду и потери воздуха (игрок должен тонуть)
    h.command("fill 990 60 990 994 65 994 minecraft:water")
    h.tp(992.5, 61.0, 992.5)
    h.wait(20)
    air = h.state()["player"].get("air", 300)
    assert air < 300, f"Player does not lose air under water after rewind: air={air}"
    h.command("fill 990 60 990 994 65 994 minecraft:air")
    h.tp(0.5, 65, 0.5)
    h.wait(4)
