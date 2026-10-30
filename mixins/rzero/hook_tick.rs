fn hook_tick(game: &mut Game<'_>) {
    crate::game::levels::rzero::handle_tick(game);
}