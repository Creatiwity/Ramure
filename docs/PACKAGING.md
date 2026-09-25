# Packaging et signature de Ramure

Ce guide explique comment le pipeline de release produit les installeurs, quels secrets il
attend, comment les obtenir côté Apple (et, à venir, côté Windows : §8) et comment les
enregistrer dans GitHub.

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
| Windows 10/11 | `.msi`, `-setup.exe` | Non signé pour l'instant (SmartScreen affiche un avertissement) ; procédure au §8. |

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

## 8. Signer pour Windows

### 8.1 Quelle option choisir

Un certificat Authenticode classique (OV ou EV) coûte plusieurs centaines d'euros par an et,
depuis juin 2023, sa clé privée doit être stockée sur un module matériel (clé USB ou HSM dans le
cloud), ce qui complique la CI. Depuis 2024, un certificat EV n'efface plus l'avertissement
SmartScreen immédiatement : EV et OV construisent leur réputation de la même manière. Payer un
EV n'apporte donc plus rien ici.

| Option | Coût | Éditeur affiché | CI GitHub | Verdict |
|--------|------|-----------------|-----------|---------|
| **Azure Artifact Signing** (ex-*Trusted Signing*), SKU *Basic* | 9,99 $/mois (5 000 signatures) | **Creatiwity** | Oui, via `signCommand` de Tauri | **Recommandée** |
| SignPath Foundation (programme open source) | Gratuit | *SignPath Foundation* | Oui, mais signature après le build et validation manuelle de chaque release | Possible, plus contraignante |
| Certificat OV classique (Sectigo, Certum…) | Plusieurs centaines d'euros par an, plus le support matériel | Creatiwity | Difficile (clé matérielle, code à usage unique) | À éviter |

Artifact Signing est ouvert aux **organisations de l'Union européenne** (les particuliers doivent
être aux États-Unis ou au Canada) : Creatiwity est éligible. Microsoft détient la clé et émet
des certificats de courte durée renouvelés automatiquement ; il n'y a aucun fichier de
certificat à gérer ni à faire expirer. Une release signe 3 à 5 fichiers, très loin de la limite
des 5 000 signatures mensuelles.

SignPath reste une option de repli si l'on ne veut rien payer : le projet est open source et
sans composant propriétaire, ce qui correspond à ses conditions. Mais l'installeur affiche
« SignPath Foundation » comme éditeur, chaque release attend une validation dans leur interface,
et la signature intervient après le build de Tauri : il faudrait resigner les archives de
l'updater (§9) après coup.

Dans tous les cas, SmartScreen peut encore avertir sur les premières versions signées, le temps
que l'éditeur se fasse une réputation. Signer chaque release avec la même identité la fait
progresser d'une version à l'autre.

### 8.2 Mise en place d'Azure Artifact Signing (une seule fois)

Prévoir de 1 à 20 jours ouvrés pour la validation d'identité, faite par Microsoft.

1. **Abonnement Azure** : se connecter au [portail Azure](https://portal.azure.com) avec un compte
   Creatiwity et créer un abonnement à l'usage (*pay-as-you-go*) s'il n'y en a pas.
2. **Fournisseur de ressources** : *Abonnements → (l'abonnement) → Fournisseurs de ressources*,
   enregistrer `Microsoft.CodeSigning`. En ligne de commande :
   `az provider register --namespace "Microsoft.CodeSigning"`.
3. **Compte Artifact Signing** : créer une ressource *Artifact Signing Account*, SKU **Basic**,
   dans une région européenne. Noter le nom du compte et l'endpoint de la région :

   | Région | Endpoint |
   |--------|----------|
   | West Europe | `https://weu.codesigning.azure.net` |
   | North Europe | `https://neu.codesigning.azure.net` |
   | Poland Central | `https://plc.codesigning.azure.net` |
   | Switzerland North | `https://swn.codesigning.azure.net` |

4. **Rôle de validation** : sur le compte, *Contrôle d'accès (IAM) → Ajouter une attribution de
   rôle*, donner à la personne qui fera la demande le rôle **Artifact Signing Identity Verifier**
   (elle doit aussi avoir au moins le rôle *Lecteur* sur l'abonnement).
5. **Validation d'identité** : sur le compte, *Identity validations → New identity → Public*,
   type *Organization*. Renseigner la raison sociale exacte de Creatiwity, son adresse, et un
   e-mail **sur le domaine de Creatiwity** (le lien de vérification expire au bout de 7 jours).
   Le représentant désigné passe ensuite une vérification individuelle avec sa pièce d'identité.
   Microsoft peut demander un extrait Kbis de moins de 12 mois et une preuve de propriété du
   domaine : les téléverser dans le portail (trois tentatives).
6. **Profil de certificat** : une fois l'identité validée, *Certificate profiles → Create →*
   **Public Trust**, rattaché à cette validation. Noter le nom du profil (par exemple `ramure`).
7. **Application pour la CI** : dans *Microsoft Entra ID → Inscriptions d'applications →
   Nouvelle inscription*, créer `ramure-release`. Noter l'*ID d'application (client)* et l'*ID de
   l'annuaire (locataire)*. Dans *Certificats et secrets*, créer un **secret client** et copier
   sa *Valeur* tout de suite (elle n'est plus affichée ensuite). Noter aussi sa date
   d'expiration (24 mois au plus).
8. **Droit de signer** : sur le **profil de certificat**, *Contrôle d'accès (IAM)*, donner à
   l'application `ramure-release` le rôle **Artifact Signing Certificate Profile Signer**.

### 8.3 Secrets et variables GitHub

Dans l'environnement `release` (§4.1). L'endpoint et les noms ne sont pas secrets : ce sont des
*variables* d'environnement.

| Nom | Type | Valeur |
|-----|------|--------|
| `AZURE_TENANT_ID` | secret | ID de l'annuaire (locataire) |
| `AZURE_CLIENT_ID` | secret | ID d'application (client) de `ramure-release` |
| `AZURE_CLIENT_SECRET` | secret | Valeur du secret client |
| `AZURE_SIGNING_ENDPOINT` | variable | par exemple `https://weu.codesigning.azure.net` |
| `AZURE_SIGNING_ACCOUNT` | variable | nom du compte Artifact Signing |
| `AZURE_SIGNING_PROFILE` | variable | nom du profil de certificat |

```sh
REPO=Creatiwity/ramure
gh secret set AZURE_TENANT_ID     --env release --repo $REPO --body "…"
gh secret set AZURE_CLIENT_ID     --env release --repo $REPO --body "…"
gh secret set AZURE_CLIENT_SECRET --env release --repo $REPO     # saisie masquée
gh variable set AZURE_SIGNING_ENDPOINT --env release --repo $REPO --body "https://weu.codesigning.azure.net"
gh variable set AZURE_SIGNING_ACCOUNT  --env release --repo $REPO --body "…"
gh variable set AZURE_SIGNING_PROFILE  --env release --repo $REPO --body "ramure"
```

Mettre un rappel avant l'expiration du secret client : il faudra en créer un nouveau et
remplacer `AZURE_CLIENT_SECRET`.

### 8.4 Ce qui changera dans le pipeline

À brancher une fois le profil de certificat créé :

- Sur le runner Windows, installer [`artifact-signing-cli`](https://github.com/Levminer/artifact-signing-cli)
  (outil recommandé par la documentation de Tauri ; .NET, Azure CLI et `signtool` sont déjà
  présents sur `windows-latest`).
- Passer à Tauri un fichier de configuration supplémentaire, uniquement dans le pipeline, pour
  que les builds locales des contributeurs ne tentent pas de signer :

  ```json
  {
    "bundle": {
      "windows": {
        "signCommand": "artifact-signing-cli -e <endpoint> -a <compte> -c <profil> -d Ramure %1"
      }
    }
  }
  ```

  `-d Ramure` fixe le nom affiché dans la fenêtre de contrôle de compte (UAC) à l'installation
  du `.msi`. Tauri signe ainsi l'exécutable, le `.msi` et l'installeur `-setup.exe`.
- Comme sur macOS : signature obligatoire sur un tag (échec explicite si un secret manque),
  facultative en lancement manuel, puis vérification par
  `signtool verify /pa /v` (ou `Get-AuthenticodeSignature`, qui doit renvoyer `Valid`).

### 8.5 Vérifier un installeur signé

Sous Windows : clic droit sur le `.msi` → *Propriétés → Signatures numériques*, l'éditeur doit
être Creatiwity. En PowerShell :

```powershell
Get-AuthenticodeSignature .\Ramure_0.2.0_x64_en-US.msi | Format-List Status, SignerCertificate
```

## 9. Pas encore couvert

- Signature Windows : procédure décrite au §8, pas encore branchée dans le pipeline.
- Mises à jour automatiques (plugin updater de Tauri) : demandera une paire de clés
  `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` et `uploadUpdaterJson`.
- Notarisation du `.dmg` lui-même : aujourd'hui l'application qu'il contient est notarisée et
  agrafée, ce qui suffit à Gatekeeper ; notariser aussi l'image évite une vérification en ligne à
  l'ouverture du `.dmg`.
