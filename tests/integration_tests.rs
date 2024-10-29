#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, process::Command};
    use tempdir::TempDir;

    #[test]
    fn warn_dirty() {
        let test_dir =
            TempDir::new("test_warn_dirty").unwrap_or_else(|e| panic!("TempDir::new failed: {e}"));
        let src = "tests/testdata/Windows.edb";
        let dst = test_dir.path().join("Windows.edb");
        fs::copy(src, &dst)
            .unwrap_or_else(|e| panic!("Could not copy '{src}' to '{}': {e}", dst.display()));

        let bin_root = PathBuf::from("target").join(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        });
        let sidr_bin = bin_root.join("sidr");
        let work_dir = test_dir.path().display().to_string();
        let output = Command::new(sidr_bin)
            .args(["--outdir", &work_dir, &work_dir])
            .output()
            .expect("failed to execute process");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr)
            .contains("WARNING: The database state is not clean."));
        assert!(test_dir.close().is_ok());
    }
}
