# Ramure v0.1 : mesures de performance

Mesures du 24 septembre 2026, build release, sur une VM cloud à **4 cœurs partagés** (les
chiffres varient d'environ ±30 % d'un passage à l'autre ; un poste de développement récent à 8–12
cœurs devrait faire mieux sur tout ce qui est parallèle).

Dépôt de référence : `fixtures/gen-repo.py`, **100 003 commits**, `main` + `develop`, ~18 600
branches de fonctionnalité mergées (merge commits), 3 branches longues, 833 tags, fichier
`commit-graph` présent.

```bash
python3 fixtures/gen-repo.py /tmp/big100k 100000
cargo run --release -p ramure-core --example bench -- /tmp/big100k
```

## Résultats

| Indicateur (spec §4.1) | Cible 100 k | Mesuré | Verdict |
|------------------------|-------------|--------|---------|
| Ouverture complète (refs, parcours, décodage, lanes, index de recherche, statut) | < 1 s à chaud, < 2 s à froid | **510–610 ms** (à froid comme à chaud) ; 544 ms affiché par l'application | Tenu |
| Recherche par frappe (« pwa cache » tapé lettre à lettre) | < 16 ms | **8–15 ms** par frappe, pic isolé à 18 ms | Tenu, marge faible sur cette VM |
| Arêtes d'une fenêtre de 200 lignes | — | 0,2 ms | — |
| Défilement | 60 fps | Fluide sous Xvfb sans accélération GPU ; pas encore mesuré en fps | À mesurer sur poste réel |
| Mémoire | < 300 Mo | Cœur : **~120 Mo** (pic 170 Mo pendant l'ouverture). Application complète : ~250 Mo processus principal (cœur + GTK/WebKit liés) + ~255 Mo processus web WebKit | **Non tenu** pour l'application complète |

Détail d'une ouverture (ms) : refs 15–28 · parcours de l'historique 67–100 · décodage des commits
(parallèle) 215–227 · lanes 52–63 · index de recherche 67–110 · statut git 29–37.

## Ce qui a été optimisé pendant la v0.1

- **Recherche** : 17–32 ms au départ, ramenée à 8–15 ms en (1) ne calculant les positions à
  surligner que pour les lignes visibles, (2) réutilisant un matcher nucleo par thread au lieu d'en
  allouer un par tâche rayon, (3) filtrant les seuls résultats précédents quand la nouvelle requête
  prolonge l'ancienne (comme fzf). Un test vérifie que ce filtrage incrémental donne exactement
  les mêmes résultats qu'une recherche complète.
- **Graph** : le front ne reçoit que les lignes et arêtes de tranches de 256 lignes autour de la
  zone visible ; le canvas ne dessine que la fenêtre visible (+12 lignes de marge).

## Pistes

- **Mémoire** : l'objectif de 300 Mo pour toute l'application est irréaliste avec une webview
  (WebKit seul occupe ~255 Mo ici). Proposition : viser < 150 Mo pour le cœur sur 100 k commits et
  mesurer la webview à part. Côté cœur, le sujet de chaque commit est stocké trois fois
  (`CommitInfo`, index du sujet, index « sujet + auteur + refs ») et le sha en hexadécimal : il y a
  de la marge.
- **Décodage** : c'est le poste le plus lourd ; le graph pourrait s'afficher avant la fin du
  décodage (streaming, spec §4.1 « à froid »), avec les messages remplis au fil de l'eau.
- **1 M de commits** : pas encore mesuré (la génération Python du dépôt prend ~15 min) ;
  prochaine étape de la phase 0.
