# parcourir-journal Specification

## Purpose
Retrouver les transactions déjà enregistrées, puis les modifier ou les supprimer sans jamais toucher une autre ligne.

## Requirements

### Requirement: Liste des transactions
Le système SHALL afficher les transactions filtrées dans l'ordre du ledger avec date, tiers, libellé,
comptes, tags et flag, par pages de 100 lignes, et indiquer le total filtré et le total général.

#### Scenario: Plus de 50 transactions
- **WHEN** le ledger contient plus de 50 transactions
- **THEN** la limite fixe de 50 lignes n'existe plus et la première page affiche jusqu'à 100 lignes

#### Scenario: Plus de 100 transactions
- **WHEN** le ledger contient plus de 100 transactions
- **THEN** elles sont accessibles par pages successives

### Requirement: Navigation clavier et molette
Le système SHALL déplacer la sélection par les flèches et par la molette de souris, via les mêmes
fonctions de déplacement, bornée à la dernière ligne affichée.

#### Scenario: Molette
- **WHEN** l'utilisateur fait défiler la molette vers le bas
- **THEN** la sélection descend, bornée à la dernière ligne affichée

### Requirement: Modification par empreinte
Le système SHALL ouvrir la transaction sélectionnée dans la Saisie (Entrée) et la relocaliser par
empreinte (date, tiers, libellé, montant du plus grand posting) au moment de l'écriture, afin de ne
jamais réécrire une autre ligne, y compris pour deux transactions jumelles le même jour.

#### Scenario: Directives avant les transactions
- **WHEN** des directives non-transactions précèdent les transactions dans le ledger
- **THEN** l'action vise la bonne ligne car la position d'affichage n'est pas confondue avec l'index du ledger

#### Scenario: Empreinte introuvable
- **WHEN** le ledger a changé et l'empreinte ne correspond plus
- **THEN** l'opération est refusée sans écriture

### Requirement: Suppression avec confirmation
Le système SHALL demander une confirmation modale avant de supprimer (Suppr) : Entrée confirme, toute
autre touche annule, un clic gauche annule, et Ctrl+C reste prioritaire. La position d'affichage est
stockée dans la modale et l'index du ledger n'est résolu qu'à la confirmation.

#### Scenario: Annulation
- **WHEN** l'utilisateur presse une autre touche qu'Entrée dans la modale
- **THEN** aucune suppression n'a lieu

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
