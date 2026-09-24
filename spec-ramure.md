# Ramure — cahier des charges

> Version 0.1 · Statut : **spécification** (draft à challenger)
> Planche design associée : [`design-board.html`](./design-board.html)

---

## 1. Contexte

L'équipe utilise GitKraken essentiellement pour **visualiser le commit graph** et pour quelques
gestes rapides : recherche instantanée, lancement d'un rebase (interactif ou non), commandes git
courantes par **drag & drop** ou **menu contextuel**. Le reste de l'offre (workspaces cloud,
intégrations issues, Launchpad, IA, etc.) n'est pas utilisé, alors que l'abonnement est facturé
par siège, chaque année.

**Ramure** (nom de code) est un client git de bureau, autonome et hors ligne, construit avec
**Tauri 2 + Rust + Vue 3**, qui reprend uniquement ces fonctionnalités, avec un objectif de
performance supérieur à GitKraken sur les gros dépôts.

### 1.1 Objectifs

| # | Objectif | Mesure de succès |
|---|----------|------------------|
| O1 | Remplacer GitKraken pour l'usage quotidien de l'équipe | 100 % de l'équipe sur Ramure 1 mois après la V1, abonnement résilié |
| O2 | Graph plus rapide que GitKraken | Ouverture d'un dépôt de 100 k commits : graph affiché en < 1 s (à chaud) |
| O3 | Recherche « au fil de la frappe » | < 16 ms par frappe sur 100 k commits (message, sha, auteur, ref) |
| O4 | Aucune opération destructrice irréversible | Toute opération qui déplace une ref est annulable (Ctrl+Z) |
| O5 | Application légère | Installeur < 20 Mo, RAM < 300 Mo sur 100 k commits |

### 1.2 Hors périmètre (explicitement)

- Hébergement cloud, comptes utilisateurs, synchronisation de préférences.
- Gestion d'issues, tableaux Kanban, « Workspaces ».
- Fonctions IA (message de commit généré, etc.) — éventuellement plus tard, en option.
- Remplacer un IDE : pas d'édition de fichiers hors résolution de conflits.
- Support de VCS autres que git.

---

## 2. Utilisateurs et cas d'usage

**Persona principal : développeur·se de l'équipe**, à l'aise avec git en ligne de commande, qui
veut *voir* l'historique et exécuter vite les opérations courantes sans taper les commandes ni
risquer une erreur.

Cas d'usage prioritaires (issus de l'usage actuel de GitKraken) :

1. **Comprendre l'état d'un dépôt** : où sont mes branches par rapport à `main` et à `origin` ?
   Qu'est-ce qui a été mergé ? Qui a touché quoi ?
2. **Retrouver un commit** en quelques frappes (bout de message, sha, auteur, nom de branche,
   ticket `#142`).
3. **Rebaser ma branche** sur `main` à jour : glisser `feature/x` sur `main` → « Rebase ».
4. **Nettoyer ma branche avant PR** : rebase interactif (squash, fixup, reword, réordonner).
5. **Gestes courants** : checkout, créer une branche à un commit, cherry-pick, reset, revert,
   stash, fetch/pull/push, supprimer une branche locale et distante.
6. **Me sortir d'un conflit** pendant un rebase/merge : voir les fichiers en conflit, choisir
   ours/theirs, ouvrir mon outil de merge, continuer/annuler.
7. **Annuler une bêtise** : Ctrl+Z après un reset, un rebase ou une suppression de branche.

---

## 3. Fonctionnalités

Priorités **MoSCoW** : **M** = Must (MVP), **S** = Should (V1), **C** = Could (V2+).
Chaque fonctionnalité a un identifiant stable (`F-xx`) à référencer dans les issues/PR.

### 3.1 Dépôts

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-01 | M | Ouvrir un dépôt local (dialogue, glisser un dossier sur la fenêtre, argument CLI `ramure <chemin>`). |
| F-02 | M | Liste des dépôts récents, épinglables. |
| F-03 | S | Onglets : plusieurs dépôts ouverts dans une même fenêtre. |
| F-04 | M | Rafraîchissement automatique sur changement du dépôt (watcher sur `.git/` et arbre de travail, debounce). Aucun bouton « Refresh » nécessaire. |
| F-05 | S | Cloner un dépôt (URL + dossier cible, progression). |
| F-06 | C | Worktrees : lister, créer, ouvrir dans un onglet. |
| F-07 | C | Sous-modules : affichage de l'état, update. |

### 3.2 Commit graph (cœur du produit)

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-10 | M | Affichage du graph : une ligne par commit, colonnes *graph · message + refs · auteur · date · sha*. Colonnes redimensionnables et masquables. |
| F-11 | M | Lanes colorées, une couleur stable par branche (la couleur suit la branche au scroll et d'une session à l'autre). Courbes de merge/fork lisibles. |
| F-12 | M | Pastilles de refs sur les commits : branche locale, branche distante, tag, `HEAD`, stash. Une branche locale et sa distante au même commit sont fusionnées en une pastille (icônes ordinateur + nuage). |
| F-13 | M | Ligne « Changements non commités » (WIP) en tête, rattachée à `HEAD`, avec compteur de fichiers. |
| F-14 | M | Virtualisation : défilement fluide (60 fps) quelle que soit la taille de l'historique ; chargement progressif au-delà des N premiers commits. |
| F-15 | M | Sélection d'un commit → panneau de détails (§3.4). Sélection multiple (Maj/Cmd) → diff cumulé et actions groupées (cherry-pick, squash). |
| F-16 | S | Mise en évidence de l'ascendance : au survol/sélection, les commits hors ascendance du commit sélectionné sont atténués. |
| F-17 | S | Filtrer le graph : masquer/afficher une branche, un remote, les tags, les stashes ; mode « branche courante uniquement ». |
| F-18 | S | Indicateurs ahead/behind sur les branches locales (`↑2 ↓5`) dans la barre latérale et sur les pastilles. |
| F-19 | S | Aller à : `HEAD`, une branche, un sha (raccourcis §3.9). |
| F-20 | C | Replier une branche mergée en une seule ligne (« 12 commits de fix/pwa-cache »). |
| F-21 | C | Avatars auteurs (Gravatar désactivable, initiales colorées par défaut — pas de requête réseau par défaut). |

**Barre latérale** (M) : sections repliables *Branches locales · Remotes · Tags · Stashes*
(*Worktrees · Sous-modules* en C), avec filtre texte. Clic = scroll jusqu'au commit ; double-clic
sur une branche locale = checkout.

### 3.3 Recherche instantanée

La recherche est la seconde raison d'être de l'outil ; elle doit sembler **gratuite**.

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-30 | M | Champ de recherche (Ctrl/Cmd+F) qui filtre ou surligne au fil de la frappe sur : message, sha (préfixe), auteur (nom + e-mail), noms de refs. |
| F-31 | M | Deux modes : **surligner** (graph intact, navigation Entrée / Maj+Entrée entre résultats) et **filtrer** (seuls les résultats, graph simplifié). |
| F-32 | M | Correspondance floue tolérante (`pndr retard` trouve « pondère les tâches en retard »), résultats triés par pertinence puis date. Surlignage des caractères trouvés. |
| F-33 | S | Opérateurs : `author:julien`, `msg:"cache"`, `ref:feature/`, `sha:4f2a`, `before:2026-06-01`, `after:`, `merge:yes/no`. Combinables. |
| F-34 | S | Recherche par chemin `path:src/scoring/` (commits ayant modifié ce chemin) — exécutée en tâche de fond, résultats en streaming. |
| F-35 | C | Recherche dans le contenu des diffs (`diff:computeUrgency`, équivalent `git log -S/-G`) — tâche de fond, annulable. |
| F-36 | M | Compteur de résultats et durée (« 38 résultats · 3 ms ») : la performance est visible et donc tenue. |

**Palette de commandes** (M, Ctrl/Cmd+K) : même moteur flou, portée élargie — commits, branches,
tags, fichiers du dépôt, *et actions* (« Rebase interactif… », « Stash », « Fetch all »). C'est le
point d'entrée clavier de toutes les commandes.

### 3.4 Détails de commit et diff

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-40 | M | Panneau de détails : message complet, auteur/committer, dates, sha (copie en un clic), parents cliquables, refs, signature (vérifiée ou non). |
| F-41 | M | Liste des fichiers modifiés (statut A/M/D/R, +/−), en arbre ou à plat. |
| F-42 | M | Diff d'un fichier : vue unifiée et côte à côte, coloration syntaxique, repli des zones inchangées, ignorer les espaces. |
| F-43 | S | Diff entre deux commits quelconques (sélection multiple) ou entre un commit et l'arbre de travail. |
| F-44 | S | Historique et blame d'un fichier. |
| F-45 | C | Diff d'images (côte à côte, superposition). |

### 3.5 Arbre de travail, staging, commit

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-50 | M | Fichiers non indexés / indexés ; stage/unstage par fichier, tout stager, discard (avec confirmation). |
| F-51 | S | Stage/unstage par hunk et par ligne. |
| F-52 | M | Zone de message de commit (résumé + corps, compteur 50/72), commit, amend. |
| F-53 | S | Respect des hooks (`pre-commit`, `commit-msg`), de la signature GPG/SSH et du template de message configurés dans git. Sortie des hooks affichée en cas d'échec. |
| F-54 | S | Création rapide de commit `fixup!` / `squash!` ciblant un commit sélectionné (pour autosquash). |

### 3.6 Opérations sur branches et remotes

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-60 | M | Checkout d'une branche / d'un commit (detached, avec avertissement). Checkout d'une branche distante = création de la branche locale de suivi. |
| F-61 | M | Créer une branche à n'importe quel commit ; renommer ; supprimer (locale, distante, les deux). |
| F-62 | M | Fetch (tous les remotes, prune), pull (ff-only / rebase / merge, selon config), push (avec `--force-with-lease` uniquement, jamais `--force` nu), définition de l'upstream au premier push. |
| F-63 | M | Merge, rebase (non interactif), cherry-pick, revert, reset (soft / mixed / hard). |
| F-64 | M | Tags : créer (léger/annoté), supprimer, pousser. |
| F-65 | M | Stash : créer (avec message, inclure non suivis), appliquer, pop, supprimer ; stashes visibles dans le graph. |
| F-66 | S | Fetch automatique en arrière-plan (intervalle configurable, désactivé par défaut). |

### 3.7 Gestes : drag & drop et menu contextuel

**Drag & drop (M pour la matrice de base, S pour le reste).** On glisse une *pastille de ref* ou un
*commit* sur une *cible*. Au dépôt, un menu court propose les actions valides, la plus probable en
premier (Entrée pour valider). Pendant le glissement, les cibles valides sont mises en évidence et
l'action par défaut est prévisualisée en texte (« Rebase feature/scoring sur main »).

| Source → Cible | Actions proposées (ordre) |
|----------------|---------------------------|
| Branche locale A → branche B | Rebase A sur B · Merge B dans A · Merge A dans B · Fast-forward B vers A *(si possible)* |
| Branche locale A → commit C | Rebase A sur C · Reset A à C (soft/mixed/hard) |
| Branche locale A → sa distante | Push · Push force-with-lease *(si divergente)* |
| Branche distante → branche locale | Pull / fast-forward · Rebase sur la distante |
| Commit(s) → branche B | Cherry-pick sur B |
| Commit C → commit D (même branche) | Rebase interactif : déplacer C après D |
| Branche → zone « Supprimer » (barre latérale) | Supprimer (confirmation) |

Si l'action demande un checkout (ex. merge B dans A alors que `HEAD` est ailleurs), Ramure
l'annonce dans le libellé : « Checkout A puis merge B ».

**Menu contextuel (M).** Sur un commit :
checkout · créer une branche ici · créer un tag ici · cherry-pick · revert · reset *branche
courante* ici ▸ (soft / mixed / hard) · rebase interactif depuis ici · copier le sha / le message ·
comparer avec l'arbre de travail · créer un commit fixup! pour ce commit.

Sur une pastille de branche : checkout · merge dans courante · rebase courante sur celle-ci ·
push / pull · renommer · définir l'upstream · supprimer · copier le nom.

Chaque entrée affiche son raccourci clavier quand il existe.

### 3.8 Rebase interactif

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-80 | S | Éditeur visuel de la todo-list : une ligne par commit, action par ligne (`pick`, `reword`, `edit`, `squash`, `fixup`, `drop`), réordonnancement par glisser ou Alt+↑/↓. |
| F-81 | S | Raccourcis mono-touche sur la ligne sélectionnée : `p` `r` `e` `s` `f` `d` (comme la todo git). |
| F-82 | S | Reword inline (éditeur de message dans la ligne) ; pour `squash`, éditeur du message combiné. |
| F-83 | S | Prévisualisation du graph résultant à droite de l'éditeur (commits fusionnés, supprimés). |
| F-84 | S | Autosquash : les commits `fixup!`/`squash!` sont placés et typés automatiquement. |
| F-85 | M | Rebase en cours : bandeau persistant « Rebase en cours — 3/7 · Continuer · Passer · Annuler », quel que soit l'outil qui a lancé le rebase (y compris la CLI). |
| F-86 | S | Lancement : menu contextuel « Rebase interactif depuis ici », palette, ou glisser un commit sur un autre. |

### 3.9 Conflits

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-90 | M | Détection de l'état (merge / rebase / cherry-pick / revert en cours) et liste des fichiers en conflit. |
| F-91 | M | Par fichier : prendre *ours* / *theirs*, ouvrir dans l'outil de merge externe configuré (`git mergetool`) ou l'éditeur, marquer comme résolu. |
| F-92 | M | Continuer / annuler l'opération en cours. |
| F-93 | C | Éditeur de conflit intégré 3 volets (ours · résultat · theirs) avec choix par bloc. |

### 3.10 Annuler / rétablir

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-100 | M | Journal des opérations effectuées via Ramure (ref(s) touchées, valeur avant/après). |
| F-101 | M | Ctrl/Cmd+Z annule la dernière opération qui a déplacé des refs (reset, rebase, merge, commit, amend, suppression de branche, cherry-pick), avec toast « Rebase de feature/scoring annulé · Rétablir ». |
| F-102 | M | Refus explicite et motivé quand l'annulation n'est pas sûre (ex. branche déjà poussée depuis, arbre de travail sale) — jamais d'annulation silencieusement partielle. |
| F-103 | S | Vue « Journal » : liste des opérations + reflog de `HEAD`, restauration d'un état antérieur. |

### 3.11 Clavier

Tout doit être faisable au clavier. Raccourcis par défaut (Cmd sur macOS, Ctrl ailleurs),
personnalisables :

| Raccourci | Action |
|-----------|--------|
| Cmd+K | Palette de commandes |
| Cmd+F | Recherche dans le graph |
| ↑ / ↓ · J / K | Commit précédent / suivant |
| ← / → | Premier parent / premier enfant |
| H | Aller à `HEAD` |
| Espace | Ouvrir/fermer le panneau de détails |
| Cmd+Entrée | Commit (depuis la zone de message) |
| Cmd+Z / Cmd+Maj+Z | Annuler / rétablir une opération |
| Cmd+Maj+F / L / P | Fetch / pull / push |
| Cmd+B | Créer une branche au commit sélectionné |
| Cmd+Maj+S | Stash |
| Cmd+1..9 | Onglet de dépôt n |

### 3.12 Préférences

M : thème (clair / sombre / système), police et taille de l'interface et du code, format de date
(relatif/absolu), éditeur externe, outil de diff/merge externe, chemin du binaire `git`, langue
(FR/EN). S : raccourcis, couleurs des lanes, fetch automatique, confirmations désactivables.

---

## 4. Exigences non fonctionnelles

### 4.1 Performance (dépôt de référence : 100 k commits, 2 k refs ; stress : 1 M commits type noyau Linux)

| Indicateur | Cible 100 k | Cible 1 M |
|------------|-------------|-----------|
| Premier affichage du graph (dépôt déjà ouvert une fois) | < 1 s | < 2 s |
| Premier affichage (à froid) | < 2 s | < 5 s (streaming, UI utilisable avant la fin) |
| Recherche par frappe (message/sha/auteur/ref) | < 16 ms | < 60 ms |
| Défilement | 60 fps constant | 60 fps constant |
| Rafraîchissement après une opération (ex. commit) | < 150 ms | < 400 ms |
| Mémoire | < 300 Mo | < 1,2 Go |

Un **benchmark automatisé** (dépôts de référence clonés en CI) mesure ces indicateurs à chaque
release ; une régression > 20 % bloque la release.

### 4.2 Plateformes

macOS (Apple Silicon + Intel, ≥ 12), Windows 10/11 (x64, arm64), Linux (AppImage + .deb, X11 et
Wayland). Mises à jour via le plugin updater de Tauri (canal stable + beta).

### 4.3 Fiabilité et sécurité

- **Fidélité git** : toute opération d'écriture passe par le binaire `git` de l'utilisateur
  (voir §5.2) → hooks, config, credential helpers, signature et `.gitattributes` respectés.
- Aucune donnée ne quitte la machine : pas de télémétrie, pas de compte, pas d'avatar distant par
  défaut. Les seules requêtes réseau sont les commandes git (fetch/pull/push) et la vérification de
  mises à jour (désactivable).
- Identifiants : délégués à git (ssh-agent, credential manager du système). Les invites HTTPS
  passent par un helper `GIT_ASKPASS` qui affiche un dialogue Ramure ; rien n'est stocké par
  Ramure.
- Tauri : capabilities minimales (pas d'accès shell arbitraire depuis le front, liste blanche de
  commandes Rust), CSP stricte.
- Les opérations destructrices (reset hard, discard, suppression de branche non mergée, push
  force-with-lease) demandent une confirmation, sauf si l'utilisateur l'a désactivée.

### 4.4 Accessibilité et i18n

Navigation clavier complète, focus visible, contrastes WCAG AA dans les deux thèmes, couleurs de
lanes distinguables en daltonisme (palette testée deutéranopie/protanopie, et motif en complément
de la couleur pour `HEAD` et WIP). Interface en français et anglais dès le MVP (fichiers de
traduction `vue-i18n`).

---

## 5. Architecture technique

### 5.1 Vue d'ensemble

```
┌──────────────────────── Webview (Vue 3 + TS) ─────────────────────────┐
│  GraphCanvas (Canvas 2D)  │ VirtualRows (DOM)  │ Panels, Palette, DnD │
│            ▲ Pinia stores (repo, graph, search, ops, ui)              │
└────────────┼──────────────────────────────────────────────────────────┘
             │ Tauri IPC : commands (typées via tauri-specta) + Channels (streaming)
┌────────────┼──────────────────── Core Rust ───────────────────────────┐
│  repo::   gix (lecture : revwalk, objets, refs, status, diff)         │
│  graph::  topo-order + attribution des lanes → lignes compactes       │
│  search:: index colonnaire en mémoire + nucleo (flou) + rayon         │
│  ops::    exécution `git` CLI, parsing de progression, journal undo   │
│  watch::  notify (debounce) → invalidation incrémentale               │
│  askpass/sequence-editor : helpers (le binaire Ramure en mode helper) │
└───────────────────────────────────────────────────────────────────────┘
```

### 5.2 Accès git : lecture en natif, écriture via la CLI

| Besoin | Choix | Raison |
|--------|-------|--------|
| Lire l'historique, les refs, les objets, les diffs, le status | **gitoxide (`gix`)** | Pur Rust, très rapide, multi-thread, pas de dépendance C ; parcours de 1 M commits en quelques centaines de ms avec le commit-graph. |
| Écrire : commit, merge, rebase (interactif), cherry-pick, reset, stash, fetch/pull/push | **binaire `git` système** | Comportement identique à la CLI (hooks, config, signature, credential helpers, LFS). libgit2/`git2` ne gère pas le rebase interactif ni les hooks de façon fidèle. |
| Fallback lecture | `git` CLI | Si `gix` ne supporte pas une extension du dépôt (ex. format d'index exotique), on bascule sans casser l'UI. |

La version minimale de git requise est vérifiée au démarrage (cible : git ≥ 2.38 pour
`--update-refs`).

### 5.3 Graph : calcul des lanes en Rust, rendu hybride

- **Ordre** : `--topo-order` avec départage par date (identique au graph de GitKraken / `git log
  --graph`), utilise le fichier `commit-graph` quand présent (générations → tri incrémental).
- **Attribution des lanes** (O(n × largeur)) : on maintient un tableau des lanes actives, chaque
  lane « attend » un commit. Un commit prend la première lane qui l'attend (sinon une lane libre) ;
  son premier parent hérite de la lane ; chaque parent supplémentaire (merge) réutilise la lane qui
  l'attend déjà, sinon prend une lane libre. Les lanes libérées sont réutilisées au plus tôt pour
  garder le graph étroit.
- **Couleur** : attribuée par *branche logique* (chaîne de premiers parents) et non par colonne,
  pour qu'une branche garde sa couleur quand elle change de colonne. Hash stable du nom de ref
  la plus proche → palette de 8 couleurs.
- **Format transmis au front** : tableaux compacts (typed arrays sérialisés en binaire via
  `tauri::ipc::Channel`/`Response` brut) par tranche de 5 000 lignes : `lane`, `color`, et
  segments d'arêtes `(fromLane, toLane, kind)` par ligne. Le texte (message, auteur, date) est
  demandé à la demande pour la fenêtre visible + marge.
- **Rendu** : la colonne graph est un `<canvas>` redessiné pour la fenêtre visible uniquement ;
  les colonnes texte sont une liste DOM virtualisée (sélection de texte, accessibilité, menus
  contextuels natifs). Le canvas et la liste partagent la même hauteur de ligne fixe (28 px) et le
  même scroll.
- **Incrémental** : après une opération ou un fetch, seules les lignes au-dessus du plus ancien
  commit modifié sont recalculées.

### 5.4 Recherche

- À l'ouverture, construction d'un **index colonnaire** en mémoire (Rust) : `Vec` de sha, résumé,
  auteur, e-mail, timestamp, et une table refs → lignes. ~100 octets/commit → ~10 Mo pour 100 k.
- Correspondance floue avec **`nucleo`** (moteur du picker de Helix), parallélisé avec **`rayon`**,
  annulation de la requête précédente à chaque frappe (token de génération).
- Retour au front : indices de lignes + positions des caractères surlignés, limités à la fenêtre
  visible (le compteur total est calculé à part).
- `path:` et `diff:` : tâches de fond streamées (Channel), annulables, jamais bloquantes.

### 5.5 Opérations et rebase interactif

- Chaque opération = une commande Rust typée (`ops::rebase { onto, branch, interactive }`, …) qui
  construit la ligne de commande git, streame la progression (`--progress` parsé) et renvoie un
  résultat structuré (succès, conflit, refus de hook + sortie).
- **Rebase interactif** : Ramure lance `git rebase -i` avec
  `GIT_SEQUENCE_EDITOR="<ramure> --sequence-editor <fichier todo préparé>"` : le binaire Ramure,
  en mode helper, remplace la todo par celle éditée dans l'UI. Même principe pour `GIT_EDITOR`
  (messages reword/squash) et `GIT_ASKPASS`. On reste 100 % compatible avec le rebase de git
  (`--autosquash`, `--update-refs`, `exec`).
- **Journal d'undo** : avant chaque opération, snapshot des refs concernées + `HEAD` + état de
  l'index (stash interne si nécessaire). Annuler = `git update-ref` / `reset` vers le snapshot
  après vérifications (§3.10).
- Une seule opération d'écriture à la fois par dépôt (file d'attente), les lectures restent
  concurrentes.

### 5.6 Front

- Vue 3 (Composition API) + TypeScript strict + Vite, **Pinia** pour l'état.
- Bindings TS générés depuis les types Rust (`tauri-specta`) : aucune commande IPC écrite à la main.
- Diff : **CodeMirror 6** (vue diff/merge, coloration syntaxique, performances sur gros fichiers).
- Drag & drop : implémentation maison sur Pointer Events (le DnD HTML5 natif est limité dans les
  webviews et ne permet pas la prévisualisation riche).
- Pas de framework UI lourd : composants maison sur tokens CSS (voir planche design).
- Tests : Vitest (unitaires), Playwright sur le build web avec un backend mocké (parcours clés).

### 5.7 Arborescence cible

```
ramure/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs            # entrée app + modes helper (--sequence-editor, --askpass)
│   │   ├── repo/              # ouverture, refs, status (gix)
│   │   ├── graph/             # topo-order, lanes, couleurs, chunks
│   │   ├── search/            # index + requêtes
│   │   ├── ops/               # commandes git CLI, progression, undo
│   │   ├── watch.rs
│   │   └── ipc.rs             # commandes Tauri exportées (specta)
│   ├── benches/               # criterion : lanes, recherche
│   └── tauri.conf.json
├── src/                       # Vue
│   ├── components/graph/      # GraphCanvas, VirtualRows, RefPill
│   ├── components/panels/     # CommitDetails, Diff, Staging, RebaseEditor
│   ├── components/overlay/    # CommandPalette, ContextMenu, DropMenu, Toasts
│   ├── stores/
│   ├── bindings.ts            # généré
│   └── styles/tokens.css
└── fixtures/                  # scripts de génération de dépôts de test
```

---

## 6. UX — principes directeurs

1. **Le graph est l'écran.** Pas de tableau de bord, pas d'écran d'accueil superflu : on ouvre un
   dépôt, on voit le graph.
2. **Rapide et prévisible avant d'être joli.** Toute action affiche son effet en < 100 ms ou une
   progression.
3. **Dire ce qui va se passer.** Drag & drop et menus annoncent la commande exacte (« Rebase
   feature/scoring sur main ») ; un survol long affiche la commande git équivalente.
4. **Tout est réversible ou confirmé.** Ctrl+Z par défaut, confirmation seulement pour ce qui ne
   peut pas s'annuler (push, discard).
5. **Clavier d'abord, souris confortable.** Chaque action du menu contextuel existe dans la
   palette.
6. **Densité réglable.** Lignes de 28 px par défaut, mode compact 22 px.

Détails visuels, composants et maquettes : [`design-board.html`](./design-board.html).

---

## 7. Feuille de route

| Phase | Durée indicative | Contenu | Sortie |
|-------|------------------|---------|--------|
| **0 — Spike** | 1–2 sem. | Tauri + `gix` : revwalk + lanes + rendu canvas virtualisé sur un dépôt de 1 M commits ; mesure recherche `nucleo`. | Go/no-go sur les cibles de perf §4.1 |
| **1 — MVP lecture** | 3–4 sem. | F-01..04, F-10..15, F-30..32, F-36, palette, F-40..42, barre latérale, thèmes. | Utilisable en *visualiseur* à côté de la CLI |
| **2 — MVP écriture** | 4 sem. | F-50, F-52, F-60..65, menus contextuels, F-85, F-90..92, F-100..102. | Remplace GitKraken pour la majorité des gestes |
| **3 — V1** | 4–6 sem. | Drag & drop complet, rebase interactif (F-80..86), F-16..19, F-33..34, F-43..44, F-51, F-53..54, onglets, updater, i18n EN. | **Résiliation GitKraken** |
| **4 — V2** | continu | Éditeur de conflits intégré, F-20, F-35, F-45, worktrees, sous-modules, intégration PR GitHub/GitLab (lecture). | — |

Extraction vers un dépôt dédié `Creatiwity/ramure` au démarrage de la phase 1 (le dossier de
l'incubator devient alors un README pointant vers ce dépôt).

---

## 8. Risques et parades

| Risque | Impact | Parade |
|--------|--------|--------|
| `gix` incomplet sur certains dépôts (index v4, sparse, partial clone) | Moyen | Fallback lecture via CLI (§5.2) ; suite de dépôts de test variés en CI. |
| Performance du rendu webview (surtout WebKitGTK sur Linux) | Élevé | Canvas pour le graph, DOM minimal, spike phase 0 sur les 3 OS avant d'engager la suite. |
| Différences de webview entre OS (WebView2, WKWebView, WebKitGTK) | Moyen | Pas d'API web exotique ; tests Playwright + smoke tests manuels par OS à chaque release. |
| Rebase interactif : cas limites (exec, update-refs, conflits en chaîne) | Moyen | S'appuyer sur git lui-même (sequence editor) plutôt que réimplémenter ; bandeau d'état universel. |
| Undo trompeur après push ou modifications externes | Élevé | Règles de refus explicites (F-102) et tests dédiés. |
| Signature de code (macOS notarization, Windows) | Faible/moyen | Certificats à prévoir avant la V1 ; builds CI GitHub Actions `tauri-action`. |
| Effort de maintenance interne | Moyen | Périmètre strict (§1.2) ; ouverture en open source envisageable pour mutualiser. |

---

## 9. Critères d'acceptation du MVP

- [ ] Ouvrir le dépôt de référence 100 k commits : graph visible en < 1 s à chaud, défilement 60 fps
      sur les 3 OS.
- [ ] Taper `pndr retard` dans la recherche surligne le bon commit en < 16 ms par frappe.
- [ ] Les couleurs de lanes restent stables après un fetch et après redémarrage.
- [ ] Commit, amend, checkout, création/suppression de branche, fetch/pull/push, merge, rebase,
      cherry-pick, reset, stash fonctionnent et respectent les hooks et la signature configurés.
- [ ] Un rebase lancé en CLI est détecté et pilotable (continuer/annuler) depuis Ramure.
- [ ] Ctrl+Z annule un reset hard, un rebase et une suppression de branche, et refuse avec un
      message clair quand ce n'est pas sûr.
- [ ] Aucune requête réseau hors commandes git et vérification de mise à jour (vérifié au proxy).

---

## 10. Questions ouvertes

1. **Licence** : outil interne ou open source dès le départ (MIT/Apache-2.0) ? L'open source aide
   à la maintenance mais impose un minimum de soin sur la doc et les issues.
2. **Intégration PR** GitHub/GitLab : utile en V2 (voir les PR sur les branches du graph) ou hors
   périmètre définitif ?
3. **Profils git** (identités multiples perso/pro par dépôt) : besoin réel dans l'équipe ?
4. **Nom** : « Ramure » est un nom de code ; vérifier la disponibilité avant publication.
5. Faut-il un **mode « terminal intégré »** ou renvoyer vers le terminal du système (ouvrir ici) ?
