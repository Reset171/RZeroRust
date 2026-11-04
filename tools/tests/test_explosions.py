"""Tests 2 & 3: Massive TNT explosions and future ignition chain isolation."""

from . import setup_clean_env


def test_2_tnt_explosion_rollback(h):
    setup_clean_env(h)
    h.tp(200.5, 70, 200.5)
    h.command("fill 197 68 197 203 69 203 minecraft:stone")
    h.command("fill 197 70 197 203 74 203 minecraft:air")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("fill 199 70 199 201 71 201 minecraft:tnt")
    h.command("summon tnt 200.5 70.0 200.5 {fuse:1s}")
    h.wait(8)

    crater = h.block(200, 69, 200)
    assert "stone" not in crater.get("block", ""), f"TNT did not destroy stone floor before rollback: {crater}"

    h.rzero("load")
    h.wait(6)

    chk = h.block(200, 69, 200)
    assert "stone" in chk.get("block", ""), f"Crater remained after rollback, stone missing: {chk}"
    tnt_chk = h.block(200, 70, 200)
    assert "tnt" not in tnt_chk.get("block", ""), f"TNT blocks remained after rollback: {tnt_chk}"
    assert not h.has_entity("@e[type=tnt,distance=..16]"), "Primed TNT remained after rollback"


def test_3_tnt_future_chain_isolation(h):
    setup_clean_env(h)
    h.tp(300.5, 70, 300.5)
    h.command("fill 298 69 298 302 69 302 minecraft:bedrock")
    h.command("fill 299 70 299 301 70 301 minecraft:tnt")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("summon tnt 300.5 71.0 300.5 {fuse:1s}")
    h.wait(8)

    destroyed = h.block(300, 70, 300)
    assert "tnt" not in destroyed.get("block", ""), f"TNT was not ignited in future timeline: {destroyed}"

    h.rzero("load")
    h.wait(6)

    chk = h.block(300, 70, 300)
    assert "tnt" in chk.get("block", ""), f"TNT was destroyed by future explosion: {chk}"
    assert not h.has_entity("@e[type=tnt,distance=..16]"), "Future primed TNT leaked into past"
    h.command("fill 299 70 299 301 70 301 minecraft:air")
    h.wait(2)