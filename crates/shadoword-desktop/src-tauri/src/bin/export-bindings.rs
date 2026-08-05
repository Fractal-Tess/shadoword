fn main() {
    if let Err(error) = shadoword_desktop_lib::export_bindings() {
        eprintln!("failed to export TypeScript bindings: {error}");
        std::process::exit(1);
    }
}
