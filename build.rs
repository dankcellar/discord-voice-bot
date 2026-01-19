use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const MODEL_NAME: &str = "vosk-model-en-us-0.22";
const MODEL_URL: &str = "https://alphacephei.com/vosk/models/vosk-model-en-us-0.22.zip";

const VOSK_WINDOWS_URL: &str =
    "https://github.com/alphacep/vosk-api/releases/download/v0.3.45/vosk-win64-0.3.45.zip";
const VOSK_LINUX_URL: &str =
    "https://github.com/alphacep/vosk-api/releases/download/v0.3.45/vosk-linux-x86_64-0.3.45.zip";
const VOSK_MACOS_URL: &str =
    "https://github.com/alphacep/vosk-api/releases/download/v0.3.45/vosk-osx-0.3.45.zip";

type BuildResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=VOSK_LIB_DIR");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| String::from("unknown"));

    // Setup Vosk library
    if let Err(e) = setup_vosk_library(&target_os) {
        println!("cargo:warning=Failed to setup Vosk library: {}", e);
        println!("cargo:warning=Set VOSK_LIB_DIR to specify library location manually");
    }

    // Setup Vosk model
    let models_dir = PathBuf::from("models");
    let model_path = models_dir.join(MODEL_NAME);

    if model_path.exists() {
        println!("cargo:warning=⚠️  Model corrupted, re-downloading...");
        let _ = fs::remove_dir_all(&model_path);
    }

    println!("cargo:warning=Downloading Vosk model...");
    if let Err(e) = download_and_extract_model(&models_dir) {
        println!("cargo:warning=Failed to download model: {}", e);
        println!("cargo:warning=Download manually from: {}", MODEL_URL);
        println!("cargo:warning=Extract to: {}", models_dir.display());
    } else {
        println!("cargo:warning=✅ Vosk model ready");
    }
}

fn setup_vosk_library(target_os: &str) -> BuildResult<()> {
    // Use custom library path if provided
    if let Ok(vosk_lib_dir) = env::var("VOSK_LIB_DIR") {
        println!("cargo:warning=Using custom VOSK_LIB_DIR: {}", vosk_lib_dir);
        println!("cargo:rustc-link-search=native={}", vosk_lib_dir);
        return Ok(());
    }

    let (url, lib_name) = match target_os {
        "windows" => (VOSK_WINDOWS_URL, "libvosk.dll"),
        "linux" => (VOSK_LINUX_URL, "libvosk.so"),
        "macos" => (VOSK_MACOS_URL, "libvosk.dylib"),
        _ => return Err(format!("Unsupported platform: {}", target_os).into()),
    };

    let lib_dir = PathBuf::from("lib").join(target_os);
    let lib_file = lib_dir.join(lib_name);

    // Skip if library already exists
    if lib_file.exists() && lib_file.metadata()?.len() > 0 {
        println!("cargo:warning=✅ Using existing Vosk library");
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        copy_runtime_dependencies(&lib_dir)?;
        return Ok(());
    }

    println!(
        "cargo:warning=Downloading Vosk library for {}...",
        target_os
    );
    fs::create_dir_all(&lib_dir)?;

    let zip_path = lib_dir.join("vosk.zip");
    download_file(url, &zip_path)?;

    println!("cargo:warning=Extracting Vosk library...");
    extract_library_files(&zip_path, &lib_dir)?;
    fs::remove_file(&zip_path)?;

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    copy_runtime_dependencies(&lib_dir)?;

    println!("cargo:warning=✅ Vosk library ready");
    Ok(())
}

fn copy_runtime_dependencies(lib_dir: &Path) -> BuildResult<()> {
    let out_dir = env::var("OUT_DIR")?;
    let out_path = PathBuf::from(out_dir);
    let target_dir = out_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or("Could not determine target directory")?;

    for entry in fs::read_dir(lib_dir)? {
        let path = entry?.path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if matches!(ext, "dll" | "so" | "dylib") {
                let file_name = path.file_name().ok_or("Invalid filename")?;
                fs::copy(&path, target_dir.join(file_name))?;
            }
        }
    }

    Ok(())
}

fn download_and_extract_model(models_dir: &Path) -> BuildResult<()> {
    fs::create_dir_all(models_dir)?;
    let zip_path = models_dir.join("model.zip");

    download_file(MODEL_URL, &zip_path)?;
    extract_zip(&zip_path, models_dir)?;
    fs::remove_file(&zip_path)?;

    Ok(())
}

fn download_file(url: &str, dest_path: &Path) -> BuildResult<()> {
    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(600))
        .call()?;

    let total_size = response
        .header("content-length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = response.into_reader();
    let mut file = fs::File::create(dest_path)?;
    let mut buffer = [0u8; 65536];
    let mut downloaded = 0u64;
    let mut last_progress = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        if total_size > 0 {
            let progress = (downloaded * 100) / total_size;
            if progress >= last_progress + 20 {
                println!("cargo:warning=Progress: {}%", progress);
                last_progress = progress;
            }
        }
    }

    Ok(())
}

fn extract_zip(zip_path: &Path, dest_dir: &Path) -> BuildResult<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let len = archive.len();

    for i in 0..len {
        let mut file = archive.by_index(i)?;
        let outpath = dest_dir.join(file.mangled_name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}

fn extract_library_files(zip_path: &Path, dest_dir: &Path) -> BuildResult<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let len = archive.len();

    for i in 0..len {
        let mut file = archive.by_index(i)?;
        let file_name = file.name();

        if file_name.ends_with('/') {
            continue;
        }

        let is_library = file_name.ends_with(".so")
            || file_name.ends_with(".dll")
            || file_name.ends_with(".dylib")
            || file_name.ends_with(".lib");

        if is_library {
            let filename = Path::new(file_name).file_name().ok_or("Invalid filename")?;

            let outpath = dest_dir.join(filename);
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }

    Ok(())
}
