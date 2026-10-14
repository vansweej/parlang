use parlang::parse_program;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_par_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if path.is_dir() {
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("par") {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

#[test]
fn corpus_files_all_parse() {
    let root = workspace_root();
    let files = collect_par_files(&root);

    assert!(
        files.len() >= 30,
        "expected at least 30 .par files in the corpus, found {}",
        files.len()
    );

    let mut failures = Vec::new();
    for path in &files {
        let contents = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        if let Err(error) = parse_program(&contents) {
            failures.push(format!("{}: {error}", path.display()));
        }
    }

    assert!(
        failures.is_empty(),
        "some corpus files failed to parse:\n{}",
        failures.join("\n")
    );
}
