"""Tests 28-32: Advanced Mob AI, Anger, Leashes, Slimes, Parrots."""

from . import setup_clean_env


def test_28_shoulder_parrots_rollback(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("kill @e[tag=shoulder_parrot]")
    h.command("summon minecraft:parrot 0.5 65 1.5 {NoAI:1b,Variant:2,Sitting:1b,Tags:['shoulder_parrot']}")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("data merge entity @e[tag=shoulder_parrot,limit=1] {Variant:4,Sitting:0b}")
    h.command("kill @e[tag=shoulder_parrot]")
    h.tp(0.5, 75, 0.5)
    h.wait(4)
    assert not h.has_entity("@e[tag=shoulder_parrot]"), "Parrot was not killed"

    h.rzero("load")
    h.wait(4)

    assert h.has_entity("@e[tag=shoulder_parrot]"), "Parrot was not restored after rollback"
    variant = h.data_get_entity("@e[tag=shoulder_parrot,limit=1]", "Variant")
    assert "2" in variant, f"Parrot variant was not restored to 2: {variant}"
    h.command("kill @e[tag=shoulder_parrot]")


def test_29_neutral_mob_anger_reset(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("kill @e[tag=anger_wolf]")
    h.command("summon minecraft:wolf 0.5 65 2.5 {NoAI:1b,Tags:['anger_wolf']}")
    h.wait(3)

    hp_before = h.data_get_entity("@e[tag=anger_wolf,limit=1]", "Health")

    h.rzero("set")
    h.wait(2)

    h.command("damage @e[tag=anger_wolf,limit=1] 4 minecraft:player_attack by @s")
    h.wait(3)
    hp_hurt = h.data_get_entity("@e[tag=anger_wolf,limit=1]", "Health")
    assert hp_hurt != hp_before, f"Wolf did not take damage before rollback: {hp_hurt}"

    h.rzero("load")
    h.wait(4)

    hp_after = h.data_get_entity("@e[tag=anger_wolf,limit=1]", "Health")
    assert hp_after == hp_before, f"Wolf health was not restored: {hp_after} vs {hp_before}"
    anger = h.data_get_entity("@e[tag=anger_wolf,limit=1]", "AngerTime")
    assert "не найдены" in anger.lower() or "no entity" in anger.lower() or "found no" in anger.lower() or " 0" in anger, (
        f"Wolf remained angry after rollback: {anger}"
    )
    h.command("kill @e[tag=anger_wolf]")


def test_30_leash_distance_preservation(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("kill @e[tag=leashed_cow]")
    h.command("setblock 0 65 2 minecraft:oak_fence")
    h.command("summon minecraft:cow 0.5 65 3.5 {NoAI:1b,Tags:['leashed_cow'],leash:[0,65,2]}")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("setblock 0 65 2 minecraft:air")
    h.command("kill @e[tag=leashed_cow]")
    h.wait(4)
    assert not h.has_entity("@e[tag=leashed_cow]"), "Cow was not removed before rollback"

    h.rzero("load")
    h.wait(4)

    assert h.has_entity("@e[tag=leashed_cow]"), "Leashed cow disappeared after rollback"
    assert "oak_fence" in h.block(0, 65, 2).get("block", ""), "Fence post was not restored"
    h.command("kill @e[tag=leashed_cow]")
    h.command("setblock 0 65 2 minecraft:air")


def test_31_slime_jump_delay_determinism(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("kill @e[tag=slime_timing]")
    h.command("fill -4 64 0 4 64 8 minecraft:stone")
    h.command("summon minecraft:slime 0.5 65 3.5 {Size:1,Tags:['slime_timing']}")
    h.wait(4)

    h.command("tick freeze")
    h.wait(2)

    h.rzero("set")
    h.wait(2)

    h.rzero("load")
    h.wait(2)
    h.command("tick step 10")
    h.wait(12)
    assert h.has_entity("@e[tag=slime_timing]"), "Slime missing in run 1"
    pos_1 = h.data_get_entity("@e[tag=slime_timing,limit=1]", "Pos")

    h.rzero("load")
    h.wait(2)
    h.command("tick step 10")
    h.wait(12)
    assert h.has_entity("@e[tag=slime_timing]"), "Slime missing in run 2"
    pos_2 = h.data_get_entity("@e[tag=slime_timing,limit=1]", "Pos")

    h.command("tick unfreeze")
    h.command("kill @e[tag=slime_timing]")
    h.command("kill @e[type=slime,distance=..16]")

    assert pos_1 == pos_2, f"Slime server position diverged between loops: {pos_1} vs {pos_2}"


def test_32_sleeping_villager_day_schedule(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("time set 1000")
    h.command("kill @e[tag=sched_villager]")
    h.command("summon minecraft:villager 0.5 65 2.5 {Tags:['sched_villager']}")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("time set 18000")
    h.wait(4)
    day_night = h.state().get("time", {}).get("day_time", 0)
    assert day_night >= 18000, f"Time did not advance to midnight: {day_night}"

    h.rzero("load")
    h.wait(4)

    day_restored = h.state().get("time", {}).get("day_time", 0)
    assert 1000 <= day_restored <= 1100, f"Day time was not restored: {day_restored}"
    assert h.has_entity("@e[tag=sched_villager]"), "Villager missing after schedule rollback"
    h.command("kill @e[tag=sched_villager]")