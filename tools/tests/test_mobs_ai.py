"""Tests 6, 7, 8, 42: Remote chunk mobs cleanup, villager AI determinism, monster spawner, sheep state."""

from . import setup_clean_env


def test_6_remote_chunks_mob_cleanup(h):
    setup_clean_env(h)
    h.tp(1111.5, 120, 1111.5)
    h.command("kill @e[type=zombie,distance=..64]")
    h.command("kill @e[type=cow,distance=..64]")
    h.command("kill @e[type=oak_boat,distance=..64]")
    h.command("setblock 1111 119 1111 minecraft:air")
    h.wait(3)

    h.tp(0.5, 65, 0.5)
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.tp(1111.5, 120, 1111.5)
    h.command("setblock 1111 119 1111 minecraft:stone")
    h.command("summon zombie 1111.5 120 1111.5")
    h.command("summon cow 1111.5 120 1111.5")
    h.command("summon oak_boat 1111.5 120 1111.5")
    h.wait(4)
    assert h.has_entity("@e[type=zombie,distance=..32]"), "Zombie was not spawned in remote chunk"

    h.tp(2222.5, 120, 2222.5)
    h.wait(4)

    h.rzero("load")
    h.wait(6)

    h.tp(1111.5, 120, 1111.5)
    h.wait(6)
    assert not h.has_entity("@e[type=zombie,distance=..32]"), "Zombies remained in remote chunk after rollback"
    assert not h.has_entity("@e[type=cow,distance=..32]"), "Cows remained in remote chunk after rollback"
    assert not h.has_entity("@e[type=oak_boat,distance=..32]"), "Boats remained in remote chunk after rollback"


def test_7_villager_ai_determinism(h):
    setup_clean_env(h)
    h.command("weather clear")
    h.tp(700.5, 65, 700.5)
    h.command("kill @e[type=villager,distance=..64]")
    h.command("fill 695 64 695 705 64 705 minecraft:stone")
    h.command("fill 695 65 695 705 68 705 minecraft:air")
    h.command("summon villager 698.5 65 700.5 {NoAI:0b}")
    h.command("summon villager 702.5 65 700.5 {NoAI:0b}")
    for _ in range(15):
        if len(h.entities(radius=12, entity_type="villager").get("entities", [])) >= 2:
            break
        h.wait(2)

    h.command("tick freeze")
    h.wait(2)

    h.rzero("set")
    h.wait(2)

    h.rzero("load")
    h.wait(3)
    h.command("tick step 10")
    h.wait(12)
    ents_1 = {
        e["id"]: (round(e["pos"][0], 3), round(e["pos"][2], 3))
        for e in h.entities(radius=12, entity_type="villager").get("entities", [])
    }

    h.rzero("load")
    h.wait(3)
    h.command("tick step 10")
    h.wait(12)
    ents_2 = {
        e["id"]: (round(e["pos"][0], 3), round(e["pos"][2], 3))
        for e in h.entities(radius=12, entity_type="villager").get("entities", [])
    }

    h.command("tick unfreeze")
    h.wait(2)

    assert len(ents_1) >= 2 and len(ents_2) >= 2, f"Villagers missing in test area: {ents_1} vs {ents_2}"
    for vid, p1 in ents_1.items():
        assert vid in ents_2, f"Villager {vid} disappeared in run 2"
        p2 = ents_2[vid]
        dx = abs(p1[0] - p2[0])
        dz = abs(p1[1] - p2[1])
        assert dx < 0.15 and dz < 0.15, f"Villager {vid} AI diverged: {p1} vs {p2}"

    h.command("kill @e[type=villager,distance=..16]")
    h.wait(2)


def test_8_spawner_determinism(h):
    setup_clean_env(h)
    h.command("time set day")
    h.command("kill @e[type=pig]")
    h.tp(800.5, 65.0, 804.5)
    h.command("kill @e[type=pig]")
    h.command("fill 795 64 795 805 64 805 minecraft:grass_block")
    h.command("fill 795 65 795 805 68 805 minecraft:air")
    h.command(
        'setblock 800 65 800 minecraft:spawner{Delay:15s,MinSpawnDelay:40s,MaxSpawnDelay:40s,SpawnCount:1s,SpawnData:{entity:{id:"minecraft:pig"}}}'
    )
    h.wait(6)

    h.rzero("set")
    h.wait(4)

    h.wait(25)
    assert h.has_entity("@e[type=pig,x=800,y=65,z=800,distance=..8]"), "Pig did not spawn from spawner"

    h.rzero("load")
    assert not h.has_entity("@e[type=pig,x=800,y=65,z=800,distance=..8]"), "Pig remained immediately after rollback"

    h.wait(45)
    assert h.has_entity("@e[type=pig,x=800,y=65,z=800,distance=..8]"), "Pig did not spawn on schedule after rollback"

    h.command("setblock 800 65 800 minecraft:air")
    h.command("kill @e[type=pig,distance=..16]")
    h.wait(2)


def test_42_sheep_sheared_and_color_rollback(h):
    setup_clean_env(h)
    h.tp(0.5, 65, 0.5)
    h.command("kill @e[tag=rzero_sheep]")
    h.command("summon minecraft:sheep 0.5 65.0 2.5 {NoAI:1b,Sheared:0b,Color:0b,Tags:['rzero_sheep']}")
    h.wait(3)

    h.rzero("set")
    h.wait(2)

    h.command("data merge entity @e[tag=rzero_sheep,limit=1] {Sheared:1b,Color:14b}")
    h.wait(3)
    sheared_after = h.data_get_entity("@e[tag=rzero_sheep,limit=1]", "Sheared")
    color_after = h.data_get_entity("@e[tag=rzero_sheep,limit=1]", "Color")
    assert "1b" in sheared_after, f"Sheep was not sheared before rollback: {sheared_after}"
    assert "14b" in color_after, f"Sheep was not dyed red before rollback: {color_after}"

    h.rzero("load")
    h.wait(4)

    sheared_restored = h.data_get_entity("@e[tag=rzero_sheep,limit=1]", "Sheared")
    color_restored = h.data_get_entity("@e[tag=rzero_sheep,limit=1]", "Color")
    assert "0b" in sheared_restored, f"Sheep wool was not restored after rollback: {sheared_restored}"
    assert "0b" in color_restored, f"Sheep color was not restored after rollback: {color_restored}"
    h.command("kill @e[tag=rzero_sheep]")