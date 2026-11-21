# valider-et-versionner Specification

## Purpose
Vérifier que le ledger est valide et le sauvegarder dans git, depuis l'application ou en ligne de commande, sans jamais commiter un ledger invalide.

## Requirements

### Requirement: Commande check
Le système SHALL fournir `household-cli check [<main.beancount>]` avec les codes de sortie 0 (valide),
1 (invalide, y compris erreur de chargement fatale), 2 (usage).

#### Scenario: Ledger valide
- **WHEN** `check` est lancé sur un ledger sans erreur
- **THEN** le code de sortie est 0

#### Scenario: Include manquant
- **WHEN** un include est absent
- **THEN** le code de sortie est 1

### Requirement: Commande balances
Le système SHALL fournir `household-cli balances [<main>] [--date=AAAA-MM-JJ]` affichant les soldes par
compte et devise, à la date donnée (incluse) ou à la fin du ledger.

#### Scenario: Date invalide
- **WHEN** `--date` n'est pas une date valide
- **THEN** la commande échoue avec le code d'usage

### Requirement: Hook pre-commit
Le système SHALL fournir un hook `githooks/pre-commit` qui valide le ledger du dépôt courant
(`main.beancount` à la racine, ou `examples/ledger/main.beancount` dans le dépôt logiciel), via `rledger`
ou en repli `household-cli`, et refuse le commit en cas d'erreur.

#### Scenario: Commit invalide
- **WHEN** un commit contient un ledger invalide
- **THEN** le commit est refusé

#### Scenario: Hook partagé avec le dépôt du vrai ledger
- **WHEN** `just setup-ledger <dossier>` est exécuté
- **THEN** `core.hooksPath` du dépôt du ledger pointe vers `githooks/` du dépôt logiciel (chemin absolu)

### Requirement: Statut par fichier
Le système SHALL lister les fichiers modifiés avec leurs états (nouveau, modifié, supprimé, renommé),
index et arborescence pouvant se cumuler, et les chemins relatifs à la racine du dépôt.

#### Scenario: Fichier modifié et indexé
- **WHEN** un fichier est modifié puis indexé
- **THEN** ses états sont cumulés dans une seule entrée

### Requirement: Commit validé
Le système SHALL, avant de commiter, recharger le ledger complet et refuser le commit s'il contient une
erreur, car libgit2 n'exécute pas le hook pre-commit. Il SHALL indexer tout puis commiter.

#### Scenario: Ledger invalide
- **WHEN** l'utilisateur commite alors que le ledger a une erreur
- **THEN** le commit est refusé et une erreur est affichée

#### Scenario: Identité absente
- **WHEN** `user.name` ou `user.email` ne sont pas configurés
- **THEN** l'identité de repli « Comptabilité du foyer » / `ledger@local` est utilisée

### Requirement: Message suggéré
Le système SHALL suggérer `ledger: AAAA-MM — saisie/édition (N fichier(s))` d'après le mois des fichiers
`transactions/AAAA/MM.beancount` modifiés, une plage `AAAA-MM → AAAA-MM` si plusieurs mois, un message
« budgets » pour `budgets/`, et un message générique sinon. Le message reste éditable : aucun commit silencieux.

#### Scenario: Un seul mois
- **WHEN** seul `transactions/2026/04.beancount` est modifié
- **THEN** le message suggéré contient `2026-04`

#### Scenario: Plusieurs mois
- **WHEN** plusieurs mois sont modifiés
- **THEN** le message indique la plage du premier au dernier mois

### Requirement: Historique et résumé de diff
Le système SHALL fournir l'historique (du plus récent) filtrable par chemin et un résumé du diff
(fichiers, insertions, suppressions).

#### Scenario: Dépôt introuvable
- **WHEN** aucun dépôt git n'englobe le ledger
- **THEN** l'écran Git est informatif (`disponible = false`) sans erreur fatale

### Requirement: Séparation des dépôts
Le système MUST NOT créer de lien de versionnement entre le dépôt logiciel et le dépôt du vrai ledger.

#### Scenario: Écran Git
- **WHEN** l'écran Git est ouvert
- **THEN** il opère sur le dépôt du ledger résolu, pas sur le dépôt logiciel
