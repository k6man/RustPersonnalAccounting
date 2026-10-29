# TODO.md — Suivi du projet

> **Rôle de ce fichier** : c'est le journal de bord du projet, versionné avec le code.
> Il doit permettre à quelqu'un (ou un autre agent) de reprendre le travail **à froid** :
> l'état actuel et la prochaine action doivent être évidents en le lisant.
>
> **Conventions** :
> - `## En cours` : la tâche active (une seule idéalement), avec son contexte de reprise.
> - `## À faire` : backlog par jalon (1 → 7), cases `- [ ]`, tâches granulaires (≤ quelques heures).
> - `## Fait` : tâches terminées `- [x]` + date (+ hash du commit si disponible).
> - Mise à jour **à chaque jalon et à chaque tâche significative**, dans le **même commit** que le code concerné.
> - Les tâches découvertes en cours de route sont ajoutées immédiatement, pas perdues.

**État du projet** : étape 0 (reconnaissance) terminée. Plan validé par le maître d'ouvrage
(licence GPL‑3.0 acceptée ; budget d'épargne sur les comptes `Assets` destinataires accepté).
Aucun code écrit pour l'instant. **Prochaine action : jalon 1.**

---

## En cours

- [ ] **Jalon 1 — Squelette workspace + ledger d'exemple valide**
  - Contexte de reprise : rien n'est encore créé à part `prompt.md` et ce fichier (branche `develop`).
    Les versions/choix techniques sont consolidés dans « Questions ouvertes / limites » et la
    section Reconnaissance plus bas. Commencer par la structure du dépôt du plan, puis le ledger,
    puis l'outillage (justfile, hooks, CI).

## À faire

### Jalon 1 — Squelette workspace + ledger d'exemple

- [ ] `Cargo.toml` workspace : membres `crates/household-{core,budget,git,gui}`, `edition = "2021"` (ou 2024), dépendances racine épinglées : `rustledger-* =0.24.0`, `eframe/egui/egui_extras =0.37`, `egui_plot =0.38`, `rust_decimal =1.43`, `git2 =0.21`, `notify =8.2`, `thiserror =2`, `anyhow =1`, `serde`, `toml` ; `rust-toolchain.toml` (stable ≥ 1.9x, MSRV rustledger).
- [ ] `LICENSE` GPL‑3.0 + mentions de licence dans les en-têtes de crates.
- [ ] `.gitignore` (`/target`, `.env`, artefacts) + `.gitattributes` (`*.beancount text eol=lf`, `*.rs text eol=lf`).
- [ ] `justfile` : `fmt`, `clippy` (`-D warnings`), `test`, `check` (tout), `check-ledger` (validation du ledger via `rledger`, repli `cargo run -p household-cli -- check`), `run` (GUI).
- [ ] `/ledger/main.beancount` : options (`operating_currency "EUR"`), includes vers accounts/commodities/budgets/transactions.
- [ ] `/ledger/commodities.beancount` : EUR (+ propriétés éventuelles).
- [ ] `/ledger/accounts.beancount` : plan de comptes complet en français sans accents —
  `Assets:Banque:CompteCourant`, `Assets:Epargne:LivretA` (métadonnée `ceiling: 22950.00 EUR`),
  `Assets:Epargne:LDD` (`ceiling: 12000.00 EUR`), `Income:Salaires:Conjoint-1|Conjoint-2`,
  `Income:Allocations-Familiales`, `Income:Interets:Livret-A|LDD`, `Income:Remboursements-Sante`,
  `Income:Autres`, `Equity:Soldes-Ouverture`, et l'arborescence `Expenses:*` du cahier des charges
  (Logement, Alimentation, Transport, Enfants, Sante, Loisirs, Vacances, Impots, Assurances,
  Frais-Bancaires, Cadeaux, Habillement, Soins-Personnels, Divers).
- [ ] `/ledger/budgets/2026.beancount` : grille budgétaire 2026 complète en `custom "budget"`
  (postes Expenses + objectifs d'épargne sur `Assets:Epargne:*`), avec variantes datées en cours
  d'année (ex. Courses 850→900 au 07‑01) et remises à zéro `0.00 EUR` (Vacances hors été).
- [ ] `/ledger/transactions/2025/bilan.beancount` : ouverture 2025‑01‑01 (CC, LivretA, LDD) +
  intérêts annuels 2025‑12‑31 (`Income:Interets:*`) + assertions de solde.
- [ ] `/ledger/transactions/2026/01.beancount`, `02.beancount`, `03.beancount` : 3 mois réalistes
  (salaires, loyer, charges, courses, cantine, carburant, mutuelle, transferts CC→LivretA/LDD,
  remboursements santé, prélèvement à la source, tags/liens d'exemple).
- [ ] Validation : `rledger check ledger/main.beancount` sans erreur (installer via `brew install rustledger`).
- [ ] `/githooks/pre-commit` : validation du ledger (rledger, repli household-cli dès le jalon 2) + `just fmt --check` + clippy rapide sur les crates modifiés ; activation documentée `git config core.hooksPath githooks` (+ recette automatique dans `just setup`).
- [ ] CI GitHub Actions `.github/workflows/ci.yml` : fmt, clippy `-D warnings`, `cargo test`, validation du ledger (`cargo binstall rustledger` puis `rledger check`).
- [ ] `README.md` initial : présentation, structure, installation (rust, rledger), `just`, rôle du TODO.md.
- [ ] Commit jalon 1 + basculer les tâches faites + remplir la section Reconnaissance en « Fait ».

### Jalon 2 — household-core : lecture et écriture du ledger

- [ ] `household-core` : wrapper du chargement — `LedgerSnapshot` immuable (`Arc`) construit sur
  `rustledger_loader::load(path, &LoadOptions)` ; exposer : directives sourcées (`Spanned<Directive>`
  + `SourceMap` → fichier/ligne), balances par compte (`balance_view()` + `Inventory`), options,
  `Ledger.errors` typés (`thiserror`), arbre des comptes, plafonds lus dans les métadonnées `open`.
- [ ] Types domaine : `Date = rustledger_core::NaiveDate` (jiff), `Decimal = rust_decimal` — ré-exports,
  pas de chrono, pas de conversions aux frontières.
- [ ] API requêtes minimales pour la GUI : soldes au jour J, écritures par période, filtres (compte,
  substring, tag), comptes ouverts à une date (autocomplétion), payees fréquents.
- [ ] **Writer transactions** : génération de bloc canonique via `rustledger_core::format` ; append
  trié par date (`canonical_sort_key`) dans `transactions/AAAA/MM.beancount` (créé si absent + include
  dans `main.beancount` — réfléchir au glob d'include pour éviter l'édition de main) ; édition =
  remplacement de l'exact span source ; suppression = coupe du bloc + revalidation ; écriture atomique
  (temp + `rename`), verrou d'écriture.
- [ ] `household-cli` (bin) : sous-commandes `check` (exit code + erreurs typées), `balances` — sert
  au hook pre-commit et à la CI sans dépendance au binaire rledger.
- [ ] Tests d'intégration **golden files** : snapshot du ledger d'exemple (balances attendues) ;
  écriture d'une transaction dans un ledger temporaire → relecture → contenu exact attendu.
- [ ] Commit jalon 2.

### Jalon 3 — household-budget : logique budgétaire pure

- [ ] Modèle : `BudgetGrid { year, postes × 12 mois }`, construit à partir de
  `rustledger_budget::Budgets::from_directives(&ledger.directives)` et du calcul des réels par
  fenêtre mensuelle (`compare(directives, types, from, to, children, filter)` par mois).
- [ ] Lignes : budget mensualisé, réel, écart, % consommé (via `BudgetRow::remaining/used_fraction`) ;
  totaux par ligne (année), par colonne (mois), par groupe hiérarchique (`children = true`).
- [ ] **Rollover** (option, calcul maison non fourni par rustledger) : report du non‑dépensé/cumulé
  mois → mois suivant ; sémantique « cumul annuel (+ report) vs réel cumulé » verrouillée par tests.
- [ ] Remise à zéro : gestion des directives `0.00 EUR` (la grille doit les écrire, pas les supprimer).
- [ ] **Duplication d'année** N→N+1 : copie des directives avec application optionnelle d'un pourcentage,
  arrondi aux centimes supérieurs multiples « ronds » paramétrable ; sortie = liste de lignes canoniques.
- [ ] Writer budget chirurgical : une directive = une ligne dans `budgets/AAAA.beancount`, tri
  (date, compte), mise à jour d'une cellule = édition d'une seule ligne (ou insertion), **préservation
  des commentaires** ; écriture atomique.
- [ ] Avertissements : parsing via `rustledger_budget::BudgetError` remonté typé (currency mismatch, etc.).
- [ ] Tests unitaires : février bissextile (dénominateur 28/29), supersession en cours de mois
  (split d'interval), budget mensuel vs directive datée en milieu de mois, rollover cumulé,
  groupement parent+enfants (addition, pas absorption), duplication ±% idempotente, golden file
  budgets/2026.beancount (round‑trip lecture→écriture = contenu identique).
- [ ] Commit jalon 3.

### Jalon 4 — GUI lecture seule (dashboard + journal)

- [ ] Squelette `household-gui` eframe : `AppState` (snapshots `Arc`, pas de logique métier), enum
  `Command` (Reload, SaveTransaction, …), **worker thread unique** de rechargement/validation,
  canaux requête/réponse, `request_repaint` à réception ; écrans via onglets, libellés en français.
- [ ] Rechargement au démarrage + **watcher `notify`** sur `/ledger` (débounced) → reload hors UI thread.
- [ ] Écran **Tableau de bord** : soldes CC/LivretA/LDD, jauges de plafond (alerte ≥ 90 %, bloquant visuel
  au dépassement), patrimoine net, carte budget vs réel du mois courant, courbe d'épargne (`egui_plot`).
- [ ] Écran **Journal** : `egui_extras::TableWidget` paginé, filtres date/compte/montant/tag, recherche,
  sélection → détail de l'écriture avec son origine fichier:ligne.
- [ ] Affichage des erreurs de validation du ledger dans une bannière (codes `LedgerError`).
- [ ] Commit jalon 4.

### Jalon 5 — Saisie et écriture de transactions

- [ ] Écran **Saisie** : date, payee, narration, postings dynamiques (ajout/retrait de lignes,
  solde automatique sur la contrepartie), montants en `Decimal`, autocomplétion comptes (ouverts à
  la date) et payees (fréquence), tags `#` et liens `^`.
- [ ] Validation en direct avant écriture : construction d'un `Ledger` enrichi en mémoire (ou
  re-validation post‑écriture systématique avec rollback si échec) — choix documenté dans la UI.
- [ ] Écriture dans le bon fichier mensuel via le writer du jalon 2 + déclenchement reload/watcher.
- [ ] Édition/suppression depuis le Journal : remplacement du span exact, confirmation, revalidation
  avant/d'après écriture.
- [ ] Test bout en bout : saisie GUI (test headless de la commande) → fichier → snapshot → dashboard.
- [ ] Commit jalon 5.

### Jalon 6 — Éditeur de budget

- [ ] Écran **Budget** : grille postes × 12 mois éditable (`TableWidget`), année selectable,
  indicateurs d'écart par cellule (réel affiché en dessous), totaux ligne/colonne, groupes hiérarchiques
  repliables.
- [ ] Écriture d'une cellule → directive via le writer chirurgical du jalon 3 ; affichage du diff attendu.
- [ ] Duplication d'année avec ajustement % (dialogue), bouton « générer l'année N+1 ».
- [ ] Lignes d'épargne (budgétées sur `Assets:Epargne:*`) affichées avec la note sur les intérêts.
- [ ] Toggles rollover par famille/groupes, persistés où ? (voir Questions ouvertes #6).
- [ ] Commit jalon 6.

### Jalon 7 — Rapports, git, polish

- [ ] Écran **Rapports** : dépenses par catégorie × mois (heatmap/tableau), comparaison N vs N‑1
  (`egui_plot` barres), revenus, taux d'épargne.
- [ ] `household-git` (git2) : statut par fichier, stage + commit programmatique (message proposé :
  `ledger: AAAA-MM — saisie/édition…` avec compte des changements), historique du ledger (log + paths),
  résumé de diff ; **validation du ledger avant commit** côté app (indépendante du hook, car git2
  n'exécute pas les hooks — c'est le choix retenu, justifié dans /docs).
- [ ] Écran **Git** : statut, diff résumé, commit, historique simple.
- [ ] Gestion d'erreurs UI homogène (chaînes `thiserror`), toasts, écran d'erreur non fatal.
- [ ] README complet : installation, lancement, conventions de nommage (comptes FR sans accents),
  format des budgets (exemples), workflow git (hook + GUI), rôle du TODO.md.
- [ ] `/docs` : ADR (choix git2 vs binaire git, sémantique budget Fava, `ceiling:` en métadonnées,
  licence GPL‑3).
- [ ] CI complète verte + tag `v0.1.0`.
- [ ] Commit jalon 7.

## Fait

- [x] Étape 0 — reconnaissance rustledger (2026‑10‑29) : lecture du dépôt `main` (3 434 commits) et
  crates.io — conclusion dans « Reconnaissance » ci‑dessous. **Décisions validées** : licence GPL‑3.0 ;
  budget d'épargne sur les comptes `Assets` destinataires.
- [x] Plan des jalons 1→7 présenté et accepté (2026‑10‑29).
- [x] `TODO.md` initialisé (ce fichier).

### Reconnaissance — capacités vérifiées de rustledger 0.24.0 (publié 2026‑09‑06)

- Crates : `rustledger-{core,parser,loader,booking,validate,query,plugin,budget,returns,ops,importer,completion,lsp,wasm}` + CLI `rustledger` (`rledger check|query|format|report budget|extract|…`, `brew install rustledger`).
- `custom` : `Directive::Custom(Custom { date, custom_type, values: Vec<MetaValue>, meta })` (core/directive.rs) ; sérialisation préservée (`format_custom_lines`, `format_directives` dans core/src/format).
- `rustledger-budget` : modèle Fava complet — `read_budget`, `Budgets::from_directives`, `accrue`,
  `compare(directives, types, from, to, children, filter) -> Comparison` ; accrual par jour avec
  dénominateur calendrier exact, supersession par `(account, currency)`, ancrage calendaire,
  agrégation hiérarchique (`covers`), non‑rétroactivité. Le **rollover n'est pas fourni** → jalon 3.
- Loader : `load(path, &LoadOptions) -> Ledger` = includes + plugins (33 natifs) + booking (7 méthodes)
  + validation ; `Ledger { directives: Vec<Spanned<Directive>>, options, errors, source_map,
  display_context, … }`, `balance_view()` pour la vue comptable des pads ; `canonical_sort_key`.
- Types : `Decimal = rust_decimal`, `NaiveDate = jiff::civil::Date` (chrono écarté).
- BQL : `rustledger-query` (moteur complet) — dispo mais les rapports maison itèrent `Ledger.directives`.
- MSRV : stable récente (repo épinglé 1.98.1). Licence : GPL‑3.0‑only (+ option commerciale).

## Questions ouvertes / limites

1. **Licence GPL‑3.0** — *décision* : acceptée (2026‑10‑29, maître d'ouvrage). Conséquence : l'app liée
   à rustledger est GPL‑3 ; usage personnel libre, divulgation du source si distribution.
2. **Pas de budget « natif » pour l'épargne** (transfert Assets→Assets) — *décision* : `custom "budget"`
   posé directement sur `Assets:Epargne:LivretA/LDD` (convention Fava, supportée par `Bucket`/
   `normalized_actual` de rustledger-budget). **Caveat** : les intérêts annuels crédités sur le même
   compte gonflent le « réel » de l'objectif de versement (≈ 1 directive/an, écart mineur). Alternative
   écartée : compte de transit fictif (pollue le bilan).
3. **Rollover absent de rustledger-budget** — *décision* : calculé dans `household-budget` (option,
   cumulée), tests dédiés (jalon 3).
4. **Budget sans héritage inter‑années** : la supersession étant datée, recopier une année est une
   opération d'édition, fournie par l'outil de duplication (jalon 3) — pas une règle du moteur.
5. **Plafonds d'épargne** : aucun standard Beancount. *Décision* : métadonnée sur la directive `open`
   (`ceiling: 22950.00 EUR`) — préservée par parseur/sérialiseur, compatible `rledger`/Fava ; une
   `custom "ceiling"` maison aurait été une extension non standard. À consolider dans un ADR (/docs, jalon 7).
6. **Persistance des réglages GUI** (toggles rollover, dernière année ouverte…) : ledger = source de
   vérité pour les données ; les préférences UI vivront dans un TOML de config local (`~/.config/…`),
   jamais dans le ledger. À confirmer au jalon 6.
7. **Include des fichiers mensuels** : pour que le writer crée un fichier de mois sans éditer
   `main.beancount`, prévoir un include par année (`transactions/2026/*.beancount` ou un
   `transactions/2026.beancount` qui include les mois, selon le support des globs du loader — vérifié
   : `is_glob_pattern` existe dans rustledger-loader). À trancher au jalon 1 et documenter ici.
8. **rustledger évolue vite** (0.24.0, licence commerciale en parallèle) : dépendances épinglées
   `=0.24.0` ; un `cargo update` ne se fait qu'après lecture du CHANGELOG, avec tests golden en garde‑fou.

## Idées / hors périmètre v1

- Import CSV/OFX bancaire via `rledger extract` (+ `importers.toml`) et son import dans la GUI.
- Multi‑devises / investissement (déjà couverts par le moteur, hors besoin du foyer).
- Comptes individuels (si le contexte change) ; comptes titres/PEA.
- Rapports d'amortissement/retours (`rustledger-returns`, XIRR) et capital gains.
- Cohabitation avec Fava (les budgets sont déjà dans sa convention — argument de compatibilité).
- Rôles/conjoint : signature des commits, second fichier de préférences.
- Export PDF des rapports ; widgets système ; notifications push du dépassement de plafond.
