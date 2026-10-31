"""RZero test modules package."""

def setup_clean_env(h):
    """Restore player to normal survival, full health, peaceful heal, clear inventory."""
    h.command("gamemode survival")
    h.command("difficulty peaceful")
    for _ in range(40):
        if h.state()["player"]["health"] >= 20.0:
            break
        h.wait(3)
    h.command("difficulty normal")
    h.command("clear @s")
    h.wait(4)
