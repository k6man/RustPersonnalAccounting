# consulter-tableau-de-bord Specification

## Purpose
Voir d'un coup d'œil où en est le foyer : soldes clés, plafonds d'épargne, patrimoine net, budget du mois et tendance.

## Requirements

### Requirement: Soldes des comptes clés
Le système SHALL afficher les soldes en fin de ledger des comptes clés (compte courant, Livret A, LDD).

#### Scenario: Ledger d'exemple
- **WHEN** le ledger d'exemple est chargé
- **THEN** les soldes reflètent le dernier jour transactionné

### Requirement: Jauges de plafond à trois états
Le système SHALL afficher pour chaque compte plafonné une jauge d'état « normal » (< 90 %), « alerte »
(90–100 %) ou « dépassement » (> 100 %), calculée sur le solde réel et jamais sur un budget.

#### Scenario: Seuil d'alerte
- **WHEN** un compte atteint 90 % de son plafond
- **THEN** la jauge passe à l'état alerte

#### Scenario: Remplissage borné
- **WHEN** le solde dépasse le plafond
- **THEN** la fraction affichée est plafonnée à 1 et l'état est dépassement

### Requirement: Patrimoine net
Le système SHALL calculer le patrimoine net comme Σ Assets − Σ Liabilities en EUR, Equity exclu.

#### Scenario: Equity d'ouverture
- **WHEN** le compte d'ouverture porte le capital historique
- **THEN** il n'entre pas dans le patrimoine net

### Requirement: Budget vs réel du mois courant
Le système SHALL afficher le budget vs réel du mois courant trié par postes les plus tendus (dépassements
d'abord), avec total budgété, total réel et reste à dépenser (négatif si dépassement global).

#### Scenario: Poste en dépassement
- **WHEN** un poste dépasse son budget
- **THEN** il apparaît avant les postes sous budget

### Requirement: Courbe du patrimoine
Le système SHALL tracer le patrimoine net par fin de mois, du premier au dernier mois transactionné, sur
24 points au plus, calculé en une passe incrémentale O(transactions).

#### Scenario: Historique long
- **WHEN** le ledger couvre plus de 24 mois
- **THEN** seuls les 24 derniers points sont affichés
