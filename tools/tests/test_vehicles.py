"""Tests 4, 5, 16: Fishing rod bobbers, boat mounts, flying projectiles."""

from . import setup_clean_env


def test_4_fishing_bobber_preservation(h):
    setup_clean_env(h)
    h.tp(400.5, 65, 400.5)
    h.command("fill 398 63 401 402 64 405 minecraft:water")
    h.command("fill 399 64 399 401 64 400 minecraft:stone")
    h.command("tp @s 400.5 65.0 400.0 0 25")
    h.command("give @s minecraft:fishing_rod 1")
    h.select_slot(1)
    h.wait(3)

    h.use_item()
    h.wait(6)

    h.rzero("set")
    h.wait(2)

    h.command("clear @s")
    h.tp(420.5, 65, 420.5)
    h.wait(4)
    assert h.inventory().get("counts", {}).get("minecraft:fishing_rod", 0) == 0, "Fishing rod was not cleared"

    h.rzero("load")
    h.wait(6)

    st = h.state()
    p = st.get("player", {})
    assert abs(p.get("position", [0, 0, 0])[0] - 400.5) < 1.5, f"Position not restored: {p}"
    assert h.inventory().get("counts", {}).get("minecraft:fishing_rod") == 1, "Fishing rod was not restored"
    h.command("clear @s")
    h.command("fill 398 63 401 402 64 405 minecraft:stone")


def test_5_boat_mount_rollback(h):
    setup_clean_env(h)
    h.tp(500.5, 65, 500.5)
    h.command("fill 498 64 498 504 64 504 minecraft:stone")
    h.command("kill @e[type=oak_boat,distance=..32]")
    h.command("summon oak_boat 500.5 65.0 502.5 {Tags:['rzero_boat']}")
    h.wait(4)

    assert h.has_entity("@e[type=oak_boat,tag=rzero_boat]"), "Boat was not summoned"

    h.rzero("set")
    h.wait(2)
    pos_before = h.data_get_entity("@e[tag=rzero_boat,limit=1]", "Pos")

    h.command("kill @e[type=oak_boat,distance=..16]")
    h.wait(4)
    assert not h.has_entity("@e[type=oak_boat,tag=rzero_boat]"), "Boat was not killed"

    h.rzero("load")
    h.wait(6)

    assert h.has_entity("@e[type=oak_boat,tag=rzero_boat]"), "Boat was not restored after rollback"
    pos_after = h.data_get_entity("@e[tag=rzero_boat,limit=1]", "Pos")
    assert pos_before == pos_after, f"Boat position changed after rollback: {pos_before} vs {pos_after}"
    h.command("kill @e[type=oak_boat,distance=..16]")


def test_16_flying_arrow_projectile(h):
    setup_clean_env(h)
    h.tp(750.5, 65, 750.5)
    h.command("kill @e[type=arrow,distance=..32]")
    h.wait(2)

    h.command("summon arrow 750.5 70.0 750.5 {Motion:[0.0,0.5,1.0],Tags:['saved_arrow']}")
    h.wait(2)
    assert h.has_entity("@e[tag=saved_arrow]"), "Saved arrow was not spawned"

    h.command("tick freeze")
    h.wait(2)

    h.rzero("set")
    h.wait(2)

    h.command("summon arrow 752.5 70.0 750.5 {Motion:[0.0,1.0,0.0],Tags:['future_arrow']}")
    h.command("tick step 5")
    h.wait(8)
    assert h.has_entity("@e[tag=future_arrow]"), "Future arrow was not spawned"
    pos_run1 = h.data_get_entity("@e[tag=saved_arrow,limit=1]", "Pos")

    h.rzero("load")
    h.wait(4)

    assert not h.has_entity("@e[tag=future_arrow]"), "Future arrow remained after rollback"
    assert h.has_entity("@e[tag=saved_arrow]"), "Saved arrow was lost after rollback"

    h.command("tick step 5")
    h.wait(8)
    pos_run2 = h.data_get_entity("@e[tag=saved_arrow,limit=1]", "Pos")
    assert pos_run1 == pos_run2, f"Arrow trajectory diverged after rollback: {pos_run1} vs {pos_run2}"

    h.command("tick unfreeze")
    h.command("kill @e[type=arrow,distance=..32]")