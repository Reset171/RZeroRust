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
