//! Exporte un dépôt au format attendu par le backend de démonstration du front (mode
//! navigateur, tests) : `cargo run --release -p ramure-core --example dump -- <dépôt> > public/sample.json`

use std::collections::BTreeMap;

use ramure_core::view::{self, Details};

fn main() {
    let path = std::env::args().nth(1).expect("usage : dump <dépôt>");
    let repo = ramure_core::Repo::open(&path).expect("ouverture");
    let n = repo.len() as u32;
    let rows = view::rows(&repo, 0, n);
    let edges = view::edges(&repo, 0, n);
    // Détails et diffs des 40 premières lignes seulement (l'échantillon reste léger).
    let mut details = BTreeMap::new();
    let mut diffs = BTreeMap::new();
    for row in 0..n.min(40) {
        let Ok(d) = view::details(&repo, row) else { continue };
        let files = match &d {
            Details::Commit(c) => c.files.clone(),
            Details::Wip { files, .. } => files.clone(),
        };
        for f in files.iter().take(6) {
            if let Ok(diff) = view::file_diff(&repo, row, &f.path, f.old_path.as_deref(), f.status == "?") {
                diffs.insert(format!("{row}:{}", f.path), diff);
            }
        }
        details.insert(row.to_string(), d);
    }
    let out = serde_json::json!({
        "summary": view::summary(&repo),
        "rows": rows,
        "edges": edges,
        "details": details,
        "diffs": diffs,
    });
    println!("{}", serde_json::to_string(&out).unwrap());
}
