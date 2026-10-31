#!/usr/bin/env python3
"""RZero modular automated End-to-End test suite for Horizon engine.

Usage:
    python3 tools/rzero_e2e.py                 # Run all 17 tests
    python3 tools/rzero_e2e.py --test 7        # Run specific test number
    python3 tools/rzero_e2e.py --group chunks  # Run specific test group
"""

import argparse
import sys
import time
from pathlib import Path

# Add project root to path
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tools.rzero_harness import RZeroHarness
from tools.tests.test_chunks import test_1_unloaded_chunks
from tools.tests.test_explosions import (
    test_2_tnt_explosion_rollback,
    test_3_tnt_future_chain_isolation,
)
from tools.tests.test_vehicles import (
    test_4_fishing_bobber_preservation,
    test_5_boat_mount_rollback,
    test_16_flying_arrow_projectile,
)
from tools.tests.test_mobs_ai import (
    test_6_remote_chunks_mob_cleanup,
    test_7_villager_ai_determinism,
    test_8_spawner_determinism,
)
from tools.tests.test_death_rewind import (
    test_9_return_by_death_fatal_fall,
    test_10_multiple_fall_cycles,
    test_18_drowning_and_damage_after_rewind,
)
from tools.tests.test_slots import test_11_slots_branching
from tools.tests.test_policies import test_12_policy_selective_inventory
from tools.tests.test_containers import (
    test_14_chest_container_rollback,
    test_15_piston_redstone_machine,
)
from tools.tests.test_dimensions import test_17_end_dragon_bossbar_rollback
from tools.tests.test_ai_telemetry import test_13_ai_json_telemetry


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
            func(self.h)
            self.log(name, "PASS", time.time() - start)
        except Exception as e:
            self.log(name, "FAIL", time.time() - start, str(e))


def build_test_catalog():
    return [
        # (id, name, group, function)
        (1, "Test 1: Unloaded Chunks Isolation", "chunks", test_1_unloaded_chunks),
        (2, "Test 2: Massive TNT Explosion Rollback", "explosions", test_2_tnt_explosion_rollback),
        (3, "Test 3: TNT Future Ignition Chain Isolation", "explosions", test_3_tnt_future_chain_isolation),
        (4, "Test 4: Fishing Bobber Preservation", "vehicles", test_4_fishing_bobber_preservation),
        (5, "Test 5: Boat Mount Rollback", "vehicles", test_5_boat_mount_rollback),
        (6, "Test 6: Remote Chunks Mob Cleanup", "mobs", test_6_remote_chunks_mob_cleanup),
        (7, "Test 7: Villager AI Determinism", "mobs", test_7_villager_ai_determinism),
        (8, "Test 8: Spawner Determinism", "mobs", test_8_spawner_determinism),
        (9, "Test 9: Return by Death Fatal Fall", "death", test_9_return_by_death_fatal_fall),
        (10, "Test 10: Multiple Fall Cycles Anti-Invulnerability", "death", test_10_multiple_fall_cycles),
        (11, "Test 11: Timeline Branching with Named Slots", "slots", test_11_slots_branching),
        (12, "Test 12: Selective Rollback Policy", "policies", test_12_policy_selective_inventory),
        (13, "Test 13: Machine-Readable AI Telemetry JSON", "ai", test_13_ai_json_telemetry),
        (14, "Test 14: Chest Container Inventory Rollback", "containers", test_14_chest_container_rollback),
        (15, "Test 15: Piston and Redstone State Rollback", "containers", test_15_piston_redstone_machine),
        (16, "Test 16: Flying Projectile Arrow Determinism", "vehicles", test_16_flying_arrow_projectile),
        (17, "Test 17: End Dragon Bossbar Cross-Dimension Rollback", "dimensions", test_17_end_dragon_bossbar_rollback),
        (18, "Test 18: Drowning and Damage After Rewind", "death", test_18_drowning_and_damage_after_rewind),
    ]


def main():
    parser = argparse.ArgumentParser(description="RZero E2E modular test suite for Horizon engine")
    parser.add_argument("--test", type=str, default="all", help="Test number or search string")
    parser.add_argument("--group", type=str, default=None, help="Filter by group (chunks, explosions, vehicles, mobs, death, slots, policies, containers, dimensions, ai)")
    args = parser.parse_args()

    h = RZeroHarness()
    print("[E2E] Connecting to Horizon game socket...")
    h.connect(timeout=45)
    h.wait_ready(timeout=45)
    print("[E2E] Connected and ready.")

    runner = TestRunner(h)
    catalog = build_test_catalog()

    selected = catalog
    if args.group:
        selected = [t for t in selected if t[2].lower() == args.group.lower()]

    if args.test != "all":
        try:
            num = int(args.test)
            selected = [t for t in selected if t[0] == num]
        except ValueError:
            selected = [t for t in selected if args.test.lower() in t[1].lower()]

    print(f"\n=======================================================")
    print(f"   RZero Modular E2E Suite ({len(selected)} tests)")
    print(f"=======================================================")

    total_start = time.time()
    for _, name, _, fn in selected:
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
