# ramure *(nom de code)*

> Statut : **spécification** — spec v0.4 · open source (MIT OR Apache-2.0)

## Résumé

**Viewer git** de bureau autonome (**Tauri 2 + Rust + Vue 3**) pour remplacer GitKraken, dont
l'équipe n'utilise qu'une petite partie : la visualisation du **commit graph**, la **recherche
instantanée**, le lancement de **rebase** et les commandes git courantes par **drag & drop** ou
**menu contextuel**. Priorité à la lisibilité du graph (refs · graph · messages alignés).

Ramure lit le dépôt (`gix`) et **ne le modifie jamais** : chaque geste produit une **commande git
native** expliquée, avec aperçu et commande d'annulation (y compris `git rebase --onto` pour les
branches empilées). Niveau 0 : la commande est copiée. Niveau 1 : elle est exécutée dans le
terminal choisi (au minimum le terminal par défaut du système, puis tmux, iTerm2, WezTerm, kitty…),
toujours dans le shell de l’utilisateur avec sa config (oh-my-zsh, alias…), ⌥ pour copier à la place. Aucune intégration de service externe, aucune
requête réseau, identités gérées par la config git (`includeIf`).

Ce projet a vocation à être extrait dans son propre dépôt public (`Creatiwity/ramure`) au
démarrage de la phase 1.

## Contenu de ce dossier

- `spec-ramure.md` : cahier des charges (objectifs, fonctionnalités F-xx en MoSCoW, exigences
  de performance, architecture, feuille de route, risques, critères d'acceptation) ; fait foi.
- `ux-graph.md` : analyse de lisibilité du commit graph (principes UX sourcés, décisions D1 à D8, protocole de test).
- `design-board.html` : planche design (fenêtre principale, comparaison v0.1/v0.2, recherche, `--onto`, drag & drop,
  rebase interactif, palette, menu contextuel, états, fondations). À ouvrir dans un navigateur ; bascule
  clair/sombre en haut de page.
