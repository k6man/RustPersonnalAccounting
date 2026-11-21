## ADDED Requirements

### Requirement: Filtres du journal
Le système SHALL filtrer par texte (tiers + libellé, insensible à la casse), préfixe de compte (n'importe
quel posting), tag exact, bornes de dates et bornes de montant, toutes inclusives, avec parsing tolérant ;
un champ vide désactive son filtre.

#### Scenario: Filtre combiné
- **WHEN** un texte et une borne de date sont actifs
- **THEN** seules les transactions satisfaisant les deux sont affichées

### Requirement: Pagination du journal
Le système SHALL paginer par pages de 100 lignes, avec au moins une page même vide, et recaler la page
courante lorsque le total se réduit.

#### Scenario: Filtre réduisant le total
- **WHEN** un filtre réduit le nombre de pages sous la page courante
- **THEN** la page courante est ramenée à la dernière page valide

## MODIFIED Requirements

### Requirement: Liste des transactions
Le système SHALL afficher les transactions filtrées dans l'ordre du ledger avec date, tiers, libellé,
comptes, tags et flag, par pages de 100 lignes, et indiquer le total filtré et le total général.

#### Scenario: Plus de 50 transactions
- **WHEN** le ledger contient plus de 50 transactions
- **THEN** la limite fixe de 50 lignes n'existe plus et la première page affiche jusqu'à 100 lignes

#### Scenario: Plus de 100 transactions
- **WHEN** le ledger contient plus de 100 transactions
- **THEN** elles sont accessibles par pages successives
