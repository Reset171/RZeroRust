"""Test 11: Timeline branching with named slots."""

from . import setup_clean_env


def test_11_slots_branching(h):
    setup_clean_env(h)

    # 1. Ветка slot_alpha
    h.tp(50.5, 75.0, 50.5)
    h.wait(8)
    h.rzero("save slot_alpha")
    h.wait(4)

    # 2. Ветка slot_beta
    h.tp(0.5, 75.0, 0.5)
    h.wait(8)
    h.rzero("save slot_beta")
    h.wait(4)

    # 3. Список слотов
    slots = h.rzero("slots")
    slot_names = [s["slot"] for s in slots.get("slots", [])]
    assert "slot_alpha" in slot_names, f"slot_alpha missing from slots: {slots}"
    assert "slot_beta" in slot_names, f"slot_beta missing from slots: {slots}"

    # 4. Откат к slot_alpha
    h.rzero("load slot_alpha")
    h.wait(8)
    pos_a = h.state()["player"]["position"]
    assert abs(pos_a[0] - 50.5) < 1.5 and abs(pos_a[2] - 50.5) < 1.5, f"Did not rewind to alpha: {pos_a}"

    # 5. Откат к slot_beta
    h.rzero("load slot_beta")
    h.wait(8)
    pos_b = h.state()["player"]["position"]
    assert abs(pos_b[0] - 0.5) < 1.5 and abs(pos_b[2] - 0.5) < 1.5, f"Did not rewind to beta: {pos_b}"

    # 6. Очистка слотов
    h.rzero("slot delete slot_alpha")
    h.rzero("slot delete slot_beta")


def test_41_disk_checkpoint_reopen_and_rollback(h):
    setup_clean_env(h)
    h.tp(2400.5, 65.0, 2400.5)
    h.command("fill 2397 64 2397 2403 64 2403 minecraft:stone")
    h.command("fill 2397 65 2397 2403 70 2403 minecraft:air")
    h.command("kill @e[tag=rzero_disk_golem]")
    h.command("kill @e[tag=rzero_disk_drop]")
    h.command("setblock 2400 65 2402 minecraft:oak_log")
    h.command("setblock 2400 66 2402 minecraft:oak_log")
    h.command("summon minecraft:iron_golem 2402.5 65.0 2400.5 {NoAI:1b,Tags:['rzero_disk_golem']}")
    h.wait(6)

    hp_before = h.data_get_entity("@e[tag=rzero_disk_golem,limit=1]", "Health")

    h.rzero("set")
    h.wait(6)

    h.command("setblock 2400 65 2402 minecraft:air")
    h.command("setblock 2400 66 2402 minecraft:air")
    h.command("summon minecraft:item 2400.5 65.0 2402.5 {Item:{id:'minecraft:oak_log',count:2},Tags:['rzero_disk_drop']}")
    h.command("damage @e[tag=rzero_disk_golem,limit=1] 10 minecraft:player_attack by @s")
    h.wait(4)

    b_mod = h.block(2400, 65, 2402)
    assert "air" in b_mod.get("block", ""), f"Oak log was not broken: {b_mod}"
    hp_hurt = h.data_get_entity("@e[tag=rzero_disk_golem,limit=1]", "Health")
    assert hp_hurt != hp_before, f"Golem did not take damage: {hp_hurt}"

    h.rzero("purge_memory")
    h.wait(4)

    h.rzero("load")
    h.wait(10)

    b_restored_1 = h.block(2400, 65, 2402)
    b_restored_2 = h.block(2400, 66, 2402)
    assert b_restored_1.get("loaded") is True, f"Chunk was dropped/xray after disk rollback: {b_restored_1}"
    assert "oak_log" in b_restored_1.get("block", ""), f"Bottom oak log not restored from disk checkpoint: {b_restored_1}"
    assert "oak_log" in b_restored_2.get("block", ""), f"Top oak log not restored from disk checkpoint: {b_restored_2}"
    assert not h.has_entity("@e[tag=rzero_disk_drop]"), "Dropped oak logs remained after disk rollback"

    hp_restored = h.data_get_entity("@e[tag=rzero_disk_golem,limit=1]", "Health")
    assert hp_restored == hp_before, f"Golem health not restored from disk checkpoint: {hp_restored} vs {hp_before}"
    anger = h.data_get_entity("@e[tag=rzero_disk_golem,limit=1]", "AngerTime")
    assert (
        "не найдены" in anger.lower()
        or "no entity" in anger.lower()
        or "found no" in anger.lower()
        or " 0" in anger
    ), f"Golem remained angry after disk rollback: {anger}"

    h.command("setblock 2401 65 2401 minecraft:torch")
    h.wait(3)
    assert "torch" in h.block(2401, 65, 2401).get("block", ""), "Torch was not placed for second rollback"

    h.rzero("load")
    h.wait(6)
    assert "torch" not in h.block(2401, 65, 2401).get("block", ""), "Second rollback after disk hydration failed to remove torch"

    ground = h.block(2400, 64, 2400)
    assert ground.get("loaded") is True and "air" not in ground.get("block", ""), f"Ground became xray air: {ground}"

    h.command("kill @e[tag=rzero_disk_golem]")
    h.command("fill 2397 65 2397 2403 70 2403 minecraft:air")
    h.tp(0.5, 65.0, 0.5)
    h.wait(4)
