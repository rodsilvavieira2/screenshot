use std::process::Command;

fn main() {
    // Check if we're on Linux
    if cfg!(target_os = "linux") {
        // Try to find GTK4 using pkg-config
        if let Ok(output) = Command::new("pkg-config")
            .args(&["--cflags", "gtk4"])
            .output()
        {
            if output.status.success() {
                println!("cargo:rustc-env=GTK4_FOUND=1");
            } else {
                println!("cargo:warning=GTK4 not found via pkg-config. Please install libgtk-4-dev or gtk4-devel");
            }
        } else {
            println!("cargo:warning=pkg-config not found. Please install pkg-config and GTK4 development packages");
        }

        // Check for Cairo
        if let Ok(output) = Command::new("pkg-config")
            .args(&["--cflags", "cairo"])
            .output()
        {
            if !output.status.success() {
                println!("cargo:warning=Cairo not found via pkg-config. Please install libcairo2-dev or cairo-devel");
            }
        }

        // Check for GLib
        if let Ok(output) = Command::new("pkg-config")
            .args(&["--cflags", "glib-2.0"])
            .output()
        {
            if !output.status.success() {
                println!("cargo:warning=GLib not found via pkg-config. Please install libglib2.0-dev or glib2-devel");
            }
        }
    } else {
        println!("cargo:warning=This application is designed for Linux only");
    }

    // Rerun if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");
}