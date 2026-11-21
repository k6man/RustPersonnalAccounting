# ADR-005 — Conventions du ledger : plafonds, includes, assertions, budgets inter-années, préférences

**Statut** : acceptée — décisions prises aux jalons 1, 3 et 6 (octobre 2026), consignées ici lors du
retrait de `TODO.md`. Complète ADR-003 (budget, rollover, épargne, patrimoine net).

## Décisions

### 1. Plafonds d'épargne : métadonnée `ceiling:` sur la directive `open`
Aucun standard beancount n'existe pour les plafonds. Une métadonnée sur `open`
(`ceiling: 22950.00 EUR`) est préservée par le parseur et le sérialiseur et reste compatible `rledger` et
Fava ; une `custom "ceiling"` maison aurait été une extension non standard. Les jauges se calculent sur le
solde réel, jamais sur un budget.

### 2. Fichiers mensuels : includes par globs annuels
`main.beancount` inclut des **globs par année** (`transactions/2025/*.beancount`,
`transactions/2026/*.beancount`). Le loader garantit un ordre déterministe et tolère l'absence de
correspondance : aucun outil n'a besoin d'éditer `main.beancount`, un fichier mensuel nouveau est créé et
référencé implicitement. Les budgets, eux, sont inclus **explicitement** par année, pour que leurs erreurs
restent visibles.

### 3. Assertions `balance` : datées du 1er du mois suivant
Sémantique beancount v3 : un `balance` à la date D est vérifié à **00:00 de D**, donc strictement avant
toute transaction de D. Convention adoptée : les assertions mensuelles sont **datées du 1er du mois
suivant**, et le writer la respecte. La directive `commodity` exige une date.

### 4. Pas d'héritage de budget entre années
La supersession est datée : recopier une année est une **opération d'édition** (outil de duplication,
`household-budget::duplicate`), pas une règle du moteur. La duplication refuse d'écraser une année déjà
budgétée.

### 5. Préférences de l'application hors du ledger
Le ledger est la seule source de vérité des données. Les réglages de l'application (chemin du vrai ledger,
rollover par famille…) vivent dans `~/.config/rust-personal-accounting.toml`, jamais dans le ledger.

## Conséquences

- Un changement de l'une de ces conventions est un changement de format : il passe par un change OpenSpec
  (specs `garantir-integrite-ecritures`, `piloter-budget`, `regler-application`).
- Les specs OpenSpec décrivent ces comportements ; cet ADR en garde la justification.
