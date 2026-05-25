use anyhow::Result;
use std::env;
use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;
use std::time::SystemTime;
use zip::ZipArchive;

// CONFIGURATION
// Since we set up the GitHub Action to release to 'latest', we can use this URL.
const DASHBOARD_URL: &str =
    "https://github.com/temper-mc/dashboard/releases/download/latest/temper-dashboard.zip";

fn main() -> Result<()> {
    // Tell Cargo about our custom cfg flags to suppress warnings
    println!("cargo::rustc-check-cfg=cfg(dashboard_in_out_dir)");
    println!("cargo::rustc-check-cfg=cfg(dashboard_in_manifest_dir)");
    println!("cargo::rustc-check-cfg=cfg(dashboard_build)");

    // Rerun if the force download environment variable changes
    println!("cargo:rerun-if-env-changed=TEMPER_FORCE_DASHBOARD_DOWNLOAD");

    // 1. Determine where to put the files.
    let (dest_dir, use_out_dir) = if let Ok(out_dir) = env::var("OUT_DIR") {
        let out_path = Path::new(&out_dir);
        // Check if OUT_DIR actually exists and is accessible
        if out_path.exists() {
            (out_path.join("dashboard"), true)
        } else {
            let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
            (Path::new(&manifest_dir).join("dashboard-dist"), false)
        }
    } else {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
        (Path::new(&manifest_dir).join("dashboard-dist"), false)
    };

    // Emit which path we're using so include_dir! knows where to look
    if use_out_dir {
        println!("cargo:rustc-cfg=dashboard_in_out_dir");
    } else {
        println!("cargo:rustc-cfg=dashboard_in_manifest_dir");
    }

    // 2. Check cache age to avoid downloading on every single compile.
    let force_download = env::var("TEMPER_FORCE_DASHBOARD_DOWNLOAD").is_ok();
    let index_html = dest_dir.join("index.html");
    
    let has_recent_dashboard = if index_html.exists() {
        if let Ok(metadata) = fs::metadata(&index_html) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    // Cache threshold: 24 hours (86,400 seconds)
                    elapsed.as_secs() < 86400
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    let should_download = force_download || !has_recent_dashboard;

    if should_download {
        println!(
            "Downloading Dashboard artifact from {}",
            DASHBOARD_URL
        );

        let client = reqwest::blocking::Client::new();
        let download_result = client
            .get(DASHBOARD_URL)
            .header("User-Agent", "temper-build-script")
            .send();

        match download_result {
            Ok(response) if response.status().is_success() => {
                // Delete existing directory if it exists
                if dest_dir.exists() {
                    fs::remove_dir_all(&dest_dir)?;
                }

                let content = response.bytes()?;

                // Extract the zip file
                let cursor = Cursor::new(content);
                let mut archive = ZipArchive::new(cursor)?;

                // Create the destination directory
                fs::create_dir_all(&dest_dir)?;

                // Extract all files
                for i in 0..archive.len() {
                    let mut file = archive.by_index(i)?;
                    let outpath = dest_dir.join(file.mangled_name()?);

                    if file.is_dir() {
                        fs::create_dir_all(&outpath)?;
                    } else {
                        if let Some(parent) = outpath.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut outfile = fs::File::create(&outpath)?;
                        std::io::copy(&mut file, &mut outfile)?;
                    }
                }

                println!("Dashboard extracted to {:?}", dest_dir);
            }
            _ => {
                // Download failed (no internet, request error, non-success status)
                if dest_dir.exists() {
                    // Use existing dashboard files
                    println!("cargo:warning=Download failed, using existing dashboard files.");
                } else {
                    // Create fallback HTML
                    println!(
                        "cargo:warning=Download failed and no existing dashboard. Creating fallback."
                    );
                    fs::create_dir_all(&dest_dir)?;
                    let mut file = fs::File::create(dest_dir.join("index.html"))?;
                    file.write_all(b"<h1>Dashboard Offline (Build failed to fetch)</h1>")?;
                }
            }
        }
    } else {
        println!(
            "Using cached Dashboard (last downloaded <24h ago). Set TEMPER_FORCE_DASHBOARD_DOWNLOAD=1 to force update."
        );
    }

    // Emit a cfg flag so the main code knows build.rs ran
    println!("cargo:rustc-cfg=dashboard_build");

    Ok(())
}
