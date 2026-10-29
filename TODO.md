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

**État du projet** : jalon 1 terminé et commité (`30cc0ab`). Ledger d'exemple validé
(`rledger check` ✓), workspace compilé, fmt/clippy/tests verts.
**Prochaine action : jalon 2** (`household-core`).

---

## En cours

- [ ] **Jalon 2 — household-core : lecture et écriture du ledger**
  - Contexte de reprise : lire d'abord « Reconnaissance » (API rustledger 0.24 vérifiées).
    Clé de voûte : `rustledger_loader::load(path, &LoadOptions) -> Ledger` (directives
    `Spanned<Directive>` + `SourceMap` pour localiser fichier/ligne, `balance_view()` +
    `Inventory` pour les soldes). Écriture : append canonique via `rustledger_core::format`
    dans `transactions/AAAA/MM.beancount` (globs inclus de main — jamais éditer main),
    replace/remove par span exact, temp+rename. Binaire `household-cli` (sous-commandes
    `check`, `balances`) appelé en repli par `githooks/pre-commit` + `just check-ledger`.
    Types : `rustledger_core::NaiveDate` (jiff 0.2), `rust_decimal::Decimal`.

## À faire

### Jalon 1 — Squelette workspace + ledger d'exemple

- [x] `Cargo.toml` workspace : membres `crates/household-{core,budget,git,gui}`, `edition = "2024"`,
  dépendances épinglées `rustledger-* =0.24.0`, eframe/egui/egui_extras 0.37, egui_plot 0.38,
  `rust_decimal 1.43`, `jiff 0.2` (chrono écarté), lints workspace (`unsafe_code = forbid`).
- [x] `LICENSE` GPL‑3.0 (texte repris de rustledger) ; conteneurs de crate à licence workspace.
- [x] `.gitignore` (`/target`, cache rledger `*.cache`, `Cargo.lock` versionné) + `.gitattributes` (LF forcé, `*.beancount` inclus).
- [x] `justfile` : `setup` (hooksPath + chmod hook), `fmt`, `clippy -D warnings`, `test`, `check-ledger`
  (rledger, repli household-cli J2+), `check`, `run`.
- [x] `/ledger/main.beancount` : options + **includes globs** (`transactions/2025/*.beancount`,
  `transactions/2026/*.beancount`) + budgets explicites — voir Q7 tranchée plus bas.
- [x] `/ledger/commodities.beancount` : `commodity EUR` (avec date directive obligatoire).
- [x] `/ledger/accounts.beancount` : plan de comptes complet FR sans accents, plafonds en
  métadonnées `open` (`ceiling: 22950.00 EUR` etc. — validé par rustledger, MetaValue::Amount OK).
- [x] `/ledger/budgets/2026.beancount` : 40+ directives `custom "budget"` (fixes 2026‑01‑01,
  supersessions datées : Courses 850→900 au 07‑01, Taxe-Auto mars, Taxe-Foncière octobre,
  fournitures septembre), **Vacances : une directive/mois avec `0.00 EUR` de remise à zéro**,
  épargne budgétée sur `Assets:Epargne:*`.
- [x] `/ledger/transactions/2025/bilan.beancount` : ouverture (2845.23 CC / 18450.12 LivretA /
  9870.55 LDD via `Equity:Soldes-Ouverture`), intérêts 2025‑12‑31 (552.37 / 296.44, non imposables).
- [x] `/ledger/transactions/2026/01–03.beancount` : 3 mois réalistes (salaires 2350/1890, CAF 155,
  loyer+charges, factures, mutuelle, DGFiP, courses, cantine/périscolaire, carburant, virement
  épargne 300/mois, santé+remboursements, tags/links, une `!` en attente), **assertions mensuelles
  vérifiées à 00:00 (sémantique v3 → datées le 1er du mois suivant)**.
- [x] Validation : `rledger check ledger/main.beancount` ✓ sans erreur ; `rledger report budget
  --children` ✓ (le budget Assets/Expenses, supersession et pro‑rating se lisent correctement).
- [x] `/githooks/pre-commit` : rangé exécutable dans git (mode 100755) — validation rledger si des
  .beancount sont indexés, repli `household-cli` (J2). NB : bit exécutable **local** à refaire via
  `just setup` (sandbox limitée, voir Q11).
- [x] CI `.github/workflows/ci.yml` : fmt / clippy `-D warnings` / tests / ledger (`cargo-binstall rustledger`).
- [x] `README.md` initial (état jalons, installation, structure, conventions).
- [x] Commit jalon 1 : `30cc0ab`.

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
- [x] `TODO.md` initialisé (commit `cfed560`).
- [x] **Jalon 1 — squelette workspace + ledger d'exemple+ outillage (2026‑10‑29, `30cc0ab`)** :
  `rledger check` ✓, `just fmt-check/clippy -D warnings/test` ✓ (workspace complet, eframe 0.37
  compilé), 26 fichiers, licence GPL‑3.0, CI, hook pre-commit.

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
7. **Include des fichiers mensuels** — *décision tranchée au jalon 1* : `main.beancount` inclut des
   **globs par année** (`transactions/2025/*.beancount`, `transactions/2026/*.beancount`).
   Le loader rustledger (vérifié en test natif : ordre déterministe garanti, no‑match toléré)
   garantit qu'aucun outil n'a besoin d'éditer `main.beancount` ; un fichier mensuel nouveau est
   créé + référencé implicitement. Budgets inclus explicitement par année (visibilité des erreurs).
8. **rustledger évolue vite** (0.24.0, licence commerciale en parallèle) : dépendances épinglées
   `=0.24.0` ; un `cargo update` ne se fait qu'après lecture du CHANGELOG, avec tests golden en garde‑fou.
9. **Sémantique beancount v3 des assertions** — *découverte au jalon 1* : un `balance` à la date D
   est vérifié à **00:00 de D** (strictement avant toute transaction de D) ; en plus, la directive
   `commodity` exige une date. Convention adoptée : **assertions mensuelles datées le 1er du mois
   suivant** (le writer du jalon 5 devra la respecter).
10. **eframe 0.37 a changé le trait `App`** — *découverte au jalon 1* : `update(ctx, frame)` →
   `logic(ctx, frame)` + `ui(&mut self, ui: &mut egui::Ui, frame)` (un `Ui` monté est remis ;
   plus de `CentralPanel` obligatoire fondé sur `ctx`). Squelette adapté ; l'architecture du jalon 4
   s'appuiera sur `logic` pour les commandes et `ui` pour les vues.
11. **chmod local refusé par la sandbox d'exécution** — le bit exécutable du hook est correct dans
   git (mode 100755, mode conservé partout) mais pas sur la copie de travail de l'environnement
   d'agent ; `just setup` (chmod +x + hooksPath) le règle à la première exécution humaine. Sans effet
   ni sur CI ni sur le hook en tant que contenu versionné.
12. **Cache binaire rledger** (`ledger/.main.beancount.cache`) : ignoré (`*.cache`), le CLI rledger
   invalide l'entrée à la modification des fichiers source.

## Idées / hors périmètre v1

- Import CSV/OFX bancaire via `rledger extract` (+ `importers.toml`) et son import dans la GUI.
- Multi‑devises / investissement (déjà couverts par le moteur, hors besoin du foyer).
- Comptes individuels (si le contexte change) ; comptes titres/PEA.
- Rapports d'amortissement/retours (`rustledger-returns`, XIRR) et capital gains.
- Cohabitation avec Fava (les budgets sont déjà dans sa convention — argument de compatibilité).
- Rôles/conjoint : signature des commits, second fichier de préférences.
- Export PDF des rapports ; widgets système ; notifications push du dépassement de plafond.
