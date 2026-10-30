fn hook_command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    crate::game::levels::rzero::handle_client_command(game, command, ran);
}