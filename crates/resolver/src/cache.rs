pub fn directory() -> std::path::PathBuf {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| {
        eprintln!("Could not determine cache directory");
        std::process::exit(1);
    });
    cache_dir.join("plum")
}
