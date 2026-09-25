# Packaging et signature de Ramure

Ce guide explique comment le pipeline de release produit les installeurs, quels secrets il
attend, comment les obtenir côté Apple et comment les enregistrer dans GitHub.

## 1. Ce que fait le pipeline

Deux workflows GitHub Actions :

| Workflow | Déclencheur | Rôle |
|----------|-------------|------|
| `.github/workflows/ci.yml` | push sur `main`, pull requests | Formatage, clippy, tests Rust (dont des dépôts git réels), typecheck, tests et build du front, cohérence des versions. |
| `.github/workflows/release.yml` | tag `vX.Y.Z` poussé, ou lancement manuel | Build des installeurs macOS, Linux et Windows. |

Installeurs produits par `release.yml` :

| Plateforme | Fichiers | Signature |
|------------|----------|-----------|
| macOS 12+ (binaire universel Apple Silicon + Intel) | `.dmg` | **Signé** avec le certificat *Developer ID Application* de Creatiwity et **notarisé** par Apple (ticket agrafé à l'application). |
| Linux | `.AppImage`, `.deb`, `.rpm` | Non signé. |
| Windows 10/11 | `.msi`, `-setup.exe` | Non signé pour l'instant (SmartScreen affiche un avertissement). |

Deux modes :

- **Tag `vX.Y.Z`** : la signature macOS est obligatoire. Si un secret Apple manque, le build macOS
  échoue avec la liste des secrets absents, plutôt que de publier une application que macOS
  refuserait d'ouvrir. Une **release brouillon** est créée avec tous les installeurs ; on la
  relit puis on la publie à la main.
- **Lancement manuel** (onglet *Actions* → *Release* → *Run workflow*) : build de test, les
  installeurs sont téléchargeables dans les artefacts du workflow. L'option *Signer et notariser
  la build macOS* est cochée par défaut ; décochée, le `.dmg` n'est pas signé (utile pour tester le
  pipeline sans les secrets).

Après le build signé, le workflow vérifie lui-même le résultat : `codesign --verify`, `spctl`
(Gatekeeper doit répondre *Notarized Developer ID*) et `xcrun stapler validate`.

## 2. Secrets attendus

Tous les secrets sont rangés dans l'**environnement GitHub `release`** (voir §4). Aucun n'est
nécessaire pour la CI ni pour un build de test non signé.

| Secret | Obligatoire | Contenu | Exemple de forme |
|--------|-------------|---------|------------------|
| `APPLE_CERTIFICATE` | oui | Le certificat *Developer ID Application* **avec sa clé privée**, exporté en `.p12`, puis encodé en base64 sur une seule ligne. | `MIIMrQIBAzCCDHcGCSqGSIb3…` |
| `APPLE_CERTIFICATE_PASSWORD` | oui | Le mot de passe choisi lors de l'export du `.p12`. | — |
| `APPLE_API_ISSUER` | oui | *Issuer ID* de l'API App Store Connect (un UUID, affiché en haut de la page des clés). | `69a6de7e-…-…-…-…` |
| `APPLE_API_KEY` | oui | *Key ID* de la clé API App Store Connect (10 caractères). | `2X9R4HXF34` |
| `APPLE_API_KEY_P8` | oui | Le **contenu complet** du fichier `AuthKey_<KeyID>.p8`, lignes `-----BEGIN PRIVATE KEY-----` et `-----END PRIVATE KEY-----` comprises. | `-----BEGIN PRIVATE KEY-----`… |
| `APPLE_SIGNING_IDENTITY` | non | Nom exact de l'identité de signature. Si absent, Tauri prend celle du certificat ; si présent, il vérifie qu'elle correspond. | `Developer ID Application: Creatiwity (ABCDE12345)` |

`GITHUB_TOKEN` est fourni automatiquement par GitHub : rien à faire.

La notarisation passe par une **clé API App Store Connect** plutôt que par un identifiant Apple et
un mot de passe d'application : elle ne dépend pas du compte personnel d'un salarié et n'est pas
soumise à la double authentification. (Tauri accepte aussi `APPLE_ID`, `APPLE_PASSWORD` et
`APPLE_TEAM_ID`, mais le workflow n'est pas branché sur cette méthode.)

## 3. Obtenir les éléments côté Apple

À faire une fois, par une personne qui a le rôle **Account Holder** du compte Apple Developer de
Creatiwity (selon la configuration du compte, un **Admin** peut aussi créer des certificats
Developer ID). Il faut un Mac.

### 3.1 Certificat *Developer ID Application*

1. Sur le Mac, ouvrir **Trousseaux d'accès** → menu *Trousseaux d'accès* → *Assistant de
   certification* → *Demander un certificat à une autorité de certificat…*. Saisir l'e-mail et le
   nom (« Creatiwity »), choisir **Enregistrée sur le disque**. On obtient un fichier
   `CertificateSigningRequest.certSigningRequest`.
2. Sur [developer.apple.com/account](https://developer.apple.com/account) → *Certificates,
   Identifiers & Profiles* → *Certificates* → **+** → **Developer ID Application** (profil
   *G2 Sub-CA*) → envoyer le fichier de demande → télécharger le `.cer`.
3. Double-cliquer le `.cer` : il s'installe dans le trousseau *session*, associé à la clé privée
   créée à l'étape 1.
4. Dans Trousseaux d'accès → *Mes certificats* → clic droit sur **Developer ID Application:
   Creatiwity (…)** → *Exporter…* → format **.p12**, avec un mot de passe robuste (c'est
   `APPLE_CERTIFICATE_PASSWORD`). Vérifier que la ligne se déplie sur une clé privée : sans elle,
   le `.p12` ne permet pas de signer.
5. Relever l'identité exacte (c'est `APPLE_SIGNING_IDENTITY`, facultatif) :

   ```bash
   security find-identity -v -p codesigning
   #  1) 3F2…A91 "Developer ID Application: Creatiwity (ABCDE12345)"
   ```

6. Encoder le `.p12` en base64 sur une ligne (c'est `APPLE_CERTIFICATE`) :

   ```bash
   base64 -i ramure-developer-id.p12 | tr -d '\n' > ramure-developer-id.p12.b64
   ```

Le certificat est valable 5 ans. Apple limite le nombre de certificats Developer ID par compte :
réutiliser celui de Creatiwity s'il existe déjà (il suffit d'en exporter le `.p12` depuis le Mac
qui a sa clé privée).

### 3.2 Clé API App Store Connect (notarisation)

1. Sur [appstoreconnect.apple.com](https://appstoreconnect.apple.com) → *Utilisateurs et accès* →
   onglet **Intégrations** → *App Store Connect API* → **Clés de l'équipe** → **+**.
2. Nom : « Ramure CI notarisation », accès : **Developer** (suffisant pour notariser).
3. Télécharger le fichier `AuthKey_<KeyID>.p8`. **Apple ne le propose qu'une seule fois.**
4. Relever le **Key ID** (colonne de la clé : `APPLE_API_KEY`) et l'**Issuer ID** (en haut de la
   page : `APPLE_API_ISSUER`).

## 4. Enregistrer les secrets dans GitHub

### 4.1 Créer l'environnement `release`

Dans le dépôt `Creatiwity/ramure` → *Settings* → *Environments* → **New environment** → nom
**`release`** (exactement). Réglages recommandés :

- *Deployment protection rules* → **Required reviewers** : une ou deux personnes de l'équipe.
  Chaque build de release attendra leur validation avant d'accéder aux secrets.
- *Deployment branches and tags* → **Selected branches and tags** → ajouter la règle de tag `v*`
  et la branche `main` (pour les lancements manuels).

Le workflow référence cet environnement : s'il n'existe pas, GitHub le crée au premier lancement,
sans protection.

### 4.2 Ajouter les secrets

**Par l'interface** : *Settings* → *Environments* → `release` → *Environment secrets* → **Add
environment secret**, une fois par secret du §2. Pour `APPLE_API_KEY_P8`, coller le contenu complet
du fichier `.p8` (plusieurs lignes, c'est normal).

**En ligne de commande** avec [GitHub CLI](https://cli.github.com) (évite les copier-coller et
l'historique du shell) :

```bash
REPO=Creatiwity/ramure
gh secret set APPLE_CERTIFICATE          --env release --repo $REPO < ramure-developer-id.p12.b64
gh secret set APPLE_CERTIFICATE_PASSWORD --env release --repo $REPO   # saisie masquée au clavier
gh secret set APPLE_API_ISSUER           --env release --repo $REPO --body "69a6de7e-…"
gh secret set APPLE_API_KEY              --env release --repo $REPO --body "2X9R4HXF34"
gh secret set APPLE_API_KEY_P8           --env release --repo $REPO < AuthKey_2X9R4HXF34.p8
gh secret set APPLE_SIGNING_IDENTITY     --env release --repo $REPO --body "Developer ID Application: Creatiwity (ABCDE12345)"   # facultatif

gh secret list --env release --repo $REPO   # vérifier la présence (les valeurs ne sont jamais affichées)
```

### 4.3 Après l'enregistrement

- Ranger le `.p12`, son mot de passe et le `.p8` dans le coffre de mots de passe de l'équipe,
  puis supprimer les copies locales (`.p12.b64` compris).
- Ne jamais les committer, ni les coller dans une issue ou une conversation.
- En cas de fuite : révoquer le certificat sur developer.apple.com et la clé sur App Store
  Connect, en recréer, puis mettre à jour les secrets.

## 5. Publier une version

1. Mettre la même version dans `package.json`, `Cargo.toml` (`[workspace.package]`) et
   `src-tauri/tauri.conf.json`, puis vérifier :

   ```bash
   npm install --package-lock-only   # met à jour package-lock.json
   node scripts/check-version.mjs v0.3.0
   ```

2. Committer sur `main`, puis taguer et pousser :

   ```bash
   git tag v0.3.0
   git push origin v0.3.0
   ```

3. Valider le déploiement dans l'onglet *Actions* si des relecteurs sont requis.
4. À la fin, ouvrir la release **brouillon** créée par le workflow, relire les notes, vérifier les
   installeurs, puis **Publish release**.

Un tag avec un tiret (`v0.3.0-beta.1`) crée une pré-release.

Pour tester le pipeline sans publier : *Actions* → *Release* → *Run workflow*, option de signature
cochée ou non ; les installeurs sont dans les artefacts du run.

## 6. Vérifier une application signée sur un Mac

```bash
codesign -dv --verbose=4 /Applications/Ramure.app   # Authority=Developer ID Application: Creatiwity (…)
spctl -a -vv /Applications/Ramure.app               # accepted, source=Notarized Developer ID
xcrun stapler validate /Applications/Ramure.app     # The validate action worked!
```

## 7. Dépannage

| Symptôme | Cause probable |
|----------|----------------|
| « Secrets Apple manquants : … » | Secret absent de l'environnement `release`, ou nom mal orthographié. |
| « certificate from APPLE_CERTIFICATE … does not match provided identity » | `APPLE_SIGNING_IDENTITY` ne correspond pas au certificat : le corriger ou le supprimer. |
| Erreur à l'import du certificat (`security import`, mot de passe) | Mauvais `APPLE_CERTIFICATE_PASSWORD`, ou base64 tronqué (réencoder avec `tr -d '\n'`). |
| `errSecInternalComponent` ou « no identity found » à la signature | Le `.p12` ne contient pas la clé privée : réexporter depuis *Mes certificats*. |
| Notarisation « Invalid » | Voir le journal Apple : `xcrun notarytool log <submission-id> --key AuthKey_….p8 --key-id … --issuer …`. |
| `spctl` : « rejected » | Application signée mais pas notarisée, ou certificat de type *Apple Development* au lieu de *Developer ID Application*. |
| Certificat expiré (au bout de 5 ans) | En créer un nouveau (§3.1) et mettre à jour `APPLE_CERTIFICATE` et `APPLE_CERTIFICATE_PASSWORD`. |

## 8. Pas encore couvert

- Signature Windows (Authenticode, ou Azure Trusted Signing) : supprimerait l'avertissement
  SmartScreen.
- Mises à jour automatiques (plugin updater de Tauri) : demandera une paire de clés
  `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` et `uploadUpdaterJson`.
- Notarisation du `.dmg` lui-même : aujourd'hui l'application qu'il contient est notarisée et
  agrafée, ce qui suffit à Gatekeeper ; notariser aussi l'image évite une vérification en ligne à
  l'ouverture du `.dmg`.
