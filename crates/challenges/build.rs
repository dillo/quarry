//! Embeds every track folder under `content/tracks/` in the binary.
//!
//! Each folder becomes one `RawTrack` of `include_str!`s, in folder-name
//! order, so adding a track needs no code. A folder missing one of its four
//! files fails the build, naming the file.

use std::{
    env,
    error::Error,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

/// The files every track folder holds, as `RawTrack` field and file name.
const FILES: [(&str, &str); 4] = [
    ("track", "track.toml"),
    ("schema", "schema.sql"),
    ("seed", "seed.sql"),
    ("challenges", "challenges.toml"),
];

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("../../content/tracks");
    println!("cargo::rerun-if-changed={}", root.display());
    let mut code = String::from("&[\n");
    for folder in track_folders(&root)? {
        println!("cargo::rerun-if-changed={}", folder.display());
        code.push_str(&raw_track(&folder)?);
    }
    code.push(']');
    let out = Path::new(&env::var("OUT_DIR")?).join("tracks.rs");
    fs::write(out, code)?;
    Ok(())
}

/// The track folders, sorted by name; none when `root` is missing.
fn track_folders(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut folders = Vec::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            folders.push(path);
        }
    }
    folders.sort();
    Ok(folders)
}

/// One `RawTrack { .. }` expression for `folder`.
fn raw_track(folder: &Path) -> Result<String, Box<dyn Error>> {
    let slug = folder
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("a track folder's name is not UTF-8")?;
    let mut code = format!("    RawTrack {{\n        slug: {slug:?},\n");
    for (field, file) in FILES {
        let path = folder.join(file);
        println!("cargo::rerun-if-changed={}", path.display());
        if !path.is_file() {
            return Err(format!("content/tracks/{slug}/{file} is missing").into());
        }
        let path = path.canonicalize()?;
        let path = path.to_str().ok_or("a track file's path is not UTF-8")?;
        writeln!(code, "        {field}: include_str!({path:?}),")?;
    }
    code.push_str("    },\n");
    Ok(code)
}
