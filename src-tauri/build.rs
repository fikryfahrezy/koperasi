use std::{env, fmt::Write, fs, path::Path};

fn main() {
    embed_sql_dir("migrations");
    embed_sql_dir("data");
    tauri_build::build()
}

/// Embeds every `<dir>/*.sql` file, sorted by name, into the binary as
/// `$OUT_DIR/<dir>.rs`, so installed apps apply them without the files being
/// shipped alongside the executable.
fn embed_sql_dir(dir: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    println!("cargo:rerun-if-changed={}", path.display());
    let mut scripts: Vec<_> = fs::read_dir(&path)
        .unwrap_or_else(|error| panic!("src-tauri/{dir} must exist: {error}"))
        .map(|entry| entry.expect("readable SQL script").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    scripts.sort();
    let mut entries = String::new();
    for path in &scripts {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path.file_stem().unwrap().to_string_lossy();
        let file = path.display().to_string();
        writeln!(entries, "    ({name:?}, include_str!({file:?})),").unwrap();
    }
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join(format!("{dir}.rs"));
    fs::write(out, format!("&[\n{entries}]\n")).expect("write embedded SQL list");
}
