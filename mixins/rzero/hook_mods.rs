fn hook_mods(mods: &mut Vec<NativeMod>) {
    crate::game::levels::rzero::register_native_mod(mods);
}