fn main() {
    // Probe for webkit2gtk-4.1 first (newer distros),
    // fall back to webkit2gtk-4.0 (older distros like Ubuntu 22.04).
    let found = pkg_config::Config::new()
        .atleast_version("2.40")
        .probe("webkit2gtk-4.1")
        .or_else(|_| {
            pkg_config::Config::new()
                .atleast_version("2.36")
                .probe("webkit2gtk-4.0")
        });

    match found {
        Ok(lib) => {
            for path in &lib.link_paths {
                println!("cargo:rustc-link-search=native={}", path.display());
            }
        }
        Err(e) => {
            eprintln!(
                "ERROR: Could not find webkit2gtk.\n\
                 Install it with:\n\
                   Debian/Ubuntu: sudo apt install libwebkit2gtk-4.1-dev\n\
                   Fedora:        sudo dnf install webkit2gtk4.1-devel\n\
                   Arch:          sudo pacman -S webkit2gtk-4.1\n\
                 \nOriginal error: {e}"
            );
            std::process::exit(1);
        }
    }
}
