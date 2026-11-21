# saisir-transaction Specification

## Purpose
Enregistrer ou corriger une dépense, un revenu ou un transfert avec une aide à la saisie et un avis immédiat sur la validité du brouillon.

## Requirements

### Requirement: Validation de forme
Le système SHALL signaler : date invalide (attendu AAAA-MM-JJ), libellé vide, flag différent de `*` ou
`!`, compte manquant sur une ligne avec montant, montant illisible, absence de posting, plus d'une ligne
d'équilibrage, déséquilibre par devise, et devises mélangées avec équilibrage automatique.

#### Scenario: Brouillon déséquilibré
- **WHEN** les montants ne s'annulent pas et aucune ligne n'est laissée vide
- **THEN** une erreur « déséquilibre » indique le reste par devise

#### Scenario: Ligne entièrement vide
- **WHEN** une ligne n'a ni compte ni montant
- **THEN** elle est ignorée

### Requirement: Ligne d'équilibrage explicite
Le système SHALL écrire explicitement la contrepartie de la ligne laissée vide avec son montant calculé
(pas de posting élidé), et MUST NOT écrire un posting à 0.00 lorsque le résidu est nul.

#### Scenario: Une ligne vide
- **WHEN** une ligne est laissée sans montant
- **THEN** elle reçoit l'opposé du résidu de la devise concernée

### Requirement: Montants et devises
Le système SHALL parser les montants en `Decimal`, avec EUR par défaut et la saisie « 10 USD » possible.

#### Scenario: Montant sans devise
- **WHEN** l'utilisateur saisit `12.50`
- **THEN** la devise est EUR

### Requirement: Tags et liens
Le système SHALL extraire les tags depuis les symboles `#` et les liens depuis `^`.

#### Scenario: Saisie de tags
- **WHEN** le champ tags contient `#vacances #2026`
- **THEN** la transaction porte les tags `vacances` et `2026`

### Requirement: Autocomplétion
Le système SHALL proposer les comptes ouverts à la date saisie et les tiers triés par fréquence.

#### Scenario: Compte fermé
- **WHEN** un compte est fermé à la date saisie
- **THEN** il n'est pas proposé

### Requirement: Mode édition
Le système SHALL pré-remplir le brouillon depuis une ligne du journal, les postings élidés revenant
avec un montant vide, et enregistrer par remplacement de la transaction d'origine.

#### Scenario: Enregistrer en édition
- **WHEN** l'utilisateur enregistre un brouillon issu du journal
- **THEN** une commande de modification est envoyée, jamais une suppression

### Requirement: Refus conservant le brouillon
Le système SHALL conserver le brouillon lorsque l'écriture est refusée et afficher le motif.

#### Scenario: Compte fermé refusé par le writer
- **WHEN** la pré-vérification refuse l'écriture
- **THEN** le ledger est inchangé et le brouillon reste éditable
