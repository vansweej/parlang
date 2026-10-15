use parlang::parse_program;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

static BAT_CACHE: OnceLock<Result<PathBuf, String>> = OnceLock::new();

fn bat_dir() -> &'static Path {
    let result = BAT_CACHE.get_or_init(|| {
        let dir = env::temp_dir().join(format!("parlang-bat-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        let syntaxes_dir = dir.join("syntaxes");
        let themes_dir = dir.join("themes");
        fs::create_dir_all(&syntaxes_dir).map_err(|e| e.to_string())?;
        fs::create_dir_all(&themes_dir).map_err(|e| e.to_string())?;

        let root = workspace_root();
        fs::copy(
            root.join("editors/sublime/ParLang.sublime-syntax"),
            syntaxes_dir.join("ParLang.sublime-syntax"),
        )
        .map_err(|e| e.to_string())?;
        fs::copy(
            root.join("editors/sublime/test/ParLangScopes.tmTheme"),
            themes_dir.join("ParLangScopes.tmTheme"),
        )
        .map_err(|e| e.to_string())?;

        let target = dir.join("bat-cache");
        let output = Command::new("bat")
            .arg("cache")
            .arg("--build")
            .arg("--blank")
            .arg("--source")
            .arg(&dir)
            .arg("--target")
            .arg(&target)
            .current_dir(&dir)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
            combined.push_str(&String::from_utf8_lossy(&output.stderr));
            return Err(combined);
        }

        Ok(dir)
    });

    match result {
        Ok(path) => path.as_path(),
        Err(error) => panic!("failed to build bat cache: {error}"),
    }
}

fn bat_render(file: &Path) -> std::process::Output {
    let dir = bat_dir();
    let target = dir.join("bat-cache");
    Command::new("bat")
        .arg("--no-config")
        .arg("--color=always")
        .arg("--style=plain")
        .arg("--paging=never")
        .arg("--theme=ParLangScopes")
        .arg(file)
        .current_dir(dir)
        .env("BAT_CACHE_PATH", &target)
        .env("COLORTERM", "truecolor")
        .output()
        .expect("bat command should run")
}

fn color_runs(ansi: &str) -> Vec<(Option<String>, String)> {
    let mut runs: Vec<(Option<String>, String)> = Vec::new();
    let mut current_color: Option<String> = None;
    let mut current_text = String::new();
    let mut chars = ansi.chars().peekable();

    let flush = |runs: &mut Vec<(Option<String>, String)>,
                 current_color: &Option<String>,
                 current_text: &mut String| {
        if !current_text.is_empty() {
            runs.push((current_color.clone(), std::mem::take(current_text)));
        }
    };

    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            let mut params = String::new();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
                params.push(next);
            }
            flush(&mut runs, &current_color, &mut current_text);
            if params.is_empty() || params == "0" {
                current_color = None;
            } else {
                let parts: Vec<&str> = params.split(';').collect();
                if parts.len() == 5 && parts[0] == "38" && parts[1] == "2" {
                    let r = parts[2].parse::<u8>().ok();
                    let g = parts[3].parse::<u8>().ok();
                    let b = parts[4].parse::<u8>().ok();
                    if let (Some(r), Some(g), Some(b)) = (r, g, b) {
                        current_color = Some(format!("#{r:02X}{g:02X}{b:02X}"));
                    }
                }
                // other params leave color unchanged
            }
        } else if c == '\n' {
            flush(&mut runs, &current_color, &mut current_text);
            current_text.push('\n');
            flush(&mut runs, &current_color, &mut current_text);
        } else {
            current_text.push(c);
        }
    }
    flush(&mut runs, &current_color, &mut current_text);

    runs
}

fn colour_of(runs: &[(Option<String>, String)], needle: &str) -> Option<String> {
    runs.iter()
        .find(|(_, text)| text.contains(needle))
        .and_then(|(colour, _)| colour.clone())
}

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
            } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("par") {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

fn query_captures(file: &Path) -> Vec<(String, String)> {
    let output = ts_command(&[
        std::ffi::OsStr::new("query"),
        std::ffi::OsStr::new("queries/highlights.scm"),
        file.as_os_str(),
    ]);

    if !output.status.success() {
        let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
        panic!("tree-sitter query failed:\n{combined}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pairs = Vec::new();

    for line in stdout.lines() {
        let Some(capture_part) = line.split("capture: ").nth(1) else {
            continue;
        };
        // Strip an optional leading "<digits> - " prefix.
        let capture_part = capture_part.trim_start();
        let after_index = if let Some(dash_pos) = capture_part.find(" - ") {
            let (prefix, rest) = capture_part.split_at(dash_pos);
            if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
                &rest[3..]
            } else {
                capture_part
            }
        } else {
            capture_part
        };

        let name = after_index
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        let Some(text_start) = line.find("text: `") else {
            continue;
        };
        let after_text = &line[text_start + "text: `".len()..];
        let Some(last_backtick) = after_text.rfind('`') else {
            continue;
        };
        let text = after_text[..last_backtick].to_string();

        pairs.push((text, name));
    }

    assert!(
        !pairs.is_empty(),
        "expected at least one capture from tree-sitter query, got none. stdout was:\n{stdout}"
    );

    pairs
}

#[test]
fn tree_sitter_highlight_captures() {
    require_cli("tree-sitter");
    require_cli("node");

    let sample = workspace_root().join("editors/samples/highlight_sample.par");
    let captures = query_captures(&sample);

    let expected: Vec<(&str, &str)> = vec![
        ("data", "keyword"),
        ("match", "keyword.conditional"),
        ("rec", "keyword.function"),
        ("Circle", "constructor"),
        ("Shape", "type.definition"),
        ("fact", "function"),
        (
            "-- | Highlight sample for the ParLang editor grammars.",
            "comment.documentation",
        ),
        ("-- A plain line comment.", "comment"),
        ("\\n", "string.escape"),
        ("-7", "number"),
        ("2.5", "number.float"),
        ("x", "property"),
        ("true", "boolean"),
        ("<=", "operator"),
        ("step", "variable.parameter"),
    ];

    let mut missing = Vec::new();
    for (text, name) in &expected {
        let found = captures
            .iter()
            .any(|(ctext, cname)| ctext == text && cname == name);
        if !found {
            missing.push(format!("({text:?}, {name:?})"));
        }
    }

    assert!(
        missing.is_empty(),
        "missing expected captures: {}\nall captures: {:?}",
        missing.join(", "),
        captures
    );
}

#[test]
fn tree_sitter_keyword_sync() {
    require_cli("tree-sitter");
    require_cli("node");

    let mut content = String::new();
    for keyword in parlang::parser::KEYWORDS {
        let line = match *keyword {
            "let" => "let x".to_string(),
            "fun" => "fun x".to_string(),
            "rec" => "rec x".to_string(),
            "data" => "data T =".to_string(),
            "type" => "type T".to_string(),
            other => other.to_string(),
        };
        content.push_str(&line);
        content.push('\n');
    }

    let root = ts_workspace();
    let keywords_file = root.join("keywords.par");
    fs::write(&keywords_file, &content).expect("failed to write keywords.par");

    let captures = query_captures(&keywords_file);

    let mut mismatches = Vec::new();
    for keyword in parlang::parser::KEYWORDS {
        let found = captures.iter().find(|(text, _)| text == keyword);
        match found {
            Some((_, name)) => {
                let ok = if *keyword == "true" || *keyword == "false" {
                    name == "boolean"
                } else {
                    name.starts_with("keyword")
                };
                if !ok {
                    mismatches.push(format!("{keyword:?}: got capture {name:?}"));
                }
            }
            None => {
                mismatches.push(format!("{keyword:?}: no capture found"));
            }
        }
    }

    assert!(
        mismatches.is_empty(),
        "keyword capture mismatches: {}",
        mismatches.join("; ")
    );
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
fn sublime_bat_colors() {
    require_cli("bat");

    let sample = workspace_root().join("editors/samples/highlight_sample.par");
    let output = bat_render(&sample);
    assert!(
        output.status.success(),
        "bat render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let runs = color_runs(&stdout);

    let expectations: Vec<(&str, &str)> = vec![
        ("data", "#FF0101"),
        ("true", "#FF0202"),
        ("2.5", "#FF0303"),
        ("-7", "#FF0303"),
        ("\\t", "#FF0505"),
        ("-- | Highlight", "#FF0707"),
        ("-- A plain", "#FF0606"),
        ("inner", "#FF0606"),
        ("Circle", "#FF0808"),
        ("<=", "#FF0909"),
    ];

    let mut mismatches = Vec::new();
    for (needle, expected) in &expectations {
        let found = colour_of(&runs, needle);
        if found.as_deref() != Some(*expected) {
            mismatches.push(format!("{needle:?}: expected {expected:?}, got {found:?}"));
        }
    }

    assert!(
        mismatches.is_empty(),
        "bat colour mismatches: {}",
        mismatches.join("; ")
    );
}

#[test]
fn sublime_keyword_sync() {
    require_cli("bat");

    let dir = bat_dir();
    let keywords_file = dir.join("keywords.par");
    let mut content = String::new();
    for keyword in parlang::parser::KEYWORDS {
        content.push_str(keyword);
        content.push('\n');
    }
    fs::write(&keywords_file, &content).expect("failed to write keywords.par");

    let output = bat_render(&keywords_file);
    assert!(
        output.status.success(),
        "bat render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let runs = color_runs(&stdout);

    let mut mismatches = Vec::new();
    for keyword in parlang::parser::KEYWORDS {
        let found = runs
            .iter()
            .find(|(_, text)| text.trim() == *keyword)
            .map(|(colour, _)| colour.clone());
        let expected = if *keyword == "true" || *keyword == "false" {
            "#FF0202"
        } else {
            "#FF0101"
        };
        match found {
            Some(Some(colour)) if colour == expected => {}
            other => mismatches.push(format!("{keyword:?}: got {other:?}, expected {expected:?}")),
        }
    }

    assert!(
        mismatches.is_empty(),
        "keyword colour mismatches: {}",
        mismatches.join("; ")
    );
}

#[test]
fn sublime_bat_corpus_runs() {
    require_cli("bat");

    let root = workspace_root();
    let files = collect_par_files(&root);

    let mut failures = Vec::new();
    for path in &files {
        let output = bat_render(path);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !output.status.success() || !stderr.is_empty() {
            failures.push(format!(
                "{}: status={:?} stderr={}",
                path.display(),
                output.status,
                stderr
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "bat failed to render some corpus files:\n{}",
        failures.join("\n")
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
