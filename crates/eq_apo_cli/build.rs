fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("ProductName", "Equalizer FD CRAFT");
        res.set("FileDescription", "Equalizer FD CRAFT - Studio Audio Engine");
        res.set("LegalCopyright", "Copyright (c) 2026 Equalizer FD CRAFT Team");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile windows resource: {}", e);
        }
    }
}
