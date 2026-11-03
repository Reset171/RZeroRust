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
    h.tp(0.5, 65.0, 0.5)
    h.wait(8)
    h.command("setblock 0 64 2 minecraft:stone")
    h.command("setblock 0 65 2 minecraft:stone")
    h.wait(4)

    # 1. Сохраняем чекпоинт на диск
    h.rzero("set")
    h.wait(6)

    # 2. Модифицируем мир (ставим обсидиан)
    h.command("setblock 0 65 2 minecraft:obsidian")
    h.wait(4)
    b_mod = h.block(0, 65, 2)
    assert "obsidian" in b_mod.get("block", ""), f"Obsidian was not placed: {b_mod}"

    # 3. Симулируем перезаход в мир: очищаем RAM (сброс снапшота в памяти)
    h.rzero("purge_memory")
    h.wait(4)

    # 4. Выполняем откат с диска
    h.rzero("load")
    h.wait(10)

    # 5. Проверка: блок откатился к stone, чанк полностью загружен (не рентген, loaded=True)
    b_restored = h.block(0, 65, 2)
    assert b_restored.get("loaded") is True, f"Chunk was dropped/xray after disk rollback: {b_restored}"
    assert "stone" in b_restored.get("block", ""), f"Block was not restored from disk checkpoint: {b_restored}"

    # 6. Проверка окружающего блока земли под ногами (твердый блок не превратился в пустоту)
    ground = h.block(0, 64, 0)
    assert ground.get("loaded") is True and "air" not in ground.get("block", ""), f"Ground became xray air: {ground}"

    h.command("setblock 0 65 2 minecraft:air")
    h.wait(4)
