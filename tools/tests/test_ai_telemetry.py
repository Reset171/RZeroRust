"""Test 13: Machine-readable AI telemetry JSON validation."""

from . import setup_clean_env


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
