# importer-releve Specification

## Purpose

Gérer l'import de relevés bancaires (CSV/OFX) vers le ledger : extraction des transactions, détection des doublons, prévisualisation et écriture validée via les writers standards. Inclut la configuration d'un profil d'import par défaut pour l'utilisateur.

## Requirements

### Requirement: Import de relevé
Le système SHALL transformer un relevé CSV ou OFX en brouillons de transactions proposés à
l'utilisateur, sans rien écrire avant validation explicite.

#### Scenario: Relevé valide
- **WHEN** l'utilisateur importe un relevé
- **THEN** chaque ligne devient un brouillon à accepter, corriger ou ignorer

### Requirement: Détection des doublons
Le système SHALL signaler tout brouillon dont la date, le montant et le libellé correspondent à une
transaction existante du ledger.

#### Scenario: Réimport du même relevé
- **WHEN** un relevé déjà importé est réimporté
- **THEN** les lignes déjà présentes sont marquées comme doublons et décochées par défaut

### Requirement: Écriture par le chemin standard
Le système SHALL écrire les transactions acceptées via les writers de garantir-integrite-ecritures, avec
pré-vérification, atomicité et revalidation.

#### Scenario: Ligne refusée par la pré-vérification
- **WHEN** une transaction acceptée référence un compte fermé
- **THEN** elle est refusée, le ledger reste inchangé et le motif est affiché

### Requirement: Profil d'import par défaut configurable
Le système SHALL permettre à l'utilisateur de définir un profil d'import par défaut dans les préférences applicatives, qui sera pré-sélectionné à l'ouverture de l'écran Import s'il existe parmi les profils chargés.

#### Scenario: Profil par défaut présent dans la liste
- **WHEN** l'utilisateur ouvre l'écran Import et un `profil_import_defaut` est configuré et correspond à un profil chargé
- **THEN** ce profil est sélectionné automatiquement (index correspondant)

#### Scenario: Profil par défaut absent de la liste
- **WHEN** l'utilisateur ouvre l'écran Import et un `profil_import_defaut` est configuré mais ne correspond à aucun profil chargé
- **THEN** le premier profil de la liste est sélectionné (comportement actuel conservé)

#### Scenario: Aucun profil par défaut configuré
- **WHEN** l'utilisateur ouvre l'écran Import et aucun `profil_import_defaut` n'est configuré
- **THEN** le premier profil de la liste est sélectionné (comportement actuel conservé)
