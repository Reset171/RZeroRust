"""RZero test modules package."""

def setup_clean_env(h):
    """Restore player to normal survival, full health, solid ground, clear inventory."""
    h.command("gamemode survival")
    h.command("difficulty normal")
    h.command("setblock 0 64 0 minecraft:stone")
    h.command("setblock 0 65 0 minecraft:air")
    h.command("setblock 0 66 0 minecraft:air")
    h.command("effect give @s instant_health 1 10 true")
    h.command("clear @s")
    for _ in range(20):
        if h.state()["player"]["health"] >= 19.5:
            break
        h.command("effect give @s instant_health 1 10 true")
        h.wait(2)
    h.wait(3)
