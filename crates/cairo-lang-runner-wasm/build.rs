use std::path::{Path, PathBuf};
use std::{env, fs, io};

fn main() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let corelib_root = manifest_dir.join("../../corelib/src").canonicalize()?;

    println!("cargo:rerun-if-changed={}", corelib_root.display());

    let mut files = Vec::new();
    collect_files(&corelib_root, &corelib_root, &mut files)?;
    files.sort();

    let mut generated = String::new();
    generated.push_str("pub(crate) static EMBEDDED_CORELIB_FILES: &[(&str, &str)] = &[\n");

    for rel_path in files {
        let abs_path = corelib_root.join(&rel_path).canonicalize()?;
        let rel_path_str =
            rel_path.iter().map(|part| part.to_string_lossy()).collect::<Vec<_>>().join("/");
        let abs_path_str = abs_path.to_string_lossy().replace('\\', "\\\\");
        generated.push_str(&format!(
            "    (\"{}\", include_str!(\"{}\")),\n",
            rel_path_str, abs_path_str
        ));
    }

    generated.push_str("];\n");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("embedded_corelib.rs"), generated)?;
    Ok(())
}

fn collect_files(root: &Path, current: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_files(root, &path, out)?;
            continue;
        }
        if file_type.is_file() && path.extension().is_some_and(|ext| ext == "cairo") {
            out.push(path.strip_prefix(root).expect("strip_prefix").to_path_buf());
        }
    }
    Ok(())
}
