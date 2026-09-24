//! Recherche au fil de la frappe : correspondance floue (nucleo) sur le sujet, l'auteur et les
//! refs, préfixe de sha, et opérateurs `author:`, `msg:`, `ref:`, `sha:`.

use std::sync::Mutex;
use std::time::Instant;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str, Utf32String};
use rayon::prelude::*;

use crate::graph::Layout;
use crate::repo::{CommitInfo, CommitKind, RefInfo};

/// Données indexées par ligne du graph.
pub struct SearchIndex {
    subject: Vec<Utf32String>,
    author: Vec<String>,
    refs: Vec<String>,
    sha: Vec<String>,
    /// Sujet + auteur + refs : cible de la recherche libre.
    all: Vec<Utf32String>,
    /// Dernière requête et ses résultats, pour filtrer au fil de la frappe.
    last: Mutex<Option<(String, bool, Vec<u32>)>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub total: u32,
    /// Lignes correspondantes, dans l'ordre du graph.
    pub rows: Vec<u32>,
    pub elapsed_ms: f64,
}

#[derive(Default, Debug, PartialEq)]
struct Query {
    free: String,
    author: Vec<String>,
    msg: Vec<String>,
    refs: Vec<String>,
    sha: Vec<String>,
}

fn parse(q: &str) -> Query {
    let mut out = Query::default();
    let mut free = Vec::new();
    for tok in q.split_whitespace() {
        let lower = tok.to_lowercase();
        if let Some(v) = lower.strip_prefix("author:") {
            out.author.push(v.to_string());
        } else if let Some(v) = tok.strip_prefix("msg:") {
            out.msg.push(v.to_string());
        } else if let Some(v) = lower.strip_prefix("ref:") {
            out.refs.push(v.to_string());
        } else if let Some(v) = lower.strip_prefix("sha:") {
            out.sha.push(v.to_string());
        } else {
            free.push(tok);
        }
    }
    out.free = free.join(" ");
    out
}

thread_local! {
    /// Un matcher par thread : son allocation est coûteuse, on la fait une seule fois.
    static MATCHER: std::cell::RefCell<Matcher> = std::cell::RefCell::new(Matcher::new(Config::DEFAULT));
}

/// Requête analysée et motifs compilés une fois par frappe.
struct Compiled {
    query: Query,
    free: Option<Pattern>,
    msg: Option<Pattern>,
    /// Un mot libre qui ressemble à un sha (hexa, ≥ 4 caractères) matche aussi par préfixe.
    free_sha: Option<String>,
}

impl Compiled {
    fn new(query: &str) -> Option<Compiled> {
        let q = parse(query);
        if q.free.is_empty() && q.author.is_empty() && q.msg.is_empty() && q.refs.is_empty() && q.sha.is_empty() {
            return None;
        }
        let free = (!q.free.is_empty()).then(|| Pattern::parse(&q.free, CaseMatching::Smart, Normalization::Smart));
        let msg = (!q.msg.is_empty()).then(|| Pattern::parse(&q.msg.join(" "), CaseMatching::Smart, Normalization::Smart));
        let free_sha =
            (q.free.len() >= 4 && !q.free.contains(' ') && q.free.chars().all(|c| c.is_ascii_hexdigit())).then(|| q.free.to_lowercase());
        Some(Compiled {
            query: q,
            free,
            msg,
            free_sha,
        })
    }
}

impl SearchIndex {
    pub fn build(layout: &Layout, commits: &[CommitInfo], refs: &[RefInfo]) -> SearchIndex {
        let rows = layout.order.len();
        let mut refs_by_row = vec![String::new(); rows];
        for r in refs {
            let s = &mut refs_by_row[r.row as usize];
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&r.name.to_lowercase());
        }
        let per_row: Vec<(Utf32String, String, String, Utf32String)> = layout
            .order
            .par_iter()
            .enumerate()
            .map(|(row, &i)| {
                let c = &commits[i as usize];
                let author = format!("{} {}", c.author, c.email).to_lowercase();
                let sha = if c.kind == CommitKind::Wip {
                    String::new()
                } else {
                    c.id.to_hex().to_string()
                };
                let all = format!("{} {} {}", c.summary, c.author, refs_by_row[row]);
                (Utf32String::from(c.summary.as_str()), author, sha, Utf32String::from(all.as_str()))
            })
            .collect();
        let mut idx = SearchIndex {
            subject: Vec::with_capacity(rows),
            author: Vec::with_capacity(rows),
            refs: refs_by_row,
            sha: Vec::with_capacity(rows),
            all: Vec::with_capacity(rows),
            last: Mutex::new(None),
        };
        for (s, a, h, all) in per_row {
            idx.subject.push(s);
            idx.author.push(a);
            idx.sha.push(h);
            idx.all.push(all);
        }
        idx
    }

    pub fn search(&self, query: &str) -> SearchResult {
        let t = Instant::now();
        let Some(m) = Compiled::new(query) else {
            *self.last.lock().unwrap() = None;
            return SearchResult {
                total: 0,
                rows: Vec::new(),
                elapsed_ms: 0.0,
            };
        };
        // Une requête qui prolonge la précédente ne peut que restreindre ses résultats : on ne
        // re-teste que ceux-là (sauf si la détection de sha change, car elle élargit).
        let previous = self.last.lock().unwrap().take();
        let candidates: Option<Vec<u32>> =
            previous.and_then(|(q, sha, rows)| (query.starts_with(q.as_str()) && sha == m.free_sha.is_some()).then_some(rows));
        let test = |row: u32| {
            MATCHER
                .with_borrow_mut(|matcher| self.matches(&m, row as usize, matcher))
                .then_some(row)
        };
        let rows: Vec<u32> = match candidates {
            Some(c) => c.into_par_iter().with_min_len(4096).filter_map(test).collect(),
            None => (0..self.subject.len() as u32)
                .into_par_iter()
                .with_min_len(4096)
                .filter_map(test)
                .collect(),
        };
        *self.last.lock().unwrap() = Some((query.to_string(), m.free_sha.is_some(), rows.clone()));
        SearchResult {
            total: rows.len() as u32,
            rows,
            elapsed_ms: t.elapsed().as_secs_f64() * 1000.0,
        }
    }

    /// Positions à surligner dans le sujet, pour les seules lignes affichées.
    pub fn highlights(&self, query: &str, rows: &[u32]) -> Vec<Vec<u32>> {
        let Some(m) = Compiled::new(query) else {
            return vec![Vec::new(); rows.len()];
        };
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut buf = Vec::new();
        rows.iter()
            .map(|&row| {
                let row = row as usize;
                if row >= self.subject.len() {
                    return Vec::new();
                }
                let mut out = Vec::new();
                for p in [&m.free, &m.msg].into_iter().flatten() {
                    buf.clear();
                    if p.indices(self.subject[row].slice(..), &mut matcher, &mut buf).is_some() {
                        out.extend(buf.iter().copied());
                    }
                }
                out.sort_unstable();
                out.dedup();
                out
            })
            .collect()
    }

    fn matches(&self, m: &Compiled, row: usize, matcher: &mut Matcher) -> bool {
        let q = &m.query;
        if !q.author.iter().all(|a| self.author[row].contains(a.as_str())) {
            return false;
        }
        if !q.refs.iter().all(|r| self.refs[row].contains(r.as_str())) {
            return false;
        }
        if !q
            .sha
            .iter()
            .all(|s| !self.sha[row].is_empty() && self.sha[row].starts_with(s.as_str()))
        {
            return false;
        }
        if let Some(p) = &m.msg
            && p.score(self.subject[row].slice(..), matcher).is_none()
        {
            return false;
        }
        if let Some(p) = &m.free {
            let sha_hit = m.free_sha.as_ref().is_some_and(|s| self.sha[row].starts_with(s.as_str()));
            if !sha_hit && p.score(self.all[row].slice(..), matcher).is_none() {
                return false;
            }
        }
        true
    }

    /// Sujet d'une ligne tel qu'indexé (utile aux tests).
    pub fn subject(&self, row: u32) -> String {
        match self.subject[row as usize].slice(..) {
            Utf32Str::Ascii(b) => String::from_utf8_lossy(b).into_owned(),
            Utf32Str::Unicode(c) => c.iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(subjects: &[&str]) -> SearchIndex {
        SearchIndex {
            subject: subjects.iter().map(|s| Utf32String::from(*s)).collect(),
            author: subjects.iter().map(|_| String::from("léa lea@example.com")).collect(),
            refs: subjects.iter().map(|_| String::new()).collect(),
            sha: (0..subjects.len()).map(|i| format!("{i:04x}abcdef")).collect(),
            all: subjects.iter().map(|s| Utf32String::from(*s)).collect(),
            last: Mutex::new(None),
        }
    }

    #[test]
    fn incremental_typing_gives_same_results_as_fresh_search() {
        let subjects = [
            "fix(pwa): invalide le cache",
            "feat: pwa hors ligne",
            "chore: cache npm",
            "docs: README",
            "perf: cache des tâches",
        ];
        let idx = index(&subjects);
        let mut typed = String::new();
        for ch in "pwa cache".chars() {
            typed.push(ch);
            let incremental = idx.search(&typed).rows;
            let fresh = index(&subjects).search(&typed).rows;
            assert_eq!(incremental, fresh, "requête {typed:?}");
        }
        // Effacer puis taper autre chose repart de zéro.
        assert_eq!(idx.search("readme").rows, vec![3]);
    }

    #[test]
    fn sha_detection_widens_and_is_not_narrowed() {
        let idx = index(&["alpha", "beta"]);
        assert!(idx.search("000").rows.is_empty());
        assert_eq!(idx.search("0001").rows, vec![1]);
    }

    #[test]
    fn parses_operators() {
        let q = parse("author:Julien pwa cache ref:feature/ sha:4F2a");
        assert_eq!(q.free, "pwa cache");
        assert_eq!(q.author, vec!["julien"]);
        assert_eq!(q.refs, vec!["feature/"]);
        assert_eq!(q.sha, vec!["4f2a"]);
    }
}
