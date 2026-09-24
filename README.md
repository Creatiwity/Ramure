# ramure *(nom de code)*

> Statut : **spécification** — spec v0.1

## Résumé

Client git de bureau autonome (**Tauri 2 + Rust + Vue 3**) pour remplacer GitKraken, dont
l'équipe n'utilise qu'une petite partie : la visualisation du **commit graph**, la **recherche
instantanée**, le lancement de **rebase** (interactif compris) et les commandes git courantes par
**drag & drop** ou **menu contextuel**. Ramure se concentre sur ces usages, sans abonnement, hors
ligne, et vise à être plus rapide sur les gros dépôts (100 k à 1 M commits).

Lecture git en natif via `gix` (gitoxide), écriture via le binaire `git` du système (fidélité aux
hooks, à la config, à la signature), et annulation (Ctrl+Z) de toute opération qui déplace des refs.

Ce projet a vocation à être extrait dans son propre dépôt (`Creatiwity/ramure`) au démarrage de
la phase 1.

## Contenu de ce dossier

- `spec-ramure.md` : cahier des charges (objectifs, fonctionnalités F-xx en MoSCoW, exigences
  de performance, architecture, feuille de route, risques, critères d'acceptation) ; fait foi.
- `design-board.html` : planche design (fenêtre principale, recherche, palette, drag & drop,
  menu contextuel, rebase interactif, états, fondations). À ouvrir dans un navigateur ; bascule
  clair/sombre en haut de page.
