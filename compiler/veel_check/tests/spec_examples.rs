//! Every unfenced Veel example of the spec checks clean (board row B-40): each ```` ```veel ````
//! block of `docs/spec/veel-v0.md` (not ```` ```veel (<task>) ````, a form a later task builds),
//! the blocks under one heading read as one project, each file named by its first line. Its
//! control plants the section 8 fragment that B-40 found, and the check must fail on it.

use std::path::Path;

use veel_check::check;

fn spec() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/veel-v0.md"),
    )
    .unwrap()
}

/// The unfenced Veel blocks of `markdown`, grouped by the heading above them: each group the
/// heading, and each block's file name (from its first line, `// path`) and text.
fn projects(markdown: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut heading = String::new();
    let mut block: Option<String> = None;
    let mut new_heading = true;
    // Inside another fence (```` ```text ````, ```` ```veel (<task>) ````).
    let mut other = false;
    for line in markdown.lines() {
        match &mut block {
            None if other => other = !line.starts_with("```"),
            None if line.starts_with("```") && line != "```veel" => other = true,
            None if line.starts_with('#') => {
                heading = line.to_string();
                new_heading = true;
            }
            None if line == "```veel" => block = Some(String::new()),
            Some(text) if line == "```" => {
                let text = std::mem::take(text);
                block = None;
                let name = text
                    .lines()
                    .next()
                    .and_then(|l| l.strip_prefix("// "))
                    .map(|l| l.split(':').next().unwrap().trim().to_string())
                    .unwrap_or_else(|| "example.veel".to_string());
                if new_heading {
                    out.push((heading.clone(), Vec::new()));
                    new_heading = false;
                }
                out.last_mut().unwrap().1.push((name, text));
            }
            Some(text) => {
                text.push_str(line);
                text.push('\n');
            }
            None => {}
        }
    }
    out
}

fn problems(markdown: &str) -> Vec<String> {
    projects(markdown)
        .into_iter()
        .filter_map(|(heading, files)| {
            let checked = check(&files);
            (!checked.diagnostics.is_empty()).then(|| format!("{heading}:\n{}", checked.render()))
        })
        .collect()
}

#[test]
fn every_unfenced_spec_example_checks_clean() {
    let spec = spec();
    let groups = projects(&spec);
    // Section 8's effect, and section 17's two worked examples (the second in two files).
    let sizes: Vec<usize> = groups.iter().map(|(_, f)| f.len()).collect();
    assert_eq!(
        sizes,
        [1, 1, 2],
        "{:?}",
        groups.iter().map(|g| &g.0).collect::<Vec<_>>()
    );
    let found = problems(&spec);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// The control: the fragment section 8 held before B-40, unfenced, fails the check.
#[test]
fn the_section_8_fragment_fails_the_check() {
    let fragment = "\n### A planted fragment\n\n```veel\neffect @push qtblend {\n  rect time=clip {\n    key 0  [0 0 1920 1080]\n    key 2s [-96 -54 2112 1188] ease=cubic-in\n  }\n  rotation 0deg\n}\n```\n";
    let bad = format!("{}{fragment}", spec());
    let found = problems(&bad);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("error[veel-kind]"), "{}", found[0]);
}
