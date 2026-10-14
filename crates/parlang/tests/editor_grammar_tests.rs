use parlang::parse_program;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn require_cli(name: &str) {
    let result = Command::new(name).arg("--version").output();
    let ok = matches!(result, Ok(output) if output.status.success());
    assert!(
        ok,
        "required tool '{name}' is not available or failed to run; run these tests inside \
         `nix develop`"
    );
}

static TS_WORKSPACE: OnceLock<Result<PathBuf, String>> = OnceLock::new();

fn ts_workspace() -> &'static Path {
    let result = TS_WORKSPACE.get_or_init(|| {
        let root = env::temp_dir().join(format!("parlang-ts-{}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
        }
        let grammar_dir = root.join("tree-sitter-parlang");
        let src_dir = grammar_dir.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| e.to_string())?;

        let source_root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../editors/tree-sitter-parlang");
        fs::copy(
            source_root.join("grammar.js"),
            grammar_dir.join("grammar.js"),
        )
        .map_err(|e| e.to_string())?;
        fs::copy(
            source_root.join("tree-sitter.json"),
            grammar_dir.join("tree-sitter.json"),
        )
        .map_err(|e| e.to_string())?;
        fs::copy(source_root.join("src/scanner.c"), src_dir.join("scanner.c"))
            .map_err(|e| e.to_string())?;

        let highlights_source = source_root.join("queries/highlights.scm");
        if highlights_source.exists() {
            let queries_dir = grammar_dir.join("queries");
            fs::create_dir_all(&queries_dir).map_err(|e| e.to_string())?;
            fs::copy(&highlights_source, queries_dir.join("highlights.scm"))
                .map_err(|e| e.to_string())?;
        }

        let config_path = root.join("config.json");
        fs::write(&config_path, r#"{"parser-directories": []}"#).map_err(|e| e.to_string())?;

        let libdir = root.join("lib");
        fs::create_dir_all(&libdir).map_err(|e| e.to_string())?;

        let generate_output = Command::new("tree-sitter")
            .arg("generate")
            .current_dir(&grammar_dir)
            .env("NO_COLOR", "1")
            .env("TREE_SITTER_LIBDIR", &libdir)
            .output()
            .map_err(|e| e.to_string())?;

        if !generate_output.status.success() {
            let mut combined = String::from_utf8_lossy(&generate_output.stdout).into_owned();
            combined.push_str(&String::from_utf8_lossy(&generate_output.stderr));
            return Err(combined);
        }

        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/simple.par");
        let _ = Command::new("tree-sitter")
            .arg("parse")
            .arg("--config-path")
            .arg(&config_path)
            .arg(&sample)
            .current_dir(&grammar_dir)
            .env("NO_COLOR", "1")
            .env("TREE_SITTER_LIBDIR", &libdir)
            .output();

        Ok(root)
    });

    match result {
        Ok(path) => path.as_path(),
        Err(error) => panic!("failed to initialize tree-sitter workspace: {error}"),
    }
}

fn ts_command<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> std::process::Output {
    let root = ts_workspace();
    let grammar_dir = root.join("tree-sitter-parlang");
    let config_path = root.join("config.json");
    let libdir = root.join("lib");

    let mut full_args: Vec<std::ffi::OsString> = Vec::new();
    if let Some(first) = args.first() {
        full_args.push(first.as_ref().to_os_string());
        let subcommand = first.as_ref().to_string_lossy();
        if subcommand == "parse" || subcommand == "query" {
            full_args.push("--config-path".into());
            full_args.push(config_path.clone().into());
        }
        for arg in &args[1..] {
            full_args.push(arg.as_ref().to_os_string());
        }
    }

    Command::new("tree-sitter")
        .args(&full_args)
        .current_dir(&grammar_dir)
        .env("NO_COLOR", "1")
        .env("TREE_SITTER_LIBDIR", &libdir)
        .output()
        .expect("tree-sitter command should run")
}

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
fn tree_sitter_generates_and_parses_corpus() {
    require_cli("tree-sitter");
    require_cli("node");

    let root = workspace_root();
    let files = collect_par_files(&root);

    let mut failures = Vec::new();
    for path in &files {
        let output = ts_command(&[std::ffi::OsStr::new("parse"), path.as_os_str()]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let has_error = stdout.contains("(ERROR") || stdout.contains("(MISSING");
        if !output.status.success() || has_error {
            let combined: String = stdout.lines().take(20).collect::<Vec<_>>().join("\n");
            failures.push(format!("{}:\n{}", path.display(), combined));
        }
    }

    assert!(
        failures.is_empty(),
        "some corpus files failed tree-sitter parsing:\n{}",
        failures.join("\n\n")
    );
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
