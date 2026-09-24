# Ramure : lisibilité du commit graph

> Version 0.2 · Document de travail pour challenger la vue principale.
> Maquettes correspondantes : planches **A** et **B** de [`design-board.html`](./design-board.html).

Le graph est l'écran principal de Ramure et l'outil n'a pas d'autre ambition que d'être un
*viewer* très lisible avec un peu de sucre. Ce document part des tâches réelles, rappelle les
principes de perception et d'UX applicables, puis tranche chaque choix de mise en page.

---

## 1. Ce que l'utilisateur fait avec le graph

Par ordre de fréquence observée avec GitKraken :

| # | Tâche | Ce que l'œil doit trouver |
|---|-------|---------------------------|
| T1 | « Où en est ma branche par rapport à `develop` / `main` / `origin` ? » | La pastille de ma branche, celle de la base, la lane qui les relie, le point de divergence. |
| T2 | « Retrouver un commit » | Un bout de message dans une colonne de texte, lue de haut en bas. |
| T3 | « Qu'est-ce qui a été mergé depuis la release ? » | Les merges et les tags, sur le tronc. |
| T4 | « Préparer un geste » (rebase, cherry-pick, reset) | La source et la cible, et qu'elles soient faciles à viser. |

T1 et T3 se lisent **par les refs et la forme du graph** ; T2 se lit **par la colonne de
messages**. Une bonne mise en page sert ces deux lectures sans que l'une gêne l'autre.

## 2. Principes appliqués

| Principe | Source | Conséquence pour Ramure |
|----------|--------|-------------------------|
| L'œil balaie le bord gauche d'une colonne de haut en bas ; les premiers mots de chaque ligne reçoivent le plus de fixations. | NN/g, *F-Shaped Pattern of Reading* | Les messages doivent **tous commencer au même x**, et les premiers mots doivent être porteurs de sens. |
| Un bord de départ variable produit une lecture « en zigzag » plus lente. | NN/g, *Zigzag Image–Text Layouts* ; conventions d'alignement des tableaux | Rien ne doit décaler le début du message : ni pastilles, ni largeur de graph variable. |
| Proximité : ce qui est proche est perçu comme lié. Région commune : un contenant groupe plus fort que la proximité. | NN/g, *Proximity Principle*, *Common Region* | Une pastille de ref doit être **près de son nœud** et séparée des messages. |
| Similarité : même couleur, même forme ⇒ même famille. | NN/g, *Similarity Principle* | La couleur d'une pastille = la couleur de sa lane. Une forme par type de ref. |
| Continuité : l'œil suit une ligne. | NN/g, *Continuation* (Gestalt) | Un trait relie la pastille au nœud ; les branches restent droites. |
| Les attributs pré-attentifs (couleur, taille, graisse) créent la hiérarchie ; trop de couleurs l'annulent. | NN/g, *Visual Hierarchy in UX* | Peu de couleurs vives, réservées à ce qui compte (ma branche, la base). |
| « Overview first, zoom and filter, then details on demand. » | Shneiderman, *The Eyes Have It* (1996) | Vue d'ensemble (minimap), filtres (branches, first-parent), détails au clic seulement. |
| Maximiser la part d'encre qui porte l'information. | Tufte, *The Visual Display of Quantitative Information* | Retirer le répété (dates identiques, préfixes, sha par défaut). |
| La couleur ne doit pas être le seul porteur d'information. | WCAG 2.2, critère 1.4.1 *Use of Color* | HEAD, WIP, stash, tag ont aussi une forme propre. |
| Branches droites : tous les commits d'une branche sur la même colonne. | pvigier, *Commit Graph Drawing Algorithms* (2019) | Algorithme de lanes « straight branches », tronc épinglé à gauche. |

## 3. Challenge de la maquette v0.1

La v0.1 plaçait les pastilles **dans** la colonne message, avant le texte. Défauts constatés :

1. Le début du message saute de 0 à ~250 px selon le nombre et la longueur des refs : c'est
   exactement le bord gauche en zigzag que les études déconseillent (T2 ralentie).
2. Les pastilles, très colorées, deviennent l'élément le plus saillant de la ligne alors qu'elles
   ne concernent que 1 ligne sur 10 environ.
3. La pastille est loin de son nœud quand la lane est à gauche, et le texte de la pastille se lit
   comme le début du message (proximité avec le mauvais élément).
4. Toutes les branches colorées « arc-en-ciel » : sur un dépôt réel à 15 branches actives, la
   couleur ne désigne plus rien.

## 4. Décisions

### D1. Trois zones : refs | graph | texte (le graph sépare les deux)

```
┌─────────── Refs ───────────┬── Graph ──┬──────── Message ─────────┬ Auteur ┬ Date ┐
│        ✓ feature/scoring ──┼──◉        │ feat  scoring  pondère…  │ Julien │ 17:02│
│              ⌂☁ main ──────┼───●       │ fix   api  gère le 404…  │ Camille│ 15:10│
│                            │  │●       │ refactor scoring extrait…│ Julien │ hier │
```

Même principe que GitKraken, que l'on reprend parce qu'il répond aux principes ci-dessus : les
refs (le *où*) et les messages (le *quoi*) sont deux lectures distinctes, et le graph, qui les
relie, sert de séparateur naturel. Améliorations par rapport à GitKraken :

- **Pastilles alignées à droite**, contre le graph, avec un **trait de liaison** jusqu'au nœud
  dans la couleur de la branche (proximité + continuité). Dans GitKraken la pastille est alignée
  à gauche et peut se retrouver loin de son nœud.
- **Une seule pastille visible par ligne**, les autres en compteur `+2` (survol = liste
  complète). Ordre de priorité : HEAD › branche locale › distante › tag › stash. Une locale et sa
  distante au même commit fusionnent en une pastille (icônes ordinateur + nuage).
- **Troncature au milieu** des noms longs (`feature/…/scoring-v2`), jamais à la fin : le préfixe
  et la fin portent l'information.
- Colonne refs redimensionnable (défaut 170 px), repliable en mode « compact » où il ne reste que
  le trait et une icône.

*Alternative écartée* : refs dans une colonne fixe entre le graph et le message. Le message est
bien aligné, mais refs et message deviennent deux colonnes de texte contiguës, sans séparation ;
la pastille est de nouveau loin de son nœud sur les lanes de droite.

*Alternative écartée* : refs après le message, à droite. Alignement parfait, mais T1 (la tâche
la plus fréquente) oblige à chercher les refs au bout de lignes de longueur variable.

### D2. Largeur de graph constante

La colonne graph a une largeur **fixe** calculée sur l'historique chargé (et non sur les lignes
visibles), plafonnée à 10 lanes. Au-delà, les lanes se resserrent (16 → 10 px) puis un
indicateur `›` signale des lanes masquées. Sinon, le bord gauche des messages bougerait au
défilement, ce qui annulerait D1.

### D3. Lanes stables, tronc épinglé

- Algorithme *straight branches* : tous les commits d'une branche sur la même colonne.
- `main` (ou la branche par défaut) toujours en colonne 0, `develop` en colonne 1 s'il existe ;
  la branche courante prend la première colonne libre ensuite. Liste configurable par dépôt.
- On minimise les croisements : une lane libérée n'est réutilisée qu'après une ligne de répit.

### D4. Couleur « focus » par défaut, arc-en-ciel en option

- Couleurs vives uniquement pour : la branche courante, sa base (tronc) et la branche survolée
  ou sélectionnée. Les autres branches gardent leur teinte mais **désaturée** : on les distingue
  encore, elles ne crient plus.
- Mode « arc-en-ciel » (une couleur vive par branche, façon GitKraken) disponible en un clic.
- La couleur suit la branche, pas la colonne.

### D5. Colonne message : le bord gauche porte le sens

- Si le dépôt suit *Conventional Commits* (détection : plus de 60 % des 500 derniers messages),
  le **type** (`feat`, `fix`, `chore`…) passe dans une sous-colonne étroite en gris, la **portée**
  en gris secondaire, et le **sujet** commence toujours au même x. Les premiers mots lus sont donc
  le sujet réel et non « feat(scoring): ».
- Les merges sont réécrits en forme courte et atténués : `Merge pull request #142 from
  fix/pwa-cache` devient `⤙ fix/pwa-cache  #142`. Le message original reste au survol et dans
  les détails.
- Mode **first-parent** (un clic) pour ne voir que l'histoire du tronc.

### D6. Retirer l'encre répétée

- **Date affichée seulement quand elle change** (première ligne du jour), en relatif
  (« 17:02 », « hier », « lun. 21 ») ; date exacte au survol.
- **Sha masqué par défaut** (colonne activable) : il est dans les détails et dans chaque
  commande générée.
- Auteur : initiales colorées + prénom ; auteurs consécutifs identiques affichés en gris clair.
- Pas de zébrage : les lanes verticales suffisent à guider l'œil ; la ligne survolée est
  surlignée sur toute la largeur (refs incluses) et son nœud grossit.

### D7. Vue d'ensemble et filtres

- **Minimap** dans la barre de défilement : marques pour HEAD, les branches locales, les tags et
  les résultats de recherche (comme les décorations de la barre de défilement d'un éditeur).
- Filtres en un clic : branches (afficher/masquer), remotes, tags, stashes, first-parent,
  « ma branche et sa base seulement ».
- Branches mergées repliables en une ligne (« 12 commits de fix/pwa-cache »).

### D8. Ne pas dépendre de la couleur

Formes distinctes pour chaque type de nœud (commit plein, HEAD cerclé, merge réduit, WIP en
pointillé, stash carré) et pour chaque type de pastille (icône ordinateur, nuage, étiquette,
boîte). Palette de lanes vérifiée en deutéranopie et protanopie.

## 5. Comment valider

Test rapide avec 5 développeur·ses de l'équipe, sur le même dépôt ouvert dans GitKraken puis
dans le prototype, tâches chronométrées :

1. « Combien de commits `feature/scoring` a-t-elle d'avance sur `main` ? » (T1)
2. « Trouve le commit qui a corrigé le cache de la PWA. » (T2)
3. « Qu'est-ce qui est entré dans `main` depuis `v0.4.0` ? » (T3)
4. « Rebase `feat/a` sur le nouveau `develop`. » (T4, avec copie de la commande)

Critère : Ramure au moins aussi rapide que GitKraken sur les 4 tâches, et plus rapide sur T2.

## Sources

- NN/g, [F-Shaped Pattern of Reading on the Web: Misunderstood, But Still Relevant](https://www.nngroup.com/articles/f-shaped-pattern-reading-web-content/)
- NN/g, [Text Scanning Patterns: Eyetracking Evidence](https://www.nngroup.com/articles/text-scanning-patterns-eyetracking/)
- NN/g, [Zigzag Image–Text Layouts Make Scanning Less Efficient](https://www.nngroup.com/articles/zigzag-page-layout/)
- NN/g, [Proximity Principle in Visual Design](https://www.nngroup.com/articles/gestalt-proximity/)
- NN/g, [The Principle of Common Region](https://www.nngroup.com/articles/common-region/)
- NN/g, [Similarity Principle in Visual Design](https://www.nngroup.com/articles/gestalt-similarity/)
- NN/g, [Continuation: Gestalt Principle (vidéo)](https://www.nngroup.com/videos/continuation-gestalt/)
- NN/g, [Visual Hierarchy in UX: Definition](https://www.nngroup.com/articles/visual-hierarchy-ux-definition/)
- Pencil & Paper, [Data Table Design UX Patterns & Best Practices](https://www.pencilandpaper.io/articles/ux-pattern-analysis-enterprise-data-tables)
- B. Shneiderman, [The Eyes Have It: A Task by Data Type Taxonomy for Information Visualizations](https://www.cs.umd.edu/~ben//papers/Shneiderman1996eyes.pdf) (1996)
- E. Tufte, *The Visual Display of Quantitative Information* (2e éd., 2001), chapitre sur le *data-ink ratio*
- W3C, [Understanding WCAG 2.2 — 1.4.1 Use of Color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html)
- P. Vigier, [Commit Graph Drawing Algorithms](https://pvigier.github.io/2019/05/06/commit-graph-drawing-algorithms.html) (2019)
- DoltHub, [Drawing a commit graph](https://www.dolthub.com/blog/2024-08-07-drawing-a-commit-graph/) (2024)

Pour aller plus loin : les catégories *Visual Design* et *Data Visualization* de
[nngroup.com](https://www.nngroup.com/topic/visual-design/), le livre de Colin Ware
*Information Visualization: Perception for Design*, et la comparaison directe avec les clients
existants (GitKraken, Fork, Sublime Merge, Tower, `git log --graph`, tig).
