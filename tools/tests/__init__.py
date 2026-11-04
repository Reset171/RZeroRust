"""RZero test modules package."""


def setup_clean_env(h):
    """Restore server, policy, and player to a clean, loaded survival platform."""
    h.command("tick unfreeze")
    h.command("tick rate 20")
    h.command("forceload remove all")
    h.rzero("policy reset")
    h.rzero("slot select default")
    h.rzero("autosave off")

    st = h.state()
    if st.get("screen") not in ("playing", None):
        try:
            h.close_screen()
        except Exception:
            pass

    if st.get("dimension") != "minecraft:overworld":
        h.command("execute in minecraft:overworld run tp @s 0.5 65.0 0.5")
    else:
        pos = st.get("player", {}).get("position", [999.0, 65.0, 999.0])
        if abs(pos[0] - 0.5) > 16.0 or abs(pos[2] - 0.5) > 16.0 or pos[1] < 60.0 or pos[1] > 100.0:
            h.command("tp @s 0.5 65.0 0.5")

    h.ensure_loaded(0, 64, 0, timeout_ticks=120)
    h.command("fill -3 64 -3 3 64 3 minecraft:stone")
    h.command("fill -3 65 -3 3 70 3 minecraft:air")
    h.command("kill @e[type=!player,distance=..24]")
    h.command("tp @s 0.5 65.0 0.5 0 0")
    h.command("gamemode survival")
    h.command("difficulty normal")
    h.command("time set day")
    h.command("weather clear")
    h.command("effect clear @s")
    h.command("effect give @s instant_health 1 10 true")
    h.command("clear @s")
    for _ in range(15):
        p = h.state().get("player", {})
        if p.get("health", 0) >= 19.5 and p.get("on_ground", False):
            break
        h.command("effect give @s instant_health 1 10 true")
        h.wait(2)
    h.command("effect clear @s")
    h.wait(2)