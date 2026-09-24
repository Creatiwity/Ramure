#!/usr/bin/env bash
# Crée le dépôt d'exemple des maquettes (historique « sherpa ») dans le dossier donné.
# Usage : fixtures/sample-repo.sh fixtures/sample-repo
set -euo pipefail
dir="${1:?usage : sample-repo.sh <dossier>}"
rm -rf "$dir" && mkdir -p "$dir" && cd "$dir"
git init -q -b main
git config user.name "Julien B."
git config user.email "dev@creatiwity.net"
git config commit.gpgsign false

n=0
c() { # c "<auteur>" "<date>" "<message>" [fichier]
  n=$((n + 1))
  local f="${4:-src/f$n.ts}"
  mkdir -p "$(dirname "$f")"
  printf 'export const v%s = %s;\n' "$n" "$n" >> "$f"
  git add -A
  GIT_AUTHOR_NAME="$1" GIT_AUTHOR_EMAIL="$(echo "$1" | tr 'A-Z' 'a-z' | cut -c1-4 | tr -dc 'a-z')@example.com" \
  GIT_AUTHOR_DATE="$2" GIT_COMMITTER_DATE="$2" GIT_COMMITTER_NAME="$1" GIT_COMMITTER_EMAIL="ci@example.com" \
    git commit -q -m "$3"
}
m() { # m "<auteur>" "<date>" "<branche>" "<message>"
  GIT_AUTHOR_NAME="$1" GIT_AUTHOR_DATE="$2" GIT_COMMITTER_DATE="$2" GIT_COMMITTER_NAME="$1" GIT_COMMITTER_EMAIL="ci@example.com" \
    git merge -q --no-ff -m "$4" "$3"
}

c "Sam T." "2026-09-02T09:12:00" "chore: initialise le projet Nuxt 4" package.json
c "Sam T." "2026-09-04T10:30:00" "feat(tasks): modèle de tâche et migrations" server/db/schema.ts
c "Sam T." "2026-09-09T16:05:00" "feat(pwa): installation hors ligne" public/sw.js
git tag -a v0.3.0 -m "v0.3.0"
git switch -q -c feature/onboarding
c "Léa M." "2026-09-12T11:20:00" "feat(onboarding): choix des catégories" pages/onboarding.vue
c "Léa M." "2026-09-14T14:45:00" "feat(onboarding): écran de bienvenue" pages/welcome.vue
git switch -q main
c "Camille R." "2026-09-15T09:40:00" "fix(auth): expire la session après 30 j" server/auth.ts
m "Sam T." "2026-09-16T10:00:00" feature/onboarding "Merge branch 'feature/onboarding'"
c "Sam T." "2026-09-18T15:30:00" "feat(tasks): vue « une tâche à la fois »" pages/index.vue
git tag -a v0.4.0 -m "v0.4.0"
git switch -q -c feature/scoring
c "Julien B." "2026-09-23T18:10:00" "refactor(scoring): extrait computeUrgency()" server/scoring/urgency.ts
git switch -q main
git switch -q -c fix/pwa-cache HEAD
c "Léa M." "2026-09-20T12:00:00" "test(pwa): couvre la mise à jour du service worker" tests/sw.spec.ts
c "Léa M." "2026-09-21T09:15:00" "fix(pwa): invalide le cache au changement de version" public/sw.js
git switch -q main
c "Sam T." "2026-09-21T17:40:00" "chore(deps): passe à nuxt 4.1.2" package.json
m "Camille R." "2026-09-23T11:05:00" fix/pwa-cache "Merge pull request #142 from creatiwity/fix/pwa-cache"
c "Camille R." "2026-09-24T15:10:00" "fix(api): gère le 404 sur /tasks/:id" server/api/tasks.ts
git branch -q release/0.4 v0.4.0
# Distantes simulées (pas de réseau) : origin/main à jour, origin/feature/scoring en retard d'un commit.
git update-ref refs/remotes/origin/main main
git update-ref refs/remotes/origin/release/0.4 release/0.4
git update-ref refs/remotes/origin/fix/pwa-cache fix/pwa-cache
git switch -q feature/scoring
git update-ref refs/remotes/origin/feature/scoring HEAD
c "Julien B." "2026-09-24T17:02:00" "feat(scoring): pondère les tâches en retard" server/scoring/urgency.ts
git branch -q -D fix/pwa-cache
# Un stash et des modifications en cours.
echo "// essai" >> pages/index.vue
GIT_COMMITTER_DATE="2026-09-17T10:00:00" git stash push -q -m "essai layout mobile"
echo "export const wip = true;" >> server/scoring/urgency.ts
echo "note" > NOTES.md
git add NOTES.md
echo "Dépôt d'exemple créé dans $dir"
