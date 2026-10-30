fn hook_key(game: &mut Game<'_>, key: &str, pressed: bool) {
    crate::game::levels::rzero::handle_key(game, key, pressed);
}