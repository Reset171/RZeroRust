fn hook_settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::levels::rzero::populate_settings(game, mod_id, buttons);
}