//! Ordre d'affichage et attribution des lanes du commit graph.
//!
//! - Ordre : équivalent de `git log --date-order` (aucun parent avant tous ses enfants,
//!   sinon du plus récent au plus ancien).
//! - Lanes : algorithme « straight branches » (tous les commits d'une branche sur la même
//!   colonne), avec des branches de tronc épinglées à gauche (`ux-graph.md`, D3).
//! - Couleurs : une *chaîne* par branche logique ; la couleur suit la chaîne, pas la colonne.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Nœud d'entrée : un commit, ses parents (indices dans la même liste) et sa date.
#[derive(Debug, Clone)]
pub struct Node {
    pub parents: Vec<u32>,
    pub time: i64,
}

/// Arête dessinée de l'enfant (`from_row`) vers le parent (`to_row`).
///
/// Le tracé part de `from_lane`, rejoint `via_lane` dès la ligne suivante si besoin, descend
/// dans `via_lane`, puis bascule vers `to_lane` juste avant le parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Edge {
    pub from_row: u32,
    pub to_row: u32,
    pub from_lane: u16,
    pub via_lane: u16,
    pub to_lane: u16,
    /// Chaîne (branche logique) dont le trait porte la couleur.
    pub chain: u32,
}

#[derive(Debug, Clone)]
pub struct Layout {
    /// `order[row]` = indice du nœud affiché à cette ligne.
    pub order: Vec<u32>,
    /// `row_of[node]` = ligne du nœud.
    pub row_of: Vec<u32>,
    /// Lane et chaîne de chaque ligne.
    pub lane: Vec<u16>,
    pub chain: Vec<u32>,
    /// Arêtes triées par `from_row`.
    pub edges: Vec<Edge>,
    /// Nombre de lanes utilisées au maximum (largeur constante du graph, D2).
    pub max_lanes: u16,
    /// Premier commit (tête) de chaque chaîne, en indice de nœud.
    pub chain_tip: Vec<u32>,
}

/// Tri « date-order » : enfants avant parents, sinon plus récent d'abord.
/// À date égale, l'ordre d'entrée départage (stable).
pub fn date_order(nodes: &[Node]) -> Vec<u32> {
    let n = nodes.len();
    let mut children = vec![0u32; n];
    for node in nodes {
        for &p in &node.parents {
            children[p as usize] += 1;
        }
    }
    let mut heap: BinaryHeap<(i64, Reverse<u32>)> = BinaryHeap::with_capacity(n.min(1 << 16));
    for (i, c) in children.iter().enumerate() {
        if *c == 0 {
            heap.push((nodes[i].time, Reverse(i as u32)));
        }
    }
    let mut order = Vec::with_capacity(n);
    while let Some((_, Reverse(i))) = heap.pop() {
        order.push(i);
        for &p in &nodes[i as usize].parents {
            let c = &mut children[p as usize];
            *c -= 1;
            if *c == 0 {
                heap.push((nodes[p as usize].time, Reverse(p)));
            }
        }
    }
    debug_assert_eq!(order.len(), n, "l'historique contient un cycle");
    order
}

/// Calcule l'ordre, les lanes et les arêtes.
///
/// `trunks` : pour chaque branche de tronc, par priorité (`main` puis `develop`…), l'indice du
/// nœud de tête. Sa chaîne de premiers parents est épinglée à la lane de même rang.
pub fn layout(nodes: &[Node], trunks: &[u32]) -> Layout {
    let n = nodes.len();
    let order = date_order(nodes);
    let mut row_of = vec![0u32; n];
    for (row, &i) in order.iter().enumerate() {
        row_of[i as usize] = row as u32;
    }

    // Lane épinglée pour les commits des troncs (premiers parents depuis chaque tête).
    const NONE: u16 = u16::MAX;
    let mut pinned = vec![NONE; n];
    for (rank, &tip) in trunks.iter().enumerate() {
        let mut cur = Some(tip);
        while let Some(c) = cur {
            if pinned[c as usize] != NONE {
                break;
            }
            pinned[c as usize] = rank as u16;
            cur = nodes[c as usize].parents.first().copied();
        }
    }
    let reserved = trunks.len() as u16;

    // État des lanes : quel nœud chaque lane attend, et sa chaîne.
    let mut expect: Vec<Option<u32>> = vec![None; reserved as usize];
    let mut lane_chain: Vec<u32> = vec![u32::MAX; reserved as usize];
    let mut chain_tip: Vec<u32> = Vec::new();
    let mut trunk_chain: Vec<Option<u32>> = vec![None; reserved as usize];
    // Lanes libérées à la ligne courante : pas réutilisables sur cette même ligne.
    let mut cooling: Vec<u16> = Vec::new();

    let mut lane = vec![0u16; n];
    let mut chain = vec![0u32; n];
    // Pour chaque lien parent : (enfant, parent, via_lane, chaîne du trait)
    let mut links: Vec<(u32, u32, u16, u32)> = Vec::with_capacity(n + n / 4);
    let mut max_lanes = reserved.max(1);

    let new_chain = |tip: u32, chain_tip: &mut Vec<u32>| -> u32 {
        chain_tip.push(tip);
        (chain_tip.len() - 1) as u32
    };

    for &c in &order {
        let ci = c as usize;
        cooling.clear();

        // 1. Lane du commit.
        let waiting: Vec<u16> = expect
            .iter()
            .enumerate()
            .filter(|(_, e)| **e == Some(c))
            .map(|(l, _)| l as u16)
            .collect();
        let my_lane = if pinned[ci] != NONE {
            pinned[ci]
        } else if let Some(&l) = waiting.iter().find(|&&l| l >= reserved) {
            l
        } else if let Some(&l) = waiting.first() {
            l
        } else {
            alloc(&mut expect, &mut lane_chain, reserved, &cooling)
        };
        if my_lane as usize >= expect.len() {
            expect.resize(my_lane as usize + 1, None);
            lane_chain.resize(my_lane as usize + 1, u32::MAX);
        }
        // Chaîne : celle de la lane si elle attendait ce commit, sinon une nouvelle.
        let my_chain = if pinned[ci] != NONE {
            let rank = pinned[ci] as usize;
            *trunk_chain[rank].get_or_insert_with(|| new_chain(c, &mut chain_tip))
        } else if expect[my_lane as usize] == Some(c) && lane_chain[my_lane as usize] != u32::MAX {
            lane_chain[my_lane as usize]
        } else {
            new_chain(c, &mut chain_tip)
        };
        lane[ci] = my_lane;
        chain[ci] = my_chain;

        // Les autres lanes qui attendaient ce commit convergent ici et se libèrent.
        for &l in &waiting {
            if l != my_lane {
                expect[l as usize] = None;
                if l >= reserved {
                    cooling.push(l);
                }
            }
        }
        expect[my_lane as usize] = None;
        lane_chain[my_lane as usize] = my_chain;

        // 2. Parents.
        let parents = &nodes[ci].parents;
        for (j, &p) in parents.iter().enumerate() {
            if j == 0 {
                // Le premier parent continue la lane (ou la rejoindra plus bas).
                expect[my_lane as usize] = Some(p);
                links.push((c, p, my_lane, my_chain));
            } else if let Some(l) = expect.iter().position(|e| *e == Some(p)) {
                links.push((c, p, l as u16, lane_chain[l]));
            } else {
                let l = alloc(&mut expect, &mut lane_chain, reserved, &cooling);
                let ch = new_chain(p, &mut chain_tip);
                expect[l as usize] = Some(p);
                lane_chain[l as usize] = ch;
                links.push((c, p, l, ch));
            }
        }
        if parents.is_empty() && my_lane >= reserved {
            cooling.push(my_lane);
        }
        max_lanes = max_lanes.max(expect.len() as u16);
        while expect.len() > reserved as usize && expect.last() == Some(&None) {
            // On garde la taille maximale atteinte dans max_lanes ; on tasse l'état.
            let last = expect.len() as u16 - 1;
            if cooling.contains(&last) {
                break;
            }
            expect.pop();
            lane_chain.pop();
        }
    }

    let mut edges: Vec<Edge> = links
        .into_iter()
        .map(|(c, p, via, ch)| Edge {
            from_row: row_of[c as usize],
            to_row: row_of[p as usize],
            from_lane: lane[c as usize],
            via_lane: via,
            to_lane: lane[p as usize],
            chain: ch,
        })
        .collect();
    edges.sort_by_key(|e| (e.from_row, e.to_row));

    let lane_rows = order.iter().map(|&i| lane[i as usize]).collect();
    let chain_rows = order.iter().map(|&i| chain[i as usize]).collect();
    Layout {
        order,
        row_of,
        lane: lane_rows,
        chain: chain_rows,
        edges,
        max_lanes,
        chain_tip,
    }
}

/// Première lane libre hors troncs, en évitant celles libérées sur la ligne courante.
fn alloc(expect: &mut Vec<Option<u32>>, lane_chain: &mut Vec<u32>, reserved: u16, cooling: &[u16]) -> u16 {
    let free = (reserved as usize..expect.len()).find(|&l| expect[l].is_none() && !cooling.contains(&(l as u16)));
    if let Some(l) = free {
        return l as u16;
    }
    expect.push(None);
    lane_chain.push(u32::MAX);
    (expect.len() - 1) as u16
}

/// Arêtes visibles dans la fenêtre de lignes `[start, end)`.
pub fn edges_in(edges: &[Edge], start: u32, end: u32) -> impl Iterator<Item = &Edge> {
    let upto = edges.partition_point(|e| e.from_row < end);
    edges[..upto].iter().filter(move |e| e.to_row >= start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(parents: &[u32], time: i64) -> Node {
        Node {
            parents: parents.to_vec(),
            time,
        }
    }

    #[test]
    fn date_order_children_before_parents() {
        // 0 <- 1 <- 2 (main), 0 <- 3 (branche, plus récente que 1 mais plus ancienne que 2)
        let nodes = vec![n(&[], 0), n(&[0], 10), n(&[1], 30), n(&[0], 20)];
        assert_eq!(date_order(&nodes), vec![2, 3, 1, 0]);
    }

    #[test]
    fn linear_history_is_one_straight_lane() {
        let nodes = vec![n(&[], 0), n(&[0], 1), n(&[1], 2), n(&[2], 3)];
        let l = layout(&nodes, &[3]);
        assert!(l.lane.iter().all(|&x| x == 0));
        assert!(l.edges.iter().all(|e| e.from_lane == 0 && e.via_lane == 0 && e.to_lane == 0));
        assert_eq!(l.max_lanes, 1);
        assert!(l.chain.iter().all(|&c| c == l.chain[0]));
    }

    #[test]
    fn trunk_stays_in_lane_zero_even_when_branch_is_newer() {
        // main : 0 <- 1 <- 2 ; feature : 1 <- 3 <- 4 (plus récente que tout main)
        let nodes = vec![n(&[], 0), n(&[0], 1), n(&[1], 2), n(&[1], 3), n(&[3], 4)];
        let l = layout(&nodes, &[2]);
        for (row, &node) in l.order.iter().enumerate() {
            let expected = if [0, 1, 2].contains(&node) { 0 } else { 1 };
            assert_eq!(l.lane[row], expected, "nœud {node}");
        }
        // La branche garde une seule chaîne, différente du tronc.
        let ch = |node: u32| l.chain[l.row_of[node as usize] as usize];
        assert_eq!(ch(3), ch(4));
        assert_ne!(ch(3), ch(2));
        // L'arête 3 -> 1 descend dans la lane de la branche puis rejoint le tronc.
        let e = l.edges.iter().find(|e| e.from_row == l.row_of[3]).unwrap();
        assert_eq!((e.from_lane, e.via_lane, e.to_lane), (1, 1, 0));
    }

    #[test]
    fn merge_edge_leaves_through_the_merged_branch_lane() {
        // main : 0 <- 1 <- 3(merge 1,2) ; feature : 0 <- 2
        let nodes = vec![n(&[], 0), n(&[0], 1), n(&[0], 2), n(&[1, 2], 3)];
        let l = layout(&nodes, &[3]);
        let merge_row = l.row_of[3];
        let second = l.edges.iter().find(|e| e.from_row == merge_row && e.to_row == l.row_of[2]).unwrap();
        assert_eq!(second.from_lane, 0);
        assert_eq!(second.via_lane, 1);
        assert_eq!(second.to_lane, 1);
        assert_eq!(l.max_lanes, 2);
    }

    #[test]
    fn lanes_are_reused_after_a_branch_ends() {
        // Deux branches successives mergées dans main : la seconde réutilise la lane 1.
        // main : 0 <- 1 <- 3(m 1,2) <- 4 <- 6(m 4,5) ; b1 : 1 <- 2 ; b2 : 4 <- 5
        let nodes = vec![
            n(&[], 0),
            n(&[0], 1),
            n(&[1], 2),
            n(&[1, 2], 3),
            n(&[3], 4),
            n(&[4], 5),
            n(&[4, 5], 6),
        ];
        let l = layout(&nodes, &[6]);
        assert_eq!(l.lane[l.row_of[2] as usize], 1);
        assert_eq!(l.lane[l.row_of[5] as usize], 1);
        assert_eq!(l.max_lanes, 2);
    }

    #[test]
    fn develop_is_pinned_to_lane_one() {
        // main : 0 <- 1 ; develop : 1 <- 2 <- 3 ; feature : 2 <- 4
        let nodes = vec![n(&[], 0), n(&[0], 1), n(&[1], 2), n(&[2], 3), n(&[2], 4)];
        let l = layout(&nodes, &[1, 3]);
        assert_eq!(l.lane[l.row_of[3] as usize], 1);
        assert_eq!(l.lane[l.row_of[2] as usize], 1);
        assert_eq!(l.lane[l.row_of[4] as usize], 2);
        assert_eq!(l.lane[l.row_of[1] as usize], 0);
    }

    #[test]
    fn edges_window_query() {
        let nodes = vec![n(&[], 0), n(&[0], 1), n(&[1], 2), n(&[2], 3)];
        let l = layout(&nodes, &[3]);
        let v: Vec<_> = edges_in(&l.edges, 1, 2).collect();
        // Arêtes 0->1 (touche la ligne 1) et 1->2 (part de la ligne 1).
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn no_two_live_edges_share_a_lane_segment() {
        // Historique pseudo-aléatoire : on vérifie qu'aucun segment vertical ne se superpose
        // à un autre allant vers un parent différent.
        let mut nodes = vec![n(&[], 0)];
        let mut seed = 42u64;
        let mut rnd = |m: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % m
        };
        for i in 1..400u32 {
            let p = i - 1 - (rnd(i.min(12) as u64) as u32);
            let mut parents = vec![p];
            if i > 5 && rnd(5) == 0 {
                let q = i - 1 - (rnd(i.min(20) as u64) as u32);
                if q != p {
                    parents.push(q);
                }
            }
            nodes.push(n(&parents, i as i64));
        }
        let l = layout(&nodes, &[399]);
        // Pour chaque ligne et chaque lane, le parent visé par le segment vertical qui la traverse.
        let rows = nodes.len();
        let mut occ: Vec<std::collections::HashMap<u16, u32>> = vec![Default::default(); rows];
        for e in &l.edges {
            for r in e.from_row + 1..e.to_row {
                let prev = occ[r as usize].insert(e.via_lane, e.to_row);
                if let Some(t) = prev {
                    assert_eq!(t, e.to_row, "deux arêtes différentes dans la lane {} à la ligne {r}", e.via_lane);
                }
            }
        }
        // Aucun nœud n'est posé sur une lane traversée par une arête qui ne le concerne pas.
        for (row, &lane) in l.lane.iter().enumerate() {
            if let Some(&t) = occ[row].get(&lane) {
                panic!("le nœud de la ligne {row} est sur une lane traversée (vers la ligne {t})");
            }
        }
    }
}
