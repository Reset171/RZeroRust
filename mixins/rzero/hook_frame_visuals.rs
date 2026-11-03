fn hook_frame_visuals(
    game: &Game<'_>,
    lines: &mut Vec<horizon_client::gizmos::Line>,
) {
    crate::game::levels::rzero::process_hitbox_lines(game, lines);
}
