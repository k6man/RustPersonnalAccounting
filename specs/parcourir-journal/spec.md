# parcourir-journal Specification

## Purpose
Retrouver les transactions déjà enregistrées, puis les modifier ou les supprimer sans jamais toucher une autre ligne.

## Requirements

### Requirement: Liste des transactions
Le système SHALL afficher les transactions dans l'ordre du ledger avec date, tiers, libellé, comptes,
tags et flag, et indiquer le nombre total de transactions. L'affichage est limité aux 50 premières lignes
(`MAX_LIGNES`) ; la sélection ne peut pas dépasser cette limite.

#### Scenario: Plus de 50 transactions
- **WHEN** le ledger contient plus de 50 transactions
- **THEN** seules les 50 premières sont affichées et le total complet reste indiqué

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
