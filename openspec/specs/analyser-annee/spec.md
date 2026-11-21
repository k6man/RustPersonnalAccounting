# analyser-annee Specification

## Purpose
Comprendre l'année écoulée : dépenses par catégorie, comparaison avec l'an dernier, revenus, épargne, répartition et cascade.

## Requirements

### Requirement: Agrégats du rapport annuel
Le système SHALL calculer en EUR, par mois pour l'année courante : dépenses par catégorie (deux premiers
segments du compte), dépenses totales, revenus totaux, variation nette de l'épargne (comptes
`Assets:Epargne:*`, transferts et intérêts signés), et les dépenses mensuelles de l'année N−1.

#### Scenario: Catégorie à un seul segment
- **WHEN** un compte a moins de deux segments
- **THEN** il est agrégé sous son nom entier

#### Scenario: Aucune transaction l'an dernier
- **WHEN** l'année précédente est vide
- **THEN** la série N−1 est absente

### Requirement: Taux d'épargne
Le système SHALL calculer le taux d'épargne comme variation nette / revenus, et l'omettre si aucun revenu n'est enregistré.

#### Scenario: Aucun revenu
- **WHEN** les revenus de l'année valent zéro
- **THEN** le taux d'épargne est absent

### Requirement: Écran Rapports
Le système SHALL afficher une heatmap catégories × mois, la comparaison N vs N−1 en barres, les revenus,
la variation nette d'épargne et le taux d'épargne.

#### Scenario: Données présentes
- **WHEN** l'écran Rapports s'ouvre
- **THEN** les cinq éléments sont affichés

### Requirement: Répartition des dépenses
Le système SHALL afficher la répartition en barres horizontales avec bascule année / mois courant, les 8
premières catégories décroissantes et un agrégat « Autres (N postes) » pour que les parts somment à 100 %.

#### Scenario: Plus de huit catégories
- **WHEN** il existe plus de huit catégories
- **THEN** les suivantes sont regroupées dans « Autres »

### Requirement: Cascade annuelle
Le système SHALL afficher la cascade selon l'identité Revenus − Dépenses = Δ patrimoine (fin de ledger
contre fin N−1, Equity exclu), l'épargne nette étant une barre d'information séparée, non additive.

#### Scenario: Pas de double compte
- **WHEN** des intérêts sont crédités sur un compte d'épargne
- **THEN** ils ne sont comptés qu'une fois, dans les revenus

### Requirement: Budget vs réel groupé
Le système SHALL afficher le budget vs réel du mois en barres groupées, avec le même tri que le tableau de bord.

#### Scenario: Cohérence des écrans
- **WHEN** les deux écrans affichent le budget vs réel du mois
- **THEN** l'ordre des postes est identique

### Requirement: Mini-onglets des graphiques
Le système SHALL regrouper répartition, cascade et budget vs réel dans un écran à trois mini-onglets
navigables au clavier.

#### Scenario: Onglet suivant
- **WHEN** l'utilisateur change de mini-onglet
- **THEN** le graphique correspondant s'affiche
