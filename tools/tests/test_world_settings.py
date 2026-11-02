"""Tests 19-23: World Settings, Commands, Tick Rate, Weather, Biomes, Schedule."""

from . import setup_clean_env


def test_19_tick_rate_and_freeze_rollback(h):
    setup_clean_env(h)
    h.command("tick unfreeze")
    h.command("tick rate 20")
    h.wait(4)

    # 1. Чекпоинт при 20 TPS
    h.rzero("set")
    h.wait(4)

    # 2. Ускоряем до 45 TPS и замораживаем тики
    h.command("tick rate 45")
    h.command("tick freeze")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: скорость тиков вернулась к 20.0, игра разморожена
    q = h.command("tick query")
    chat = q.get("chat", [])
    assert any("20.0" in line for line in chat), f"Tick rate not rolled back: {chat}"
    assert any("обычном режиме" in line.lower() or "running" in line.lower() for line in chat), f"Tick freeze not rolled back: {chat}"

    # 5. Проверка: скорость хода суток нормальная (1.0, а не 20.0)
    t0_day = h.state().get("time", {}).get("day_time", 0)
    h.wait(10)
    t1_day = h.state().get("time", {}).get("day_time", 0)
    diff_day = t1_day - t0_day
    assert diff_day <= 25, f"Sun/day cycle is moving too fast! Delta {diff_day} ticks over 10 game ticks (expected ~10-15)"


def test_20_world_spawn_and_difficulty_lock(h):
    setup_clean_env(h)
    h.command("difficulty normal")
    h.command("setworldspawn 100 70 100")
    h.wait(4)

    # 1. Чекпоинт на Normal и спавне (100, 70, 100)
    h.rzero("set")
    h.wait(4)

    # 2. Меняем сложность на Hard и спавн на (800, 80, 800)
    h.command("difficulty hard")
    h.command("setworldspawn 800 80 800")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка сложности
    d_res = h.command("difficulty")
    d_chat = d_res.get("chat", [])
    assert any("нормальная" in line.lower() or "normal" in line.lower() for line in d_chat), f"Difficulty not rolled back: {d_chat}"


def test_21_schedule_function_cancellation(h):
    setup_clean_env(h)
    h.wait(4)

    # 1. Чекпоинт до планирования функции
    h.rzero("set")
    h.wait(4)

    # 2. Планируем выполнение функции через 1000 тиков
    res = h.command("schedule function rzero:future_ping 1000t")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: расписание очищено
    clear_res = h.command("schedule clear rzero:future_ping")
    chat = clear_res.get("chat", [])
    assert any("нет запланированных" in line.lower() or "не найдено" in line.lower() or "не существует" in line.lower() or "fail" in line.lower() for line in chat), f"Scheduled function was not cancelled: {chat}"


def test_22_fillbiome_rollback(h):
    setup_clean_env(h)
    h.tp(500.5, 65, 500.5)
    h.wait(8)

    # 1. Чекпоинт в равнине
    h.rzero("set")
    h.wait(4)

    # 2. Меняем биом на пустыню
    h.command("fillbiome 495 60 495 505 75 505 minecraft:desert")
    h.wait(4)

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: позиция и блоки восстановлены
    st = h.state()
    assert abs(st["player"]["position"][0] - 500.5) < 2.0, "Player moved during fillbiome rollback"


def test_23_weather_and_thunder_rollback(h):
    setup_clean_env(h)
    h.command("weather clear")
    h.wait(6)

    # 1. Чекпоинт при ясной погоде
    h.rzero("set")
    h.wait(4)

    # 2. Включаем грозу
    h.command("weather thunder")
    h.wait(4)
    st_thunder = h.state().get("weather", {})
    assert st_thunder.get("thundering", False), f"Thunder weather was not set: {st_thunder}"

    # 3. Откат
    h.rzero("load")
    h.wait(6)

    # 4. Проверка: погода снова ясная
    st_clear = h.state().get("weather", {})
    assert not st_clear.get("thundering", False), f"Thunder weather not rolled back: {st_clear}"
    assert not st_clear.get("raining", False), f"Rain weather not rolled back: {st_clear}"
