fn hook_lifecycle(game: &mut Game<'_>, phase: WorldPhase) {
    crate::game::levels::rzero::handle_lifecycle(game, phase);
}