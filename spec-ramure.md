# Ramure : cahier des charges

> Version 0.4 · Statut : **spécification** (draft à challenger)
> Planche design : [`design-board.html`](./design-board.html) · Lisibilité du graph : [`ux-graph.md`](./ux-graph.md)

### Changements depuis la v0.3

- **Socle du niveau 1** : le terminal par défaut du système (Terminal.app, Windows Terminal,
  terminal par défaut sous Linux) est toujours disponible, et toute commande s'exécute dans le
  **shell interactif de l'utilisateur**, avec sa config (zsh + oh-my-zsh, alias, PATH, agent
  SSH), comme dans Warp ou iTerm2 (F-108, F-109).
- Warp évalué : pas d'API pour écrire dans un onglet existant, intégration limitée à
  l'ouverture d'un onglet.

### Changements depuis la v0.2

- **Niveaux d'intégration du terminal** (§3.7) : niveau 0 = copier la commande ; niveau 1 =
  Ramure envoie la commande dans un terminal externe choisi par l'utilisateur parmi les
  intégrations supportées (tmux, iTerm2, Terminal.app, WezTerm, kitty…), et ⌥ (Alt) copie à la
  place. Toujours pas de terminal intégré.
- Question « exécution directe » (§10) tranchée par ces niveaux.

### Changements depuis la v0.1

- **Open source** dès le départ.
- Positionnement resserré : **viewer git** avec un peu de sucre. Aucune intégration de service
  externe, jamais.
- **Ramure n'exécute aucune commande qui modifie le dépôt** (phase initiale) : il **génère des
  commandes git natives** à coller dans le terminal. Plus de terminal intégré, plus de gestion
  d'identifiants, plus de journal d'opérations maison.
- Identités git : lecture de la config existante uniquement ; on encourage le standard
  `includeIf`.
- Staging et commit sortis du périmètre (IDE ou terminal).
- Nouveau : **branches empilées** et `git rebase --onto` assisté.
- Vue du graph revue (refs | graph | messages alignés), voir `ux-graph.md`.

---

## 1. Contexte

L'équipe utilise GitKraken essentiellement pour **visualiser le commit graph** et pour quelques
gestes rapides : recherche instantanée, rebase (interactif ou non), commandes git courantes par
**drag & drop** ou **menu contextuel**. Le reste de l'offre n'est pas utilisé, alors que
l'abonnement est facturé par siège, chaque année.

**Ramure** (nom de code) est un **viewer git** de bureau, open source, autonome et hors ligne,
construit avec **Tauri 2 + Rust + Vue 3**. Il affiche le graph le plus lisible possible et
transforme les gestes (glisser, menu, palette) en **commandes git natives prêtes à coller**.

### 1.1 Objectifs

| # | Objectif | Mesure de succès |
|---|----------|------------------|
| O1 | Remplacer GitKraken pour l'usage quotidien de l'équipe | 100 % de l'équipe sur Ramure 1 mois après la V1, abonnement résilié |
| O2 | Graph plus lisible que GitKraken | Tâches du test utilisateur (`ux-graph.md` §5) au moins aussi rapides, T2 plus rapide |
| O3 | Graph plus rapide que GitKraken | Ouverture d'un dépôt de 100 k commits : graph affiché en < 1 s (à chaud) |
| O4 | Recherche « au fil de la frappe » | < 16 ms par frappe sur 100 k commits |
| O5 | Zéro risque pour le dépôt | Ramure n'écrit jamais dans le dépôt ; chaque commande générée affiche sa commande d'annulation |
| O6 | Application légère | Installeur < 20 Mo, RAM < 300 Mo sur 100 k commits |

### 1.2 Principes structurants

1. **Viewer d'abord.** La valeur principale est de *voir* et de *comprendre* l'historique.
2. **Git natif, rien d'autre.** Ramure ne connaît que le dépôt local et la config git. Pas
   d'API GitHub/GitLab/Bitbucket, pas de compte, pas de cloud, pas de télémétrie, pas d'avatar
   distant. Ramure ne fait aucune requête réseau (même `fetch` est une commande à copier).
3. **Lecture seule sur le dépôt.** Les opérations sont des **commandes générées** qui
   s'exécutent toujours dans le terminal de l'utilisateur, avec son contexte (shell, identité,
   agent SSH, hooks, signature) : copiées-collées (niveau 0) ou envoyées par Ramure dans le
   terminal choisi (niveau 1, §3.7). Ramure ne lance jamais `git` en écriture lui-même et détecte
   le résultat grâce au watcher.
4. **Réutiliser le standard.** Identités, signature, alias, `rerere`, `pull.rebase` : tout vient
   de la config git ; Ramure l'affiche, ne la duplique pas.

### 1.3 Hors périmètre (explicitement et durablement)

- Intégrations de services : PR/MR, issues, CI, forges, avatars Gravatar.
- Terminal intégré, et exécution de commandes d'écriture par Ramure lui-même (hors du terminal
  de l'utilisateur).
- Gestion d'identifiants (credential helper, SSH) et de profils git.
- Staging, commit, édition de fichiers, résolution de conflits intégrée.
- Fonctions IA, synchronisation de préférences, VCS autres que git.

### 1.4 Licence et gouvernance

- Licence : **MIT OR Apache-2.0** (double licence, convention de l'écosystème Rust et de Tauri).
- Dépôt public `Creatiwity/ramure` dès l'extraction de l'incubator (début de phase 1), avec
  `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, modèles d'issues, CI publique.
- Builds signés et publiés via GitHub Releases ; mises à jour via le plugin updater de Tauri
  (seule requête réseau de l'application, désactivable, et désactivée dans les builds distribués
  par des gestionnaires de paquets).
- Les intégrations de terminaux (§3.7) sont des modules indépendants, pour faciliter les
  contributions externes (« ajouter mon terminal »).

---

## 2. Utilisateurs et cas d'usage

**Persona principal : développeur·se de l'équipe**, à l'aise avec git en ligne de commande, qui
veut *voir* l'historique et obtenir vite la bonne commande, sans risque d'erreur de syntaxe ni de
cible.

1. **Comprendre l'état d'un dépôt** : où sont mes branches par rapport à `main`, `develop` et
   `origin` ? Qu'est-ce qui a été mergé ? Qui a touché quoi ?
2. **Retrouver un commit** en quelques frappes (message, sha, auteur, branche, `#142`).
3. **Rebaser ma branche** : glisser `feature/x` sur `develop` → commande copiée.
4. **Rebaser une branche empilée** après la réécriture de sa base :
   `git rebase --onto develop feat/a~3 feat/a`, sans compter les commits à la main.
5. **Nettoyer ma branche avant PR** : rebase interactif édité visuellement, puis une commande à
   coller.
6. **Gestes courants** : checkout, créer/supprimer une branche, cherry-pick, reset, revert,
   stash, fetch/pull/push, toujours sous forme de commande.
7. **Suivre une opération en cours** (rebase, merge, cherry-pick) lancée dans le terminal :
   étape, fichiers en conflit, commande pour continuer ou annuler.
8. **Annuler** : copier la commande qui remet la branche où elle était.

---

## 3. Fonctionnalités

Priorités **MoSCoW** : **M** = Must (MVP), **S** = Should (V1), **C** = Could (V2+).
Identifiants stables `F-xx` à référencer dans les issues et PR.

### 3.1 Dépôts

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-01 | M | Ouvrir un dépôt local (dialogue, glisser un dossier sur la fenêtre, `ramure <chemin>`). |
| F-02 | M | Dépôts récents, épinglables. |
| F-03 | S | Onglets : plusieurs dépôts dans une fenêtre. |
| F-04 | M | Rafraîchissement automatique (watcher sur `.git/` et l'arbre de travail, debounce). Aucun bouton « Refresh ». |
| F-06 | C | Worktrees : lister, ouvrir dans un onglet. |
| F-07 | C | Sous-modules : état. |

### 3.2 Commit graph (cœur du produit)

Les règles de mise en page et leurs justifications sont dans [`ux-graph.md`](./ux-graph.md) (décisions D1 à D8).

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-10 | M | Trois zones : **refs · graph · texte** (message, auteur, date). Tous les messages commencent au même x (D1). Colonnes redimensionnables et masquables ; sha masqué par défaut. |
| F-11 | M | Pastilles de refs alignées à droite contre le graph, reliées au nœud par un trait de la couleur de la branche ; une pastille visible + compteur `+N` ; locale et distante fusionnées ; troncature au milieu (D1). |
| F-12 | M | Largeur de graph constante sur tout l'historique chargé, plafonnée à 10 lanes puis resserrée (D2). |
| F-13 | M | Lanes *straight branches*, tronc épinglé en colonne 0 (`main`), `develop` en 1, configurable (D3). |
| F-14 | M | Couleur « focus » : branche courante, sa base et la branche survolée en couleurs vives, les autres désaturées ; mode arc-en-ciel en option ; la couleur suit la branche (D4). |
| F-15 | M | Ligne WIP en tête (compteur de fichiers modifiés / indexés), rattachée à `HEAD`. |
| F-16 | M | Virtualisation : 60 fps quel que soit l'historique ; chargement progressif. |
| F-17 | M | Sélection → panneau de détails (§3.4). Sélection multiple → diff cumulé et commandes groupées. |
| F-18 | S | Conventional Commits détectés : type en sous-colonne grise, portée en gris secondaire, sujet aligné (D5). |
| F-19 | S | Merges en forme courte et atténuée (`⤙ fix/pwa-cache  #142`) ; mode first-parent (D5). |
| F-20 | S | Date affichée seulement quand elle change ; auteurs consécutifs identiques atténués (D6). |
| F-21 | S | Mise en évidence de l'ascendance du commit sélectionné, le reste atténué. |
| F-22 | S | Minimap dans la barre de défilement : HEAD, branches locales, tags, résultats de recherche (D7). |
| F-23 | S | Filtres : branches, remotes, tags, stashes, first-parent, « ma branche et sa base » (D7). |
| F-24 | S | Indicateurs ahead/behind (`↑2 ↓5`) par rapport à l'upstream et à la base. |
| F-25 | C | Replier une branche mergée en une ligne (« 12 commits de fix/pwa-cache ») (D7). |
| F-26 | M | Formes distinctes par type de nœud et de pastille, sans dépendre de la couleur (D8). |

**Barre latérale** (M) : *Branches locales · Remotes · Tags · Stashes* (*Worktrees* en C),
repliables, avec filtre texte. Clic = aller au commit.

### 3.3 Recherche instantanée

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-30 | M | Recherche (⌘/Ctrl+F) au fil de la frappe sur message, sha (préfixe), auteur, refs. |
| F-31 | M | Modes **surligner** (graph intact, ↵ / ⇧↵ entre résultats, marques dans la minimap) et **filtrer**. |
| F-32 | M | Correspondance floue (`pndr retard` → « pondère les tâches en retard »), tri pertinence puis date, caractères trouvés surlignés. |
| F-33 | S | Opérateurs combinables : `author:`, `msg:`, `ref:`, `sha:`, `before:`, `after:`, `merge:yes/no`. |
| F-34 | S | `path:src/scoring/` : commits ayant modifié ce chemin, en tâche de fond, résultats en streaming. |
| F-35 | C | `diff:computeUrgency` (équivalent `git log -S/-G`), en tâche de fond, annulable. |
| F-36 | M | Compteur de résultats et durée (« 4 résultats · 2 ms »). |

**Palette de commandes** (M, ⌘/Ctrl+K) : même moteur, portée élargie aux branches, tags,
fichiers et **actions** (« Rebase interactif… », « Fetch all »…). Chaque action aboutit à une
commande générée (§3.6–3.7).

### 3.4 Détails de commit et diff (lecture)

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-40 | M | Message complet, auteur/committer, dates, sha (copie), parents cliquables, refs, signature vérifiée ou non. |
| F-41 | M | Fichiers modifiés (A/M/D/R, +/−), en arbre ou à plat. |
| F-42 | M | Diff unifié et côte à côte, coloration syntaxique, repli des zones inchangées, ignorer les espaces. |
| F-43 | S | Diff entre deux commits quelconques, ou entre un commit et l'arbre de travail. |
| F-44 | S | Historique et blame d'un fichier. |
| F-45 | M | Changements non commités (WIP) consultables en lecture : fichiers et diff, indexés ou non. |
| F-46 | C | Diff d'images. |

### 3.5 Identité git (lecture seule)

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-50 | M | Afficher l'identité effective du dépôt (`user.name`, `user.email`, clé de signature) et **son origine** (`git config --show-origin`) dans la barre d'outils. |
| F-51 | S | Avertir si l'identité effective ne correspond pas à la règle attendue (ex. e-mail pro dans `~/code/creatiwity/`), avec un lien vers la documentation `includeIf` et un **extrait de config** proposé à copier (`[includeIf "gitdir:~/code/creatiwity/"] path = ~/.gitconfig-creatiwity`). Ramure n'écrit jamais la config. |

### 3.6 Commandes générées : le principe

Toute action d'écriture (menu contextuel, drag & drop, palette, bouton) ouvre une **fiche de
commande** au lieu d'agir :

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-60 | M | **Commande git native**, colorée (refs dans la couleur de leur branche), copiable en un clic (⌘/Ctrl+C) ou en ↵ depuis la fiche, ou exécutée dans le terminal choisi au niveau 1 (§3.7). Plusieurs étapes enchaînées par `&&`. |
| F-61 | M | **Explication** ligne à ligne des arguments (« `--onto develop` : nouvelle base = develop actuel, 9c2e7f1 »). |
| F-62 | M | **Aperçu** avant/après du graph concerné (mini-graph). |
| F-63 | M | **Commande d'annulation** calculée au moment de la génération, avec des sha explicites (`git reset --keep 7a1e0c4`, `git branch -f feat/a 7a1e0c4`). Conservée dans l'historique des commandes. |
| F-64 | M | Options d'écriture : refs relatives (`feat/a~3`) ou sha explicites ; préfixe `git -C <chemin>` pour éviter de coller dans le mauvais dossier ; dialecte de shell (sh/bash/zsh, fish, PowerShell) pour la citation. |
| F-65 | M | **Sûreté par défaut** : `--force-with-lease` (jamais `--force` nu), `reset --keep` plutôt que `--hard` quand c'est équivalent, avertissement explicite sur ce qui ne peut pas s'annuler (modifications non commitées). |
| F-66 | M | **Détection de l'exécution** : le watcher voit les refs bouger et affiche « Commande exécutée : feat/a a été déplacée » avec la commande d'annulation. Si l'état a changé avant l'exécution (refs déplacées), la fiche se marque « périmée » et propose de régénérer. |
| F-67 | S | Historique des commandes générées (session), avec leur statut (copiée, exécutée détectée) et leur annulation. |
| F-68 | S | Survol long d'une entrée de menu : la commande s'affiche en infobulle. |

### 3.7 Niveaux d'intégration du terminal

Ramure n'a pas de terminal intégré : il **s'appuie sur celui que l'utilisateur utilise déjà**. Le
niveau se règle par dépôt (ou globalement) et change le comportement des boutons des fiches de
commande, menus, palette et drag & drop.

| Niveau | Nom | Clic / ↵ | ⌥ clic / ⌥↵ |
|--------|-----|----------|-------------|
| **0** | Copier | copie la commande | — |
| **1** | Terminal connecté | **envoie la commande dans le terminal choisi et l'exécute** | copie la commande (comme au niveau 0) |

Au niveau 1, maintenir ⌥ (Alt) bascule visuellement tous les boutons en « Copier » : le libellé
change tant que la touche est enfoncée, pour que l'utilisateur sache ce que fera son clic.

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-100 | M | Niveau 0 : copie (comportement de §3.6). Disponible partout, sans configuration. |
| F-101 | S | Réglage « Intégration terminal » : choix du terminal parmi les intégrations **détectées** sur la machine, avec leur état (disponible, permission requise, non configuré). |
| F-102 | S | Choix de la **cible** : session / onglet / pane existant, listé avec son dossier courant et le programme au premier plan. Ramure présélectionne la cible dont le dossier est le dépôt ouvert. Option « Ouvrir un nouvel onglet dans le dépôt » quand aucune cible ne convient. Cible mémorisée par dépôt. |
| F-103 | S | Envoi : la commande est tapée dans la cible puis validée (Entrée). Toujours préfixée par `git -C '<dépôt>'` si le dossier courant de la cible n'est pas le dépôt (ou si l'intégration ne sait pas le dire). |
| F-104 | S | **Garde-fous** : pas d'envoi si la cible n'a pas un shell au premier plan (vim, less, un process en cours…) ; message « iTerm2 · onglet 2 est occupé (vim) » et copie proposée. Les commandes **non annulables** (reset hard avec fichiers modifiés, discard, push force-with-lease) sont **préremplies sans Entrée** : l'utilisateur valide dans son terminal. |
| F-105 | S | Retour : Ramure ne lit pas la sortie du terminal ; il confirme via le watcher (« Commande exécutée : feat/a déplacée ») comme au niveau 0 et propose l'annulation (envoyée ou copiée selon ⌥). |
| F-106 | S | Indicateur permanent dans la barre d'outils : « ⌘ iTerm2 · sherpa (onglet 2) », clic = changer de cible, ⌥ clic = revenir au niveau 0 pour la session. |
| F-107 | C | Dialecte de shell déduit de la cible (programme au premier plan) plutôt que du réglage. |
| F-108 | M¹ | **Shell de l'utilisateur, config complète.** Quand Ramure ouvre un onglet ou lance une commande, il utilise le shell par défaut de l'utilisateur (`$SHELL`, ou le profil par défaut du terminal) en mode **interactif et login**, pour charger exactement la même config que dans Warp ou iTerm2 : `.zprofile`, `.zshrc`, oh-my-zsh et ses plugins, alias, `PATH` (nvm, asdf, Homebrew…), agent SSH, GPG. Jamais de `sh -c` nu. Une fois la commande terminée, le shell reste ouvert pour la suite : `zsh -l -i -c '<commande>; exec zsh -l -i'`. |
| F-109 | M¹ | **Terminal de base toujours disponible** : le terminal par défaut du système, sans installation ni configuration. macOS : Terminal.app. Windows : Windows Terminal (repli sur PowerShell / `cmd`). Linux : terminal par défaut (`x-terminal-emulator`, `gnome-terminal`, `konsole`, `xdg-terminal-exec`). C'est la première intégration livrée. |

¹ Obligatoire dès que le niveau 1 est livré (V1) : le niveau 1 ne sort pas sans ce socle.

**Intégrations prévues** (chaque intégration déclare ce qu'elle sait faire : lister les cibles,
connaître le dossier courant et le programme au premier plan, envoyer du texte, valider, ouvrir
un onglet) :

| Terminal | OS | Mécanisme | Lister / cwd | Envoyer | Ouvrir un onglet | Prio |
|----------|----|-----------|--------------|---------|------------------|------|
| **Terminal par défaut du système (socle, F-109)** | tous | macOS : AppleScript Terminal.app (`do script` dans l'onglet du dépôt ou un nouvel onglet). Windows : `wt.exe -w 0 nt -d <dépôt> <shell>`. Linux : `gnome-terminal --working-directory=<dépôt> -- $SHELL -l -i -c '…'`, `konsole --workdir`, `x-terminal-emulator -e` | macOS : non ; autres : non | macOS : oui ; autres : nouvel onglet ou fenêtre | oui | **M¹** |
| tmux | macOS, Linux | `tmux list-panes -a -F …`, `tmux send-keys -t <pane> -l '<cmd>'` puis `Enter` | oui (`pane_current_path`, `pane_current_command`) | oui | `tmux new-window -c <dépôt>` | S |
| WezTerm | tous | `wezterm cli list --format json`, `wezterm cli send-text --pane-id` | oui | oui | `wezterm cli spawn --cwd` | S |
| kitty | macOS, Linux | remote control (`kitty @ ls`, `kitty @ send-text`), à activer par l'utilisateur | oui | oui | `kitty @ launch --cwd` | S |
| iTerm2 | macOS | AppleScript (`write text` dans une session) ; permission Automation macOS | partiel (via variables de session) | oui | oui | S |
| Terminal.app | macOS | AppleScript (`do script … in` un onglet) ; permission Automation | non | oui | oui | S |
| Windows Terminal | Windows | `wt.exe -w 0 nt -d <dépôt>` : ne sait pas écrire dans un onglet existant | non | nouvel onglet uniquement | oui | S |
| Konsole | Linux | D-Bus (`runCommand` sur une session) | partiel | oui | oui | C |
| Warp | macOS, Linux, Windows | Pas d'API d'envoi (ni AppleScript, ni CLI) ; Ramure écrit une *launch configuration* temporaire (`cwd` + `exec`) et l'ouvre via `warp://launch/<chemin>` ; `warp://action/new_tab?path=` pour un onglet vide | non | nouvel onglet uniquement | oui | C |
| Autres (Ghostty, GNOME Terminal, terminal de VS Code…) | — | selon les API disponibles ; à défaut, niveau 0 | — | — | — | C |

Pour les terminaux sans API d'envoi (Windows Terminal, terminaux Linux de base, Warp), le
niveau 1 se limite à « ouvrir un onglet dans le dépôt et y lancer la commande », signalé comme
tel dans le réglage. Dans tous les cas, la commande tourne dans le shell de l'utilisateur avec sa
config complète (F-108) : l'environnement est le même que s'il avait ouvert l'onglet lui-même.

### 3.8 Opérations couvertes (toutes en commandes générées)

| ID | Prio | Opération | Exemple de commande |
|----|------|-----------|---------------------|
| F-70 | M | Checkout d'une branche, d'un commit (detached), d'une distante (crée la locale de suivi) | `git switch feature/scoring` |
| F-71 | M | Créer / renommer / supprimer une branche (locale, distante) | `git branch fix/x d88e1b4` |
| F-72 | M | Fetch, pull, push (upstream au premier push, force-with-lease si divergente) | `git push --force-with-lease origin feature/scoring` |
| F-73 | M | Merge, rebase, cherry-pick, revert, reset (soft / mixed / keep / hard) | `git rebase develop` |
| F-74 | M | Tags : créer, supprimer, pousser | `git tag -a v0.4.1 2aa90d3 -m "v0.4.1"` |
| F-75 | M | Stash : créer, appliquer, pop, supprimer | `git stash push -u -m "essai layout"` |
| F-76 | S | Commit `fixup!` ciblant un commit sélectionné (le commit est fait par l'utilisateur) | `git commit --fixup=e27d410` |
| F-77 | M | Commandes pour une opération en cours : continuer, passer, annuler, prendre *ours*/*theirs* sur un fichier | `git checkout --theirs -- path && git add path` |

### 3.9 Branches empilées et `rebase --onto` assisté

Cas d'usage : `feat/a` est partie de `develop` ; `develop` a été réécrite (rebase sur `main`,
amend). `feat/a` repose désormais sur des commits qui n'existent plus dans `develop`. Un simple
`git rebase develop` rejouerait ces anciens commits et produirait des conflits s'ils ont été
modifiés.

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-80 | S | **Détection** : pour chaque branche locale et sa base (upstream configurée, ou `develop`/`main`), calcul du point de départ réel via le reflog de la base (`git merge-base --fork-point`) et l'équivalence de patchs (`git cherry`). Les anciens commits de la base apparaissent en pointillé gris « ancienne version de develop ». |
| F-81 | S | Badge « base réécrite » sur la pastille de la branche, et suggestion en un clic : `git rebase --onto develop feat/a~3 feat/a`, avec le nombre de commits propres calculé et vérifiable dans l'aperçu. |
| F-82 | S | **Geste manuel** : sélectionner une plage de commits d'une branche (clic puis ⇧clic), la glisser sur la nouvelle base → `git rebase --onto <cible> <premier>~1 <branche>`. |
| F-83 | S | **Pile de branches** (`feat/a` ← `feat/b` ← `feat/c`) : proposer `--update-refs` pour déplacer toute la pile en une commande (`git rebase --update-refs develop feat/c`). |

### 3.10 Rebase interactif

L'éditeur visuel produit **une seule commande** qui applique la todo préparée, sans éditeur
interactif :

```sh
git -c sequence.editor="cp '/tmp/ramure/todo-7f3a'" rebase -i --autosquash main
```

git invoque l'éditeur de séquence via son propre `sh` (y compris Git for Windows) : `cp`
remplace la todo par celle de Ramure. Les messages *reword* et *squash* sont appliqués par des
lignes `exec git commit --amend --only -F '<fichier>'` ajoutées à la todo. Les fichiers
temporaires sont écrits dans le dossier temporaire de l'application, jamais dans le dépôt.

| ID | Prio | Fonctionnalité |
|----|------|----------------|
| F-90 | S | Éditeur de todo : action par ligne (`pick`, `reword`, `edit`, `squash`, `fixup`, `drop`), réordonnancement par glisser ou ⌥↑/↓, raccourcis `p r e s f d`. |
| F-91 | S | Reword en ligne ; message combiné pour `squash`. |
| F-92 | S | Aperçu du résultat à droite ; autosquash des `fixup!`/`squash!`. |
| F-93 | S | Génération de la commande (ci-dessus) avec commande d'annulation (`git reset --keep <sha d'origine>`). |
| F-94 | M | Bandeau « Rebase en cours — 3/7 » (quel que soit l'outil qui l'a lancé), fichiers en conflit, et commandes `--continue` / `--skip` / `--abort` à copier. |

### 3.11 Drag & drop et menu contextuel

Même matrice qu'en v0.1, chaque action aboutissant à une fiche de commande (F-60) :

| Source → Cible | Actions proposées (ordre) |
|----------------|---------------------------|
| Branche A → branche B | Rebase A sur B · Merge B dans A · Merge A dans B · Fast-forward B vers A *(si possible)* |
| Branche A → commit C | Rebase A sur C · Reset A à C |
| Plage de commits de A → branche/commit B | `rebase --onto` (F-82) |
| Branche A → sa distante | Push (force-with-lease si divergente) |
| Commit(s) → branche B | Cherry-pick sur B |
| Commit C → commit D (même branche) | Rebase interactif pré-rempli (déplacer C après D) |

Si l'action demande un checkout préalable, la commande l'enchaîne et le libellé l'annonce :
« Checkout main puis merge feature/scoring ».

**Menu contextuel** d'un commit : checkout · créer une branche ici · créer un tag ici ·
cherry-pick · revert · reset *branche courante* ici ▸ · rebase interactif depuis ici ·
`commit --fixup` pour celui-ci · copier le sha / le message · comparer avec l'arbre de travail.
D'une pastille : checkout · merge dans courante · rebase courante sur celle-ci · rebase `--onto`… ·
push / pull · renommer · supprimer · copier le nom.

### 3.12 Clavier

| Raccourci | Action |
|-----------|--------|
| ⌘K | Palette de commandes |
| ⌘F | Recherche dans le graph |
| ↑ / ↓ · J / K | Commit précédent / suivant |
| ← / → | Premier parent / premier enfant |
| H | Aller à `HEAD` |
| Espace | Panneau de détails |
| ↵ (fiche de commande ouverte) | Niveau 0 : copier · niveau 1 : exécuter dans le terminal choisi |
| ⌥↵ · ⌥ clic | Copier au lieu d'exécuter (niveau 1) |
| ⌘C (fiche ouverte) | Copier, quel que soit le niveau |
| ⌘Z | Annulation de la dernière commande exécutée (copiée ou envoyée selon le niveau) |
| ⌘B | Branche au commit sélectionné |
| ⌘1..9 | Onglet de dépôt n |

### 3.13 Préférences

M : thème, police et taille, format de date, éditeur externe (ouvrir un fichier), niveau
d'intégration et terminal cible (S), dialecte de shell, préfixe `git -C`, refs relatives ou sha, branches de tronc, langue (FR/EN).
S : raccourcis, couleurs des lanes, mode couleur par défaut (focus / arc-en-ciel).

---

## 4. Exigences non fonctionnelles

### 4.1 Performance (référence : 100 k commits, 2 k refs ; stress : 1 M commits)

| Indicateur | Cible 100 k | Cible 1 M |
|------------|-------------|-----------|
| Premier affichage (à chaud) | < 1 s | < 2 s |
| Premier affichage (à froid) | < 2 s | < 5 s, UI utilisable pendant le streaming |
| Recherche par frappe | < 16 ms | < 60 ms |
| Défilement | 60 fps | 60 fps |
| Mise à jour après une commande exécutée dans le terminal | < 150 ms | < 400 ms |
| Mémoire | < 300 Mo | < 1,2 Go |

Benchmark automatisé en CI sur des dépôts de référence ; une régression > 20 % bloque la release.

### 4.2 Plateformes

macOS (≥ 12, Apple Silicon et Intel), Windows 10/11 (x64, arm64), Linux (AppImage, .deb, X11 et
Wayland). git ≥ 2.38 requis (pour `--update-refs`), vérifié au démarrage.

### 4.3 Sécurité et confidentialité

- **Aucune écriture dans le dépôt**, aucune exécution de commande d'écriture git.
- **Aucune requête réseau**, hors vérification de mise à jour (désactivable).
- Aucun identifiant manipulé ni stocké.
- Tauri : capabilities minimales, pas d'accès shell depuis le front, CSP stricte. Processus
  lancés : `git` en lecture (`log`, `cherry`, `merge-base`, `config --show-origin`) quand `gix`
  ne suffit pas, et, au niveau 1 seulement, les outils de pilotage du terminal choisi (`tmux`,
  `wezterm cli`, `kitty @`, `osascript`, `wt.exe`, D-Bus) via une liste blanche côté Rust.
- Au niveau 1, la commande passe par l'outil de pilotage sous forme d'argument (jamais
  interpolée dans un script) ; les scripts AppleScript sont fixes et reçoivent la commande en
  paramètre.
- Ramure n'écrit toujours rien lui-même : même au niveau 1, c'est le shell de l'utilisateur qui
  exécute la commande, dans sa session.
- Commandes générées : citation stricte des arguments selon le dialecte de shell (noms de
  branches avec caractères spéciaux), testée par des tests de propriété.

### 4.4 Accessibilité et i18n

Navigation clavier complète, focus visible, contrastes WCAG AA dans les deux thèmes, formes en
plus des couleurs (D8), palette vérifiée en daltonisme. FR et EN dès le MVP.

---

## 5. Architecture technique

### 5.1 Vue d'ensemble

```
┌──────────────────────── Webview (Vue 3 + TS) ─────────────────────────┐
│  RefsColumn │ GraphCanvas │ VirtualRows │ Panels · Palette · CmdSheet │
│            ▲ Pinia stores (repo, graph, search, commands, ui)         │
└────────────┼──────────────────────────────────────────────────────────┘
             │ Tauri IPC : commands typées (tauri-specta) + Channels (streaming)
┌────────────┼──────────────────── Core Rust ───────────────────────────┐
│  repo::    gix en lecture (revwalk, refs, objets, status, diff, config)│
│  graph::   topo-order, lanes straight-branches, couleurs, chunks       │
│  stack::   fork-point, équivalence de patchs, piles de branches        │
│  search::  index colonnaire + nucleo + rayon                           │
│  cmdgen::  construction des commandes, citation par shell, undo        │
│  term::    intégrations terminal (tmux, WezTerm, kitty, iTerm2…)      │
│  watch::   notify (debounce) → invalidation incrémentale, détection    │
└───────────────────────────────────────────────────────────────────────┘
```

### 5.2 Accès git

| Besoin | Choix | Raison |
|--------|-------|--------|
| Historique, refs, objets, diffs, status, config | **gitoxide (`gix`)** | Pur Rust, rapide, multi-thread ; utilise le fichier `commit-graph`. |
| Fork-point, `cherry`, cas non couverts par `gix` | binaire `git` en lecture | Même résultat que la CLI de l'utilisateur. |
| Écriture | **aucune** : `cmdgen` produit du texte | Contexte d'exécution = le terminal de l'utilisateur. |

### 5.3 Graph

- Ordre `--topo-order` départagé par date ; lanes *straight branches* (réf. pvigier) avec tronc
  épinglé ; couleur par branche logique (hash stable du nom de ref la plus proche).
- Transmission au front en tableaux compacts par tranches de 5 000 lignes (lane, couleur,
  segments d'arêtes, indicateurs de refs) ; texte à la demande pour la fenêtre visible.
- Rendu : colonne refs et texte en DOM virtualisé, graph en `<canvas>` ; hauteur de ligne fixe
  (28 px, compact 22 px) et scroll partagé. Largeur de graph constante (D2).
- Recalcul incrémental au-dessus du plus ancien commit modifié.

### 5.4 Recherche

Index colonnaire en mémoire (~100 octets/commit), `nucleo` + `rayon`, annulation à chaque
frappe ; `path:` et `diff:` streamés en tâche de fond.

### 5.5 Génération de commandes (`cmdgen`)

- Chaque action est un type Rust (`Rebase { branch, onto, upstream, interactive, update_refs }`,
  …) qui produit : la commande (liste d'arguments, jamais une chaîne concaténée), son
  explication, l'aperçu attendu (refs après exécution), la commande d'annulation et les
  préconditions (refs attendues à leurs sha actuels).
- Rendu texte par dialecte de shell (sh, fish, PowerShell) au dernier moment.
- **Détection** : le watcher compare l'état des refs aux préconditions et au résultat attendu →
  statut « exécutée » ou « périmée ».
- Tests : chaque commande générée est exécutée en CI sur des dépôts fixtures et le graph obtenu
  est comparé à l'aperçu annoncé.

### 5.6 Intégrations terminal (`term::`)

Chaque intégration implémente un trait commun, et déclare ses capacités pour que l'UI
n'affiche que ce qui marche :

```rust
trait TerminalBackend {
    fn id(&self) -> &'static str;                   // "tmux", "iterm2", …
    fn detect(&self) -> Availability;               // installé, permission requise, à configurer
    fn capabilities(&self) -> Caps;                 // LIST, CWD, FOREGROUND, SEND, OPEN_TAB
    fn targets(&self) -> Result<Vec<Target>>;       // id, libellé, cwd?, programme au premier plan?
    fn send(&self, t: &TargetId, cmd: &str, submit: bool) -> Result<()>;
    fn open_tab(&self, cwd: &Path, cmd: Option<&str>) -> Result<TargetId>;
}
```

- `submit = false` sert au préremplissage des commandes non annulables (F-104).
- Envoi en *bracketed paste* quand l'outil le permet (WezTerm par défaut, `tmux send-keys -l`),
  pour que le shell ne l'interprète pas caractère par caractère.
- Tests : backend `tmux` testé en CI (Linux) de bout en bout ; les autres via des mocks des
  outils de pilotage, plus une checklist manuelle par release.

### 5.7 Front

Vue 3 (Composition API) + TypeScript strict + Vite, Pinia, bindings générés par `tauri-specta`,
CodeMirror 6 pour les diffs, drag & drop maison sur Pointer Events, composants maison sur tokens
CSS. Tests : Vitest, Playwright sur le build web avec backend mocké.

### 5.8 Arborescence cible

```
ramure/
├── src-tauri/src/
│   ├── main.rs
│   ├── repo/  graph/  stack/  search/  cmdgen/  term/
│   ├── watch.rs
│   └── ipc.rs
├── src-tauri/benches/
├── src/components/{graph,panels,overlay}/
├── src/stores/  src/bindings.ts  src/styles/tokens.css
└── fixtures/              # scripts de dépôts de test (piles, rebases, merges)
```

---

## 6. UX : principes directeurs

1. **Le graph est l'écran**, et il est lisible avant d'être dense (`ux-graph.md`).
2. **Dire exactement ce qui va se passer** : chaque geste aboutit à une commande lisible,
   expliquée, avec son aperçu.
3. **Toujours une sortie** : chaque commande vient avec sa commande d'annulation.
4. **Clavier d'abord** : tout geste existe dans la palette.
5. **Aucune surprise** : rien ne touche le dépôt sans que l'utilisateur colle la commande.

---

## 7. Feuille de route

| Phase | Durée indicative | Contenu | Sortie |
|-------|------------------|---------|--------|
| **0 — Spike** | 1–2 sem. | `gix` + lanes + rendu canvas virtualisé sur 1 M commits ; prototype de la mise en page refs · graph · texte ; mesure `nucleo`. | Go/no-go perf + test utilisateur rapide (`ux-graph.md` §5) |
| **1 — Viewer** | 3–4 sem. | F-01..04, F-10..17, F-26, F-30..32, F-36, palette, F-40..42, F-45, F-50, barre latérale, thèmes. Extraction en dépôt public. | Utilisable en viewer à côté du terminal |
| **2 — Commandes** | 3–4 sem. | F-60..66, F-70..75, F-77, menus contextuels, drag & drop de base, F-94. | **Remplace GitKraken** pour la majorité des gestes |
| **3 — V1** | 5 sem. | F-80..83 (`--onto`, piles), F-90..93 (rebase interactif), niveau 1 du terminal (F-101..106, F-108..109) : terminal par défaut du système d'abord, puis tmux, WezTerm, kitty, iTerm2, F-18..24, F-33..34, F-43..44, F-51, F-67..68, onglets, updater, EN. | **Résiliation GitKraken**, release publique 1.0 |
| **4 — Ensuite** | continu | F-25, F-35, F-46, F-107, worktrees, sous-modules, intégrations terminal supplémentaires (Konsole, Ghostty…). | — |

---

## 8. Risques et parades

| Risque | Impact | Parade |
|--------|--------|--------|
| Coller une commande dans le mauvais dossier ou sur un état qui a changé | Élevé | Option `git -C <chemin>` ; sha explicites en option ; fiche marquée « périmée » quand les refs ont bougé ; commande d'annulation toujours fournie. |
| Friction du copier-coller pour les gestes très fréquents (checkout, fetch) | Moyen | Niveau 1 : envoi direct dans le terminal choisi (§3.7). |
| Commande envoyée dans la mauvaise cible, ou pendant que l'utilisateur tape | Élevé | Cible affichée en permanence (F-106), `git -C` automatique, refus si un programme occupe la cible, préremplissage sans Entrée pour le non annulable (F-104). |
| API des terminaux instables ou absentes (AppleScript, remote control désactivé) | Moyen | Capacités déclarées par intégration, détection au démarrage, repli sur le niveau 0 avec message explicite. |
| Permissions macOS (Automation) refusées | Faible | Explication dans le réglage et bouton vers les préférences système ; repli niveau 0. |
| Citation incorrecte sous PowerShell ou fish | Moyen | Arguments structurés + tests de propriété par dialecte. |
| Détection de fork-point faussée (reflog expiré, branche clonée récemment) | Moyen | Croiser fork-point et équivalence de patchs ; toujours montrer l'aperçu ; ne jamais proposer sans aperçu vérifiable. |
| `gix` incomplet sur certains dépôts | Moyen | Fallback lecture via la CLI ; fixtures variées en CI. |
| Performance du rendu (surtout WebKitGTK) | Élevé | Canvas pour le graph, DOM minimal, spike sur les 3 OS. |
| Maintenance d'un projet open source | Moyen | Périmètre strict (§1.3) ; contributions externes bienvenues. |

---

## 9. Critères d'acceptation

**MVP viewer (fin de phase 1)**

- [ ] Dépôt de 100 k commits : graph en < 1 s à chaud, 60 fps sur les 3 OS.
- [ ] Tous les messages commencent au même x, quelles que soient les refs et la position de défilement.
- [ ] `pndr retard` surligne le bon commit en < 16 ms par frappe.
- [ ] Couleurs de lanes stables après un fetch et après redémarrage.
- [ ] Identité effective et son origine affichées.
- [ ] Aucune écriture dans le dépôt, aucune requête réseau (vérifié par un test d'intégration et au proxy).

**Commandes (fin de phase 2)**

- [ ] Chaque opération F-70..77 produit une commande qui, exécutée sur les fixtures, donne exactement le graph annoncé dans l'aperçu.
- [ ] Chaque commande a une commande d'annulation qui restaure les refs d'origine.
- [ ] Un rebase lancé à la main est détecté, avec les commandes continuer/passer/annuler.
- [ ] Citation correcte sous bash, zsh, fish et PowerShell pour des noms de branche avec espaces, guillemets et caractères non ASCII.

**V1**

- [ ] Niveau 1 avec le terminal par défaut du système sur les 3 OS : la commande s'exécute dans le shell de l'utilisateur avec sa config (vérifié avec zsh + oh-my-zsh : un alias défini dans `.zshrc` est disponible dans l'onglet ouvert).
- [ ] Niveau 1 avec tmux et iTerm2 : un clic exécute la commande dans la cible choisie, ⌥ clic la copie, et le libellé des boutons change pendant l'appui sur ⌥.
- [ ] Une cible occupée (vim ouvert) n'est jamais écrite ; une commande non annulable est préremplie sans être validée.
- [ ] Sur le scénario `develop` réécrite, Ramure propose `git rebase --onto develop feat/a~3 feat/a` et le résultat ne contient que les 3 commits de `feat/a`.

---

## 10. Décisions et questions ouvertes

**Décidé (v0.2)**

| Sujet | Décision |
|-------|----------|
| Licence | Open source, MIT OR Apache-2.0 |
| Services externes | Aucune intégration, durablement |
| Profils git | Gérés par git (`includeIf`) ; Ramure lit et affiche seulement |
| Terminal | Non intégré. Niveau 0 : commandes à copier. Niveau 1 : envoi dans un terminal externe choisi (tmux, WezTerm, kitty, iTerm2, Terminal.app, Windows Terminal), ⌥ pour copier |
| Vue du graph | Refs · graph · texte, messages alignés (`ux-graph.md`) |

**Ouvert**

1. **Staging et commit** : confirmer qu'ils restent hors périmètre (faits dans l'IDE ou le
   terminal), Ramure ne montrant que le WIP en lecture.
2. **Niveau 1 et commandes qui détruisent du travail** (ex. `git reset --hard` alors que des
   fichiers modifiés n'ont pas été commités : ces modifications sont perdues, et `⌘Z` ne peut
   rétablir que la branche, pas les fichiers). Deux options : Ramure tape la commande dans le
   terminal **sans appuyer sur Entrée** et l'utilisateur valide lui-même (proposé), ou Ramure
   demande une **confirmation dans sa propre fenêtre** puis exécute comme les autres commandes.
3. **Warp** : l'intégration ne peut qu'ouvrir un nouvel onglet par commande (voir §3.7). À
   confirmer par un essai (bug signalé sur les chemins de launch configurations personnalisés)
   et à juger à l'usage : un onglet par commande risque d'être pénible pour les gestes fréquents.
4. **Licence exacte** : double MIT/Apache-2.0 proposée ; à valider.
5. **Nom** : « Ramure » est un nom de code ; vérifier la disponibilité (crates.io, npm, domaine).
