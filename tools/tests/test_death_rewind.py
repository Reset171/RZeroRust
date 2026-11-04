"""Tests 9, 10, 18: Fatal fall Return by Death, multiple fall cycles, and post-rewind damage."""

from . import setup_clean_env


def test_9_return_by_death_fatal_fall(h):
    setup_clean_env(h)
    base_pos = h.state()["player"]["position"]

    h.rzero("set")
    h.wait(2)

    h.command(f"tp @s {base_pos[0]} 125.0 {base_pos[2]}")

    rewound = False
    for _ in range(35):
        h.wait(3)
        st = h.state()
        p = st.get("player", {})
        if p.get("on_ground") and abs(p.get("position", [0, 0, 0])[1] - base_pos[1]) < 2.0:
            rewound = True
            break
    h.wait(4)

    st_final = h.state()
    assert rewound, "Player did not rewind on fatal fall death"
    assert st_final["player"]["health"] >= 19.0, f"Health not restored to 20: {st_final['player']['health']}"
    assert st_final.get("screen") == "playing", f"Death screen was not suppressed: {st_final.get('screen')}"


def test_10_multiple_fall_cycles(h):
    setup_clean_env(h)
    base_pos = h.state()["player"]["position"]

    h.rzero("set")
    h.wait(2)

    for cycle in (1, 2):
        h.command(f"tp @s {base_pos[0]} 125.0 {base_pos[2]}")
        rewound = False
        for _ in range(35):
            h.wait(3)
            p = h.state().get("player", {})
            if p.get("on_ground") and abs(p.get("position", [0, 0, 0])[1] - base_pos[1]) < 2.0:
                rewound = True
                break
        h.wait(4)
        assert rewound, f"Fall cycle {cycle} did not trigger Return by Death"
        assert h.state()["player"]["health"] >= 19.0, f"Fall cycle {cycle} health not recovered"


def test_18_drowning_and_damage_after_rewind(h):
    setup_clean_env(h)
    base_pos = h.state()["player"]["position"]

    h.rzero("set")
    h.wait(2)

    h.command(f"tp @s {base_pos[0]} 125.0 {base_pos[2]}")
    for _ in range(35):
        h.wait(3)
        p = h.state().get("player", {})
        if p.get("on_ground") and abs(p.get("position", [0, 0, 0])[1] - base_pos[1]) < 2.0:
            break
    h.wait(4)

    dmg = h.command("damage @s 6 minecraft:generic")
    h.wait(4)
    chat = dmg.get("chat", [])
    assert not any("невосприимчива" in line.lower() or "invulnerable" in line.lower() for line in chat), (
        f"Player remained invulnerable after rewind: {chat}"
    )
    assert h.state()["player"]["health"] <= 18.0, f"Player health did not decrease after generic damage: {h.state()['player']}"

    h.tp(992.5, 61.0, 992.5)
    h.command("fill 990 59 990 994 59 994 minecraft:stone")
    h.command("fill 990 60 990 994 64 994 minecraft:water")
    h.wait(18)
    air = h.state()["player"].get("air", 300)
    assert air < 300, f"Player does not lose air under water after rewind: air={air}"
    h.command("fill 990 60 990 994 64 994 minecraft:air")
    h.tp(0.5, 65, 0.5)