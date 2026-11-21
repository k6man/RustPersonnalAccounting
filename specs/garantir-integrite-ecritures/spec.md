# garantir-integrite-ecritures Specification

## Purpose
Garantie transversale : aucune action de l'utilisateur ne peut corrompre le ledger ni l'invalider, quelle que soit la voie d'écriture (saisie, budget, import).

## Requirements

### Requirement: Écriture canonique et chirurgicale
Le système SHALL produire les blocs via le formateur rustledger et modifier uniquement la span exacte
concernée, en préservant commentaires et mise en forme du reste du fichier.

#### Scenario: Ajout d'une transaction
- **WHEN** une transaction datée du 2026-03-26 est ajoutée
- **THEN** elle est insérée dans `transactions/2026/03.beancount`, avant la première ligne de date strictement supérieure

#### Scenario: Fichier mensuel inexistant
- **WHEN** le fichier mensuel cible n'existe pas
- **THEN** il est créé avec un en-tête et `main.beancount` n'est pas modifié (includes en globs)

### Requirement: Pré-vérification sur copie scratch
Le système SHALL jouer toute mutation sur une copie temporaire du ledger et refuser l'écriture si elle
introduit une erreur fatale nouvelle ; un diagnostic préexistant MUST NOT bloquer le ledger.

#### Scenario: Écriture introduisant une erreur
- **WHEN** la transaction référence un compte fermé
- **THEN** l'écriture est refusée et le ledger réel reste inchangé

#### Scenario: Warning préexistant
- **WHEN** le ledger contient déjà un warning sans rapport
- **THEN** une écriture valide reste acceptée

### Requirement: Atomicité et revalidation
Le système SHALL écrire via fichier temporaire puis rename, revalider le ledger réel, et appliquer les
mutations multi-fichiers de façon tout-ou-rien.

#### Scenario: Remplacement avec changement de date
- **WHEN** une transaction est modifiée vers une autre date
- **THEN** retrait et réinsertion triée sont appliqués en une seule mutation

### Requirement: Verrou d'écriture
Le système SHALL prendre un verrou `.write.lock` pendant l'écriture, libéré même en cas de panic, et
considéré périmé après 120 s.

#### Scenario: Écritures concurrentes
- **WHEN** deux processus tentent d'écrire simultanément
- **THEN** le second attend le verrou ou échoue, sans écriture concurrente

### Requirement: Suppression par span
Le système SHALL supprimer une transaction par span exacte et normaliser les lignes vides en excès.

#### Scenario: Suppression
- **WHEN** une transaction est supprimée
- **THEN** seul son bloc disparaît et les triplets de lignes vides sont réduits à un
