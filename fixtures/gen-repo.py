#!/usr/bin/env python3
"""Génère un dépôt git synthétique réaliste pour les mesures de performance de Ramure.

Usage : python3 fixtures/gen-repo.py <dossier> [nombre_de_commits]

Historique produit : un tronc `main`, une branche `develop`, des branches de fonctionnalité
courtes mergées régulièrement (merge commits), quelques branches longues toujours ouvertes,
des tags de release. Utilise `git fast-import` (quelques secondes pour 100 k commits).
"""
import os, random, subprocess, sys

target = sys.argv[1]
total = int(sys.argv[2]) if len(sys.argv) > 2 else 100_000
random.seed(7)
os.makedirs(target, exist_ok=True)
subprocess.run(["git", "init", "-q", "-b", "main", target], check=True)

TYPES = ["feat", "fix", "chore", "refactor", "test", "docs", "perf"]
SCOPES = ["api", "scoring", "pwa", "auth", "tasks", "ui", "deps", "db", "export", "search"]
WORDS = ("ajoute gère corrige extrait simplifie couvre invalide pondère renomme supprime "
         "optimise documente migre la vue le cache les tâches le score la session le tri "
         "l'export la pagination les filtres le formulaire la recherche le service worker").split()
AUTHORS = [("Camille R.", "camille@example.com"), ("Sam T.", "sam@example.com"),
           ("Léa M.", "lea@example.com"), ("Julien B.", "julien@example.com"),
           ("Noé K.", "noe@example.com"), ("Inès D.", "ines@example.com")]

out = []
mark = 0
t = 1_600_000_000

def msg():
    return f"{random.choice(TYPES)}({random.choice(SCOPES)}): " + " ".join(random.sample(WORDS, random.randint(3, 7)))

def commit(ref, parents, message):
    global mark, t
    mark += 1
    t += random.randint(60, 3600)
    name, email = random.choice(AUTHORS)
    data = message.encode()
    out.append(f"commit {ref}\nmark :{mark}\nauthor {name} <{email}> {t} +0200\ncommitter {name} <{email}> {t} +0200\n")
    out.append(f"data {len(data)}\n")
    out.append(message + "\n")
    if parents:
        out.append(f"from :{parents[0]}\n")
        for p in parents[1:]:
            out.append(f"merge :{p}\n")
    blob = f"{mark}\n".encode()
    out.append(f"M 100644 inline src/{random.choice(SCOPES)}/f{mark % 500}.txt\ndata {len(blob)}\n{mark}\n\n")
    return mark

main = commit("refs/heads/main", [], "chore: initialise le projet")
develop = commit("refs/heads/develop", [main], "chore: crée develop")
long_lived = {}
pr = 0
count = 2
while count < total:
    r = random.random()
    if r < 0.55:
        # Branche de fonctionnalité courte, mergée dans develop.
        base = develop
        tip = base
        name = f"feature/{random.choice(SCOPES)}-{count}"
        for _ in range(random.randint(1, 6)):
            tip = commit(f"refs/heads/{name}", [tip], msg()); count += 1
        pr += 1
        develop = commit("refs/heads/develop", [develop, tip], f"Merge pull request #{pr} from creatiwity/{name}"); count += 1
    elif r < 0.85:
        develop = commit("refs/heads/develop", [develop], msg()); count += 1
    elif r < 0.93:
        main = commit("refs/heads/main", [main, develop], "Merge branch 'develop'"); count += 1
        if random.random() < 0.3:
            out.append(f"reset refs/tags/v{count // 1000}.{count % 1000}\nfrom :{main}\n\n")
    else:
        key = random.choice(["refonte-ui", "migration-db", "export-v2"])
        base = long_lived.get(key, develop)
        long_lived[key] = commit(f"refs/heads/wip/{key}", [base], msg()); count += 1

# Les branches de fonctionnalité mergées n'ont plus de ref (comme après suppression).
p = subprocess.run(["git", "-C", target, "fast-import", "--quiet"], input="".join(out).encode(), check=True)
for ref in subprocess.run(["git", "-C", target, "for-each-ref", "--format=%(refname)", "refs/heads/feature/"], capture_output=True, text=True).stdout.split():
    subprocess.run(["git", "-C", target, "update-ref", "-d", ref], check=True)
subprocess.run(["git", "-C", target, "update-ref", "refs/remotes/origin/main", "refs/heads/main"], check=True)
subprocess.run(["git", "-C", target, "update-ref", "refs/remotes/origin/develop", "refs/heads/develop"], check=True)
subprocess.run(["git", "-C", target, "checkout", "-q", "develop"], check=True)
subprocess.run(["git", "-C", target, "commit-graph", "write", "--reachable"], check=True)
print(f"{count} commits dans {target}")
