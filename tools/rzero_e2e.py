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
from tools.tests.test_world_settings import (
    test_19_tick_rate_and_freeze_rollback,
    test_20_world_spawn_and_difficulty_lock,
    test_21_schedule_function_cancellation,
    test_22_fillbiome_rollback,
    test_23_weather_and_thunder_rollback,
)
from tools.tests.test_portals import (
    test_24_nether_portal_cold_chunk,
    test_25_flying_ender_pearl_discard,
    test_26_portal_processor_cooldown,
    test_27_end_credits_respawn_safety,
)
from tools.tests.test_mobs_advanced import (
    test_28_shoulder_parrots_rollback,
    test_29_neutral_mob_anger_reset,
    test_30_leash_distance_preservation,
    test_31_slime_jump_delay_determinism,
    test_32_sleeping_villager_day_schedule,
)
from tools.tests.test_containers_advanced import (
    test_33_furnace_cook_and_burn_time,
    test_34_hopper_transfer_cooldown,
    test_35_item_frame_duplication_safety,
    test_36_creative_cursor_item_retention,
)
from tools.tests.test_determinism_advanced import (
    test_37_raid_determinism_and_coords,
    test_38_chest_loot_table_determinism,
    test_39_explored_map_fog_of_war,
    test_40_subchunk_palette_expansion_bounds,
)


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
        (19, "Test 19: Tick Rate and Freeze Rollback", "world", test_19_tick_rate_and_freeze_rollback),
        (20, "Test 20: World Spawn and Difficulty Lock", "world", test_20_world_spawn_and_difficulty_lock),
        (21, "Test 21: Scheduled Function Cancellation", "world", test_21_schedule_function_cancellation),
        (22, "Test 22: Biome Palette FillBiome Rollback", "world", test_22_fillbiome_rollback),
        (23, "Test 23: Weather and Thunder State Rollback", "world", test_23_weather_and_thunder_rollback),
        (24, "Test 24: Nether Portal Cold Chunk Disappearance Fix", "portals", test_24_nether_portal_cold_chunk),
        (25, "Test 25: Flying Ender Pearl Discard", "portals", test_25_flying_ender_pearl_discard),
        (26, "Test 26: Portal Processor Cooldown Reset", "portals", test_26_portal_processor_cooldown),
        (27, "Test 27: End Credits Respawn Safety", "portals", test_27_end_credits_respawn_safety),
        (28, "Test 28: Shoulder Parrots Rollback", "mobs", test_28_shoulder_parrots_rollback),
        (29, "Test 29: Neutral Mob Anger Reset", "mobs", test_29_neutral_mob_anger_reset),
        (30, "Test 30: Leash Distance Preservation", "mobs", test_30_leash_distance_preservation),
        (31, "Test 31: Slime Jump Delay Determinism", "mobs", test_31_slime_jump_delay_determinism),
        (32, "Test 32: Sleeping Villager Day Schedule Sync", "mobs", test_32_sleeping_villager_day_schedule),
        (33, "Test 33: Advanced Containers (Barrel Items) Rollback", "containers", test_33_furnace_cook_and_burn_time),
        (34, "Test 34: Hopper Transfer Cooldown Preservation", "containers", test_34_hopper_transfer_cooldown),
        (35, "Test 35: ArmorStand Equipment Duplication Safety", "containers", test_35_item_frame_duplication_safety),
        (36, "Test 36: Player Cursor Inventory Retention", "containers", test_36_creative_cursor_item_retention),
        (37, "Test 37: Raid Bad Omen Determinism", "determinism", test_37_raid_determinism_and_coords),
        (38, "Test 38: Chest Loot Table Preservation", "determinism", test_38_chest_loot_table_determinism),
        (39, "Test 39: Explored Map Fog of War Rollback", "determinism", test_39_explored_map_fog_of_war),
        (40, "Test 40: Subchunk Palette Bounds Y 318 and -50", "determinism", test_40_subchunk_palette_expansion_bounds),
    ]


def main():
    parser = argparse.ArgumentParser(description="RZero E2E modular test suite for Horizon engine")
    parser.add_argument("--test", type=str, default="all", help="Test number or search string")
    parser.add_argument("--group", type=str, default=None, help="Filter by group (chunks, explosions, vehicles, mobs, death, slots, policies, containers, dimensions, ai, world, portals, determinism)")
    parser.add_argument("--fast", "--smoke", action="store_true", help="Run 5 fast representative smoke tests (10-12s) instead of the full suite")
    args = parser.parse_args()

    h = RZeroHarness()
    print("[E2E] Connecting to Horizon game socket...")
    h.connect(timeout=45)
    h.wait_ready(timeout=45)
    print("[E2E] Connected and ready.")

    runner = TestRunner(h)
    catalog = build_test_catalog()

    selected = catalog
    if args.fast:
        fast_ids = {1, 2, 9, 14, 19}
        selected = [t for t in selected if t[0] in fast_ids]
    elif args.group:
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
