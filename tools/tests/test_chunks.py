"""Test 1: Unloaded chunks isolation and time persistence."""

import time
from . import setup_clean_env


def test_1_unloaded_chunks(h):
    setup_clean_env(h)
    offset = int(time.time()) % 500
    ax = 90000 + offset * 16
    bx = 80000 + offset * 16

    h.rzero("set")
    h.wait(2)

    h.tp(ax + 0.5, 120, ax + 0.5)
    h.wait(4)
    h.command(f"setblock {ax} 119 {ax} minecraft:diamond_block")
    h.wait(2)
    b1 = h.block(ax, 119, ax)
    assert "diamond_block" in b1.get("block", ""), f"Diamond block not set: {b1}"

    h.tp(0.5, 65, 0.5)
    h.wait(4)

    h.rzero("set")
    h.wait(2)

    h.tp(bx + 0.5, 120, bx + 0.5)
    h.wait(4)
    h.command(f"setblock {bx} 119 {bx} minecraft:emerald_block")
    h.wait(2)
    b2 = h.block(bx, 119, bx)
    assert "emerald_block" in b2.get("block", ""), f"Emerald block not set: {b2}"

    h.tp(0.5, 65, 0.5)
    h.wait(4)

    h.rzero("load")
    h.wait(6)

    h.tp(ax + 0.5, 120, ax + 0.5)
    h.wait(4)
    chk_diamond = h.block(ax, 119, ax)
    assert "diamond_block" in chk_diamond.get("block", ""), f"Chunk A lost changes: {chk_diamond}"

    h.tp(bx + 0.5, 120, bx + 0.5)
    h.wait(4)
    chk_emerald = h.block(bx, 119, bx)
    assert "emerald_block" not in chk_emerald.get("block", ""), f"Chunk B was not rolled back: {chk_emerald}"

    h.command(f"setblock {ax} 119 {ax} minecraft:air")
    h.command(f"setblock {bx} 119 {bx} minecraft:air")
    h.tp(0.5, 65, 0.5)
    h.wait(2)