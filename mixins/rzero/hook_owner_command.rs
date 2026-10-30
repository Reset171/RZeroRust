fn hook_owner_command(
    game: &mut Game<'_>,
    command: &str,
) -> std::ops::ControlFlow<Vec<String>> {
    crate::game::levels::rzero::intercept_owner_console_command(game, command)
}