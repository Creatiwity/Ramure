//! Mesures de performance : `cargo run --release -p ramure-core --example bench -- <dépôt>`

use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("usage : bench <dépôt>");
    for run in ["à froid", "à chaud"] {
        let t = Instant::now();
        let repo = ramure_core::Repo::open(&path).expect("ouverture");
        let total = t.elapsed().as_secs_f64() * 1000.0;
        println!(
            "Ouverture ({run}) : {} lignes, {} lanes max, {:.0} ms",
            repo.len(),
            repo.layout.max_lanes,
            total
        );
        println!("  {:?}", repo.timings);
        if run == "à chaud" {
            let mut worst = 0f64;
            for q in [
                "p",
                "pw",
                "pwa",
                "pwa c",
                "pwa ca",
                "pwa cac",
                "pwa cache",
                "author:léa",
                "sha:4f2",
                "pndr retrd",
            ] {
                let r = repo.search.search(q);
                worst = worst.max(r.elapsed_ms);
                println!("  recherche {q:>12} : {:>6} résultats en {:>6.2} ms", r.total, r.elapsed_ms);
                let t = Instant::now();
                let visible: Vec<u32> = r.rows.iter().take(60).copied().collect();
                let _ = repo.search.highlights(q, &visible);
                worst = worst.max(r.elapsed_ms + t.elapsed().as_secs_f64() * 1000.0);
            }
            println!("  pire frappe : {worst:.2} ms");
            let t = Instant::now();
            let e = repo.edges(50_000.min(repo.len() as u32 / 2), 50_000.min(repo.len() as u32 / 2) + 200);
            println!(
                "  arêtes d'une fenêtre de 200 lignes : {} en {:.3} ms",
                e.len(),
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
}
