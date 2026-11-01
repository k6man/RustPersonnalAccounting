# Comptabilité du foyer

Comptabilité personnelle de foyer (couple + 2 enfants, EUR) fondée sur **Beancount**,
moteur **rustledger** (implémentation Rust), suivi **git** natif, et une GUI **egui/eframe**.

> **Suivi du projet** : voir [`TODO.md`](TODO.md) — état courant, jalons, décisions.
> C'est le point d'entrée pour reprendre le travail à froid.
> Les décisions d'architecture sont consignées dans [`docs/`](docs/) (ADR).

## État

| Jalon | Contenu | État |
|---|---|---|
| 1 | Workspace + ledger d'exemple valide | ✅ |
| 2 | `household-core` (lecture/écriture ledger) | ✅ |
| 3 | `household-budget` (grille, rollover, duplication) | ✅ |
| 4 | GUI lecture seule (dashboard, journal) | ✅ |
| 5 | Saisie/écriture de transactions | ✅ |
| 6 | Éditeur de budget postes × 12 mois | ✅ |
| 7 | Rapports, git dans la GUI, polish | ✅ |

## Installation

```sh
# Toolchain Rust (pinned via rust-toolchain.toml)
brew install rust just ripgrep
# Moteur beancount Rust (CLI + hook pre-commit + CI)
brew install rustledger
# Hooks
just setup
```

## Lancer

```sh
cargo run -p household-gui   # GUI
just check                   # fmt + clippy + tests + validation du ledger
just test                    # cargo test --workspace
rledger check ledger/main.beancount   # validation ledger seule
```

## Les écrans

- **Tableau de bord** : soldes des comptes clés, jauges de plafond à 3 états
  (normal / ≥ 90 % / dépassement), patrimoine net (Σ bilan — Equity exclu),
  budget vs réel du mois courant, courbe du patrimoine par fin de mois.
- **Journal** : toutes les transactions lisibles ; double-clic pour modifier,
  bouton pour supprimer (relocalisation par empreinte : la suppression
  visée ne peut pas toucher une autre ligne).
- **Saisie** : postings dynamiques avec contrepartie explicite (ligne
  d'équilibrage écrite explicitement), autocomplétion des comptes ouverts à
  la date et des tiers par fréquence, tags/liens ; pré-vérification sur
  copie scratch du ledger avant écriture atomique.
- **Budget** : grille postes × 12 mois avec **arbre hiérarchique repliable**
  (les groupes sans directive propre sont synthétisés : agrégat = somme des
  lignes budgétées de tête, chaque budget compté une fois), cellules
  éditable avec confirmation du diff (modification in place ou supersession
  datée), duplication d'année N+1 (pour cent ±, pas d'arrondi), rollover par
  famille (disponible cumulé ; réglage en TOML local, jamais dans le ledger).
- **Rapports** : dépenses par catégorie × mois (heatmap), comparaison N vs
  N−1 en barres, revenus, variation nette d'épargne, taux d'épargne.
- **Graphiques** : un seul écran à trois mini-onglets — répartition des
  dépenses en barres horizontales (bascule année / mois courant, top 8 +
  agrégat « Autres »), cascade annuelle suivant l'**identité comptable**
  Revenus − Dépenses = Δ patrimoine (l'épargne nette, non additive, est en
  barre d'information à part — pas de double-compte des intérêts),
  budget vs réel du mois en barres groupées. Raccourcis Ctrl+1 à Ctrl+7.
- **Git** : statut des fichiers modifiés, résumé du diff, message de commit
  suggéré (`ledger: AAAA-MM — saisie/édition…`), historique filtré sur le
  ledger. Le commit valide le ledger avant d'indexer (ADR-002).

## Structure

```
/ledger                     # dépôt de comptabilité, texte, versionné (source de vérité)
  main.beancount            # options + includes (globs par année/budget)
  accounts.beancount        # open/close + plan de comptes (noms français, sans accents)
  commodities.beancount
  /budgets/2026.beancount   # budgets : custom "budget" (convention Fava)
  /transactions/2025…       # un fichier par mois ; croisement par date
/crates
  /household-core           # wrapper rustledger : load, snapshot, writer transactions (atomique)
  /household-budget         # budget pur : grille postes×12 mois, rollover, duplication d'année
  /household-git            # git2 : statut, commit (avec revalidation), historique
  /household-gui            # eframe/egui : état applicatif / commandes worker / vues
/docs                       # ADR (décisions d'architecture)
```

## Conventions

- **Comptes** : noms en français, sans accents dans les identifiants (limite
  Beancount) ; libellés, narrations et UI en français.
  Exemples : `Assets:Banque:CompteCourant`, `Assets:Epargne:LivretA`,
  `Expenses:Alimentation:Courses`, `Expenses:Logement:Loyer`,
  `Income:Salaire:Jeanne`.
- **Chiffres** : `rust_decimal::Decimal` de bout en bout (jamais `f64` pour
  un montant ; conversion `f64` uniquement à la frontière d'un tracé) ;
  format ledger `"1234.56 EUR"`.
- **Dates** : `rustledger_core::NaiveDate` (jiff) — pas de chrono.
- **Plafonds épargne** : métadonnée sur la directive `open`
  (`ceiling: 22950.00 EUR`) — Livret A 22 950 €, LDD 12 000 €.
- **Budgets** : `custom "budget"` Fava-compatible, une directive par ligne,
  un fichier par année :

  ```beancount
  2026-01-01 custom "budget" Expenses:Alimentation:Courses "monthly" 850.00 EUR
  2026-07-01 custom "budget" Expenses:Alimentation:Courses "monthly" 900.00 EUR ; supersession datée
  ```
  Le budget d'épargne vise les comptes `Assets:Epargne:*` destinataires
  (caveat : les intérêts annuels crédités sur le compte gonflent son
  « réel » — signalé dans l'éditeur). Aucune directive sur
  `main.beancount` : les includes budgets y sont ajoutés une fois, les
  fichiers budgétaires sont la zone d'écriture.
- **Transactions** : suppression = réécriture du bloc canonique entier ;
  édition de budget = writer chirurgical (token montant seul, round-trip
  identique à valeur égale).
- **Écriture** : toujours atomique (fichier temporaire + rename), format
  canonique stable, aligné, diffs git propres.
- **Versionnement** : tout commit (hook pre-commit ou écran Git) revalide le
  ledger ; message type `ledger: AAAA-MM — saisie/édition`.
- **Licence** : GPL‑3.0 (situation de dépendance vis‑à‑vis de rustledger,
  GPL‑3.0‑only).
