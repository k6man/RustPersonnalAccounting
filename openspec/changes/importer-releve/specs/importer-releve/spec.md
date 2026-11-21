## ADDED Requirements

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
