#!/usr/bin/env python3
"""RZero automated End-to-End test suite for Horizon engine.

Implements full validation matrix:
- Test 1: Unloaded chunks isolation and time persistence
- Test 2: Massive TNT explosion rollback
- Test 3: TNT future ignition chain isolation
- Test 4: Fishing bobber entity preservation
- Test 5: Boat mount and vehicle state rollback
- Test 6: Remote chunks mob and entity cleanup
- Test 7: Villager AI pathfinding determinism
- Test 8: Spawner timing and entity determinism
- Test 9: Return by Death lethal fall interception (20 HP recovery)
- Test 10: Multiple consecutive death falls (anti-invulnerability check)
- Test 11: Timeline branching with named slots (alpha/beta jump)
- Test 12: Selective rollback policy (keep diamonds while rolling back world)
- Test 13: Machine-readable AI telemetry JSON validation
"""

import argparse
import json
import math
import sys
import time
from pathlib import Path

# Add project root to path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.rzero_harness import RZeroHarness


class TestRunner:
    def __init__(self, harness):
        self.h = harness
        self.passed = 0
        self.failed = 0
        self.results = []

    def log(self, name, status, duration, details=""):
        res = {
            "name": name,
            "status": status,
            "duration_ms": int(duration * 1000),
            "details": details,
        }
        self.results.append(res)
        color = "\033[92m[PASS]\033[0m" if status == "PASS" else "\033[91m[FAIL]\033[0m"
        print(f" {color} {name} ({res['duration_ms']} ms) {details}")
        if status == "PASS":
            self.passed += 1
        else:
            self.failed += 1

    def run_test(self, name, func):
        print(f"\n--- Running: {name} ---")
        start = time.time()
        try:
            func()
            self.log(name, "PASS", time.time() - start)
        except Exception as e:
            self.log(name, "FAIL", time.time() - start, str(e))


def setup_clean_env(h):
    h.command("gamemode survival")
    h.command("effect give @s instant_health 1 255")
    h.command("clear @s")
    h.wait(4)


# ============================================================================
# Test 1: Unloaded chunks isolation
# ============================================================================
def test_1_unloaded_chunks(h):
    setup_clean_env(h)
    h.command("setblock 0 64 0 minecraft:stone")
    h.command("setblock 0 65 0 minecraft:air")
    h.command("setblock 0 66 0 minecraft:air")
    h.tp(0.5, 65, 0.5)
    h.wait(8)

    # 1. Точка сохранения 1
    h.rzero("set")
    h.wait(4)

    # 2. Не загруженный чанк 99999
    h.tp(99999.5, 120, 99999.5)
    h.wait(15)
    h.command("setblock 99999 119 99999 minecraft:diamond_block")
    h.wait(4)
    b1 = h.block(99999, 119, 99999)
    assert "diamond_block" in b1.get("block", ""), f"Diamond block not set: {b1}"

    # 4. Прийти обратно
    h.tp(0.5, 65, 0.5)
    h.wait(10)

    # 5. Сделать точку сохранения 2
    h.rzero("set")
    h.wait(4)

    # 6. Не загруженный чанк 8888
    h.tp(8888.5, 120, 8888.5)
    h.wait(15)
    h.command("setblock 8888 119 8888 minecraft:emerald_block")
    h.wait(4)
    b2 = h.block(8888, 119, 8888)
    assert "emerald_block" in b2.get("block", ""), f"Emerald block not set: {b2}"

    # 8. Уйти на базу
    h.tp(0.5, 65, 0.5)
    h.wait(10)

    # 9. Сделать откат к точке 2
    h.rzero("load")
    h.wait(10)

    # Проверка: в 99999 алмазный блок остался
    h.tp(99999.5, 120, 99999.5)
    h.wait(15)
    chk_diamond = h.block(99999, 119, 99999)
    assert "diamond_block" in chk_diamond.get("block", ""), f"99999 lost changes: {chk_diamond}"

    # Проверка: в 8888 изумрудный блок откатился (стал воздухом/старым блоком)
    h.tp(8888.5, 120, 8888.5)
    h.wait(15)
    chk_emerald = h.block(8888, 119, 8888)
    assert "emerald_block" not in chk_emerald.get("block", ""), f"8888 was not rolled back: {chk_emerald}"

    # Очистка
    h.command("setblock 99999 119 99999 minecraft:air")
    h.tp(0.5, 65, 0.5)
    h.wait(6)


# ============================================================================
# Test 2: Massive TNT explosion rollback
# ============================================================================
def test_2_tnt_explosion_rollback(h):
    setup_clean_env(h)
    h.tp(200.5, 70, 200.5)
    h.wait(10)
    h.command("setblock 200 69 200 minecraft:stone")
    h.command("setblock 200 70 200 minecraft:air")
    h.wait(4)

    # 1. Чекпоинт до TNT
    h.rzero("set")
    h.wait(4)

    # 2. Создаем и поджигаем TNT
    h.command("fill 198 70 198 202 72 202 minecraft:tnt")
    h.wait(4)
    h.command("summon tnt 200 70 200")
    h.wait(6)

    # 4. Телепорт вдаль
    h.tp(99999.5, 120, 99999.5)
    h.wait(12)

    # 5. Откат
    h.rzero("load")
    h.wait(10)

    # Проверка базы: кратера нет, блок 200 69 200 на месте
    h.tp(200.5, 70, 200.5)
    h.wait(8)
    chk = h.block(200, 69, 200)
    assert "stone" in chk.get("block", ""), f"Crater detected, stone missing: {chk}"
    tnt_chk = h.block(200, 70, 200)
    assert "tnt" not in tnt_chk.get("block", ""), f"TNT blocks remained after rollback: {tnt_chk}"


# ============================================================================
# Test 3: TNT future ignition chain isolation
# ============================================================================
def test_3_tnt_future_chain_isolation(h):
    setup_clean_env(h)
    h.tp(300.5, 70, 300.5)
    h.wait(10)
    h.command("setblock 300 69 300 minecraft:bedrock")
    h.command("fill 299 70 299 301 70 301 minecraft:tnt")
    h.wait(4)

    # 2. Сделать точку сохранения с готовым TNT
    h.rzero("set")
    h.wait(4)

    # 3. Поджечь
    h.command("summon tnt 300 71 300")
    h.wait(6)

    # 4. Телепорт в 99999
    h.tp(99999.5, 120, 99999.5)
    h.wait(12)

    # 5. Откат
    h.rzero("load")
    h.wait(10)

    # Проверка: блоки TNT стоят на месте нетронутыми
    h.tp(300.5, 70, 300.5)
    h.wait(8)
    chk = h.block(300, 70, 300)
    assert "tnt" in chk.get("block", ""), f"TNT was destroyed by future explosion: {chk}"
    h.command("fill 299 70 299 301 70 301 minecraft:air")
    h.wait(4)


# ============================================================================
# Test 4: Fishing bobber preservation
# ============================================================================
def test_4_fishing_bobber_preservation(h):
    setup_clean_env(h)
    h.tp(400.5, 65, 400.5)
    h.wait(8)
    h.command("setblock 400 64 402 minecraft:water")
    h.wait(4)

    # 1. Создаем рыболовный крючок в воде
    h.command("summon fishing_bobber 400.5 64.5 402.5")
    h.wait(4)

    # 2. Точка сохранения
    h.rzero("set")
    h.wait(4)

    # 3. Телепорт вдаль
    h.tp(99999.5, 120, 99999.5)
    h.wait(10)

    # 4. Откат
    h.rzero("load")
    h.wait(10)

    # Проверка наличия сущности поплавка
    assert h.has_entity("@e[type=fishing_bobber,distance=..16]"), "Fishing bobber was not preserved after rollback"


# ============================================================================
# Test 5: Boat mount and vehicle state rollback
# ============================================================================
def test_5_boat_mount_rollback(h):
    setup_clean_env(h)
    h.tp(500.5, 65, 500.5)
    h.wait(8)
    h.command("kill @e[type=oak_boat,distance=..32]")
    h.command("summon oak_boat 500.5 65.0 500.5")
    h.wait(4)

    # 2. Чекпоинт
    h.rzero("set")
    h.wait(4)

    # 3. Ломаем лодки вокруг
    h.command("kill @e[type=oak_boat,distance=..10]")
    h.wait(4)
    assert not h.has_entity("@e[type=oak_boat,distance=..10]"), "Boat was not killed"
    h.tp(99999.5, 120, 99999.5)
    h.wait(10)

    # 5. Откат
    h.rzero("load")
    h.wait(12)

    assert h.has_entity("@e[type=oak_boat,distance=..16]"), "Boat was not restored after rollback"


# ============================================================================
# Test 6: Remote chunks mob and entity cleanup
# ============================================================================
def test_6_remote_chunks_mob_cleanup(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.wait(8)

    # 1. Точка сохранения
    h.rzero("set")
    h.wait(4)

    # 2. Телепорт в 1111 120 1111 и спавн мобов/лодок
    h.tp(1111.5, 120, 1111.5)
    h.wait(15)
    h.command("setblock 1111 119 1111 minecraft:stone")
    h.command("summon zombie 1111.5 120 1111.5")
    h.command("summon cow 1111.5 120 1111.5")
    h.command("summon oak_boat 1111.5 120 1111.5")
    h.wait(6)

    # 4. Телепорт в 2222
    h.tp(2222.5, 120, 2222.5)
    h.wait(15)

    # 5. Откат
    h.rzero("load")
    h.wait(10)

    # 6. Возвращение в 1111
    h.tp(1111.5, 120, 1111.5)
    h.wait(15)
    assert not h.has_entity("@e[type=zombie,distance=..32]"), "Zombies remained in remote chunk after rollback"
    assert not h.has_entity("@e[type=cow,distance=..32]"), "Cows remained in remote chunk after rollback"
    assert not h.has_entity("@e[type=oak_boat,distance=..32]"), "Boats remained in remote chunk after rollback"


# ============================================================================
# Test 7: Villager AI determinism
# ============================================================================
def test_7_villager_ai_determinism(h):
    setup_clean_env(h)
    h.tp(700.5, 65, 700.5)
    h.wait(8)
    h.command("fill 695 64 695 705 64 705 minecraft:stone")
    h.command("fill 695 65 695 705 66 705 minecraft:air")
    h.command("summon villager 698.5 65 700.5 {NoAI:0b}")
    h.command("summon villager 702.5 65 700.5 {NoAI:0b}")
    h.wait(6)

    # 2. Чекпоинт
    h.rzero("set")
    h.wait(4)

    # Запись движения 1
    h.rzero("load")
    h.wait(6)
    pos_run_1 = []
    for _ in range(5):
        h.wait(4)
        ents = h.entities(radius=10, entity_type="villager").get("entities", [])
        pos_run_1.append([(e["pos"][0], e["pos"][2]) for e in ents])

    # Запись движения 2 после отката
    h.rzero("load")
    h.wait(6)
    pos_run_2 = []
    for _ in range(5):
        h.wait(4)
        ents = h.entities(radius=10, entity_type="villager").get("entities", [])
        pos_run_2.append([(e["pos"][0], e["pos"][2]) for e in ents])

    assert len(pos_run_1) == len(pos_run_2), "Step counts differ"
    for s1, s2 in zip(pos_run_1, pos_run_2):
        assert len(s1) == len(s2), "Entity counts differ"
        for p1, p2 in zip(s1, s2):
            dx = abs(p1[0] - p2[0])
            dz = abs(p1[1] - p2[1])
            assert dx < 0.1 and dz < 0.1, f"Villager AI diverged: {p1} vs {p2}"


# ============================================================================
# Test 8: Monster spawner determinism
# ============================================================================
def test_8_spawner_determinism(h):
    setup_clean_env(h)
    h.tp(800.5, 65, 800.5)
    h.wait(8)
    h.command("kill @e[type=silverfish,distance=..32]")
    h.command("setblock 800 64 800 minecraft:stone")
    h.command("setblock 800 65 800 minecraft:spawner{SpawnData:{entity:{id:\"minecraft:silverfish\"}},MinSpawnDelay:20,MaxSpawnDelay:20,SpawnCount:1}")
    h.wait(4)

    # Чекпоинт
    h.rzero("set")
    h.wait(4)

    # Ждем спавна
    h.wait(35)
    assert h.has_entity("@e[type=silverfish,distance=..8]"), "Silverfish did not spawn from spawner"

    # Откат
    h.rzero("load")
    h.wait(4)
    assert not h.has_entity("@e[type=silverfish,distance=..8]"), "Silverfish remained immediately after rollback"

    # Повторное ожидание спавна
    h.wait(35)
    assert h.has_entity("@e[type=silverfish,distance=..8]"), "Silverfish did not spawn after rollback"

    # Очистка
    h.command("setblock 800 65 800 minecraft:air")
    h.command("kill @e[type=silverfish,distance=..16]")
    h.wait(4)


# ============================================================================
# Test 9: Return by Death lethal fall interception
# ============================================================================
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


# ============================================================================
# Test 10: Multiple consecutive death falls (anti-invulnerability)
# ============================================================================
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


# ============================================================================
# Test 11: Timeline branching with named slots
# ============================================================================
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


# ============================================================================
# Test 12: Selective rollback policy
# ============================================================================
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


# ============================================================================
# Test 13: Machine-readable AI telemetry JSON validation
# ============================================================================
def test_13_ai_json_telemetry(h):
    setup_clean_env(h)
    info = h.rzero("json")
    assert isinstance(info, dict), f"rzero json is not a valid dict: {info}"
    assert "has_checkpoint" in info, "has_checkpoint field missing"
    assert "game_time" in info, "game_time field missing"
    assert "combat_intensity" in info, "combat_intensity field missing"
    assert "player" in info, "player field missing"

    metrics = h.rzero("metrics")
    assert isinstance(metrics, dict), f"rzero metrics is not a valid dict: {metrics}"
    assert "threat_level" in metrics, "threat_level missing"
    assert "safe_to_save" in metrics, "safe_to_save missing"
    assert "recommended_action" in metrics, "recommended_action missing"


# ============================================================================
# Test 14: Chest container inventory rollback
# ============================================================================
def test_14_chest_container_rollback(h):
    setup_clean_env(h)
    h.tp(600.5, 65, 600.5)
    h.wait(8)
    h.command("setblock 600 65 600 minecraft:chest")
    h.wait(4)

    # 1. Чекпоинт с пустым сундуком
    h.rzero("set")
    h.wait(4)

    # 2. Наполняем сундук алмазами
    h.command("item replace block 600 65 600 container.0 with minecraft:diamond 64")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(8)

    # Проверка: в сундуке нет алмазов (откатился к пустому)
    res = h.command("data get block 600 65 600 Items")
    chat = res.get("chat", [])
    has_diamonds = any("diamond" in line.lower() for line in chat)
    assert not has_diamonds, f"Chest items were not rolled back: {chat}"
    h.command("setblock 600 65 600 minecraft:air")


# ============================================================================
# Test 15: Piston and redstone state rollback
# ============================================================================
def test_15_piston_redstone_machine(h):
    setup_clean_env(h)
    h.tp(650.5, 65, 650.5)
    h.wait(8)
    h.command("setblock 650 65 650 minecraft:piston[extended=false]")
    h.command("setblock 650 65 651 minecraft:redstone_wire[power=0]")
    h.wait(4)

    # Чекпоинт в выключенном состоянии
    h.rzero("set")
    h.wait(4)

    # Активируем редстоун блоком
    h.command("setblock 650 65 652 minecraft:redstone_block")
    h.wait(6)

    # Откат
    h.rzero("load")
    h.wait(8)

    b_piston = h.block(650, 65, 650)
    b_wire = h.block(650, 65, 651)
    b_source = h.block(650, 65, 652)

    assert "extended=false" in b_piston.get("state", ""), f"Piston state not rolled back: {b_piston}"
    assert "redstone_block" not in b_source.get("block", ""), f"Redstone block remained: {b_source}"

    # Очистка
    h.command("fill 649 65 649 651 65 653 minecraft:air")


# ============================================================================
# Test 16: Flying projectile arrow determinism
# ============================================================================
def test_16_flying_arrow_projectile(h):
    setup_clean_env(h)
    h.tp(750.5, 65, 750.5)
    h.wait(8)
    h.command("kill @e[type=arrow,distance=..32]")

    # Чекпоинт без стрел
    h.rzero("set")
    h.wait(4)

    # Спавним летящую стрелу
    h.command("summon arrow 750.5 70.0 750.5 {Motion:[0.0,1.0,0.0]}")
    h.wait(4)
    assert h.has_entity("@e[type=arrow,distance=..16]"), "Arrow was not spawned"

    # Откат
    h.rzero("load")
    h.wait(6)

    # Стрелы из будущего нет
    assert not h.has_entity("@e[type=arrow,distance=..16]"), "Future arrow remained after rollback"


# ============================================================================
# Main test suite runner
# ============================================================================
def main():
    parser = argparse.ArgumentParser(description="RZero E2E automated parity and determinism tests")
    parser.add_argument("--test", type=str, default="all", help="Test number or 'all'")
    args = parser.parse_args()

    h = RZeroHarness()
    print("[E2E] Connecting to Horizon game socket...")
    h.connect(timeout=15)
    h.wait_ready(timeout=20)
    print("[E2E] Connected and ready.")

    runner = TestRunner(h)

    tests = [
        ("Test 1: Unloaded Chunks Isolation", lambda: test_1_unloaded_chunks(h)),
        ("Test 2: Massive TNT Explosion Rollback", lambda: test_2_tnt_explosion_rollback(h)),
        ("Test 3: TNT Future Ignition Chain Isolation", lambda: test_3_tnt_future_chain_isolation(h)),
        ("Test 4: Fishing Bobber Preservation", lambda: test_4_fishing_bobber_preservation(h)),
        ("Test 5: Boat Mount Rollback", lambda: test_5_boat_mount_rollback(h)),
        ("Test 6: Remote Chunks Mob Cleanup", lambda: test_6_remote_chunks_mob_cleanup(h)),
        ("Test 7: Villager AI Determinism", lambda: test_7_villager_ai_determinism(h)),
        ("Test 8: Spawner Determinism", lambda: test_8_spawner_determinism(h)),
        ("Test 9: Return by Death Fatal Fall", lambda: test_9_return_by_death_fatal_fall(h)),
        ("Test 10: Multiple Fall Cycles Anti-Invulnerability", lambda: test_10_multiple_fall_cycles(h)),
        ("Test 11: Timeline Branching with Named Slots", lambda: test_11_slots_branching(h)),
        ("Test 12: Selective Rollback Policy", lambda: test_12_policy_selective_inventory(h)),
        ("Test 13: Machine-Readable AI Telemetry JSON", lambda: test_13_ai_json_telemetry(h)),
        ("Test 14: Chest Container Inventory Rollback", lambda: test_14_chest_container_rollback(h)),
        ("Test 15: Piston and Redstone State Rollback", lambda: test_15_piston_redstone_machine(h)),
        ("Test 16: Flying Projectile Arrow Determinism", lambda: test_16_flying_arrow_projectile(h)),
    ]

    selected = tests
    if args.test != "all":
        try:
            num = int(args.test)
            selected = [t for t in tests if t[0].startswith(f"Test {num}:")]
        except ValueError:
            selected = [t for t in tests if args.test.lower() in t[0].lower()]

    print(f"\n=======================================================")
    print(f"   RZero E2E Parity & Determinism Suite ({len(selected)} tests)")
    print(f"=======================================================")

    total_start = time.time()
    for name, fn in selected:
        runner.run_test(name, fn)

    total_duration = time.time() - total_start
    print(f"\n=======================================================")
    print(f" Total: {runner.passed + runner.failed} | Passed: \033[92m{runner.passed}\033[0m | Failed: \033[91m{runner.failed}\033[0m | Time: {total_duration:.2f}s")
    print(f"=======================================================")

    h.close()
    if runner.failed > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
