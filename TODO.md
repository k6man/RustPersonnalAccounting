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

**État du projet** : jalons 1→6 terminés (jalon 6 : éditeur de budget). Ledger d'exemple
validé (`rledger check` ✓), fmt/clippy `-D warnings`/tests verts (69 tests).
**Prochaine action : jalon 7** (rapports, git, polish).

---

## En cours

_Néant — voir « À faire » (jalon 7)._

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

- [x] `household-core` : wrapper du chargement — `Snapshot` immuable construit sur
  `rustledger_loader::load(path, &LoadOptions)` ; exposé : directives sourcées (`Spanned<Directive>`
  + `SourceMap` → fichier/ligne via `source_of`, spans exacts via `span_of`), balances par compte/
  devise (rejeu `BookingEngine` sur `balance_view()`, soldes au `Through::{StartOfDay,EndOfDay}`),
  erreurs `LedgerError` typées (`is_valid`/`errors`), comptes ouverts + payees indexés,
  plafonds lus dans les métadonnées `open` (`ceiling:`, `MetaValue::Amount|Number`).
- [x] Types domaine : `NaiveDate = rustledger_core::NaiveDate` (jiff), `Decimal = rust_decimal` —
  ré-exports, pas de chrono, pas de conversions aux frontières.
- [x] API requêtes pour la GUI (partie requêtes du jalon) : `balances_at/EndOfDay/balance_of/
  balances_at_end`, `last_transaction_date`, `transactions()` (index), `transaction(index)`,
  `payees()` (tri fréquence puis alpha), `accounts()`/`ceilings()` — les filtres riches
  (substring/tag) seront posés avec l'écran Journal (jalon 4/5, selon besoin réel).
- [x] **Writer transactions** (`src/writer.rs`) : bloc canonique via `rustledger_core::format`
  (`FormatConfig::default()`) ; append trié par date (`find_insertion_byte` sur lignes ISO
  reconnues) dans `transactions/AAAA/MM.beancount` (créé si absent, en‑tête documentaire) ;
  replace sur place par span exact (même date) sinon remove+append ; suppression = coupe du
  bloc + pli des lignes vides ; écriture atomique (temp + `rename`, `sync_all`) ; verrou
  `.write.lock` (create_new, anti‑stale 120 s) ; ⚠️ **garantie pro** : toute mutation est
  d'abord jouée sur une **copie scratch du ledger** (`precheck_with`) et refusée si le chargement
  n'est pas sans erreur, puis le ledger réel est revalidé après écriture.
- [x] `household-cli` (bin du crate household-core) : `check` (exit 0/1/2, erreurs
  fichier:ligne) et `balances` (`--date=AAAA-MM-JJ`, format colonne scriptable) — branché en
  repli dans `githooks/pre-commit` et `just check-ledger` (`cargo run -p household-core --bin
  household-cli`). NB le repli reste vérifié à la main (rledger présent localement).
- [x] Tests d'intégration : `tests/golden.rs` (lecture du ledger réel : validité, soldes de
  contrôle 2026‑04‑01, plafonds 22 950/12 000, dernier jour, payees) et `tests/writer.rs`
  (copie scratch : append→replace→remove bout en bout avec soldes attendus, ordre
  chronologique inter‑insertions, refus d'une transaction déséquilibrée **fichier intact**,
  refus d'une écriture qui casserait une assertion `balance`). 9 tests au vert.
- [x] Commit jalon 2.

**Écarts d'API 0.24.0 publiée vs dépôt `main` découverts au jalon 2** (source de vérité :
crates ~/.cargo/registry/…-0.24.0 ; à revoir si épinglage un jour déplacé) :
- `BookingEngine` **n'a pas** `for_ledger` ni `into_presented_inventories` ; utiliser
  `with_method` + `register_account_methods(view.iter())` + `apply(txn)` + `into_inventories()`;
  la présentation/arrondi est une préoccupation d'affichage (0.24 ne l'expose pas).
- `Ledger.options.booking_method` est la **chaîne d'option** beancount ("STRICT"…) →
  `BookingMethod::from_str` (via `std::str::FromStr`), repli Strict.
- `MetaMap` (struct interne du main) n'est pas re‑exporté : `open.meta: Metadata`
  = `FxHashMap<String, MetaValue>` ; lecture via `meta.get("ceiling")`.
- `Spanned` se prend dans `rustledger_core` (re-export, même type que celui du loader, dont le
  re-export est privé).
- `ErrorLocation { file, line, column }` (`column`, pas `col`) ; `LedgerError` **n'a pas** le
  champ `hint` en 0.24.0 ; `errors: Vec<LedgerError>`.
- Chemins de la `SourceMap` : tels qu'inclus (relatifs possibles) **ou canoniques**
  (macOS : `/var` → stocké `/private/var`) — `Snapshot::resolve_source` + canonisation
  (`canonize`) du cible/parent dans le writer, sinon `strip_prefix` échoue.
- `rust_decimal_macros` : pas de 1.43 publiée → **1.40** épinglée (dev‑dep).

### Jalon 3 — household-budget : logique budgétaire pure

- [x] Modèle : `BudgetGrid::build(directives, types, year)` — grille postes × 12 mois construite
  sur `rustledger_budget::Budgets::from_directives` + un appel `compare(directives, types,
  [1er mois, 1er suivant), children=true)` **par mois** (accrual Fava : pro‑rata par jour,
  ancrage calendaire, supersession).
- [x] Lignes : `GridCell { budgeted, actual }` + `remaining()`/`used_fraction()` ; `GridRow`
  (totaux année) ; totaux par colonne via `Comparison.totals` (chaque budget/poste compté une
  fois — jamais la somme des lignes, qui double‑compte les enfants) ; diagnostics
  `BudgetError` dédupliqués sur l'année dans `warnings` (libellés de catégorie = vocabulaire
  filaire `as_str()` — `Bucket::label` produit un libellé de ligne TOTAL, pas un nom de type).
- [x] **Rollover** (`rollover.rs`) : `RolloverSeries::from_cells(cells, initial)` —
  `disponible(m) = report_initial + Σ budget − Σ réel`, report **négatif autorisé** (choix
  « cumul », le dérapage reste visible), `carry_out()` = report sortant. Sémantique verrouillée
  par tests.
- [x] Remise à zéro : les directives `0.00 EUR` restent des cellules budgétées à 0 (la grille
  les écrit, ne les supprime pas) — testé via le ledger réel (Vacances).
- [x] **Duplication d'année** (`duplicate.rs`) : N→N+1 avec pour cent (±), arrondi **au multiple
  supérieur** d'un pas paramétrable (`0.01` centimes, `1.00` euros, `5.00`…), remise à l'échelle
  monétaire 2 décimales ; sortie = directives canoniques triées (date, compte) ; refus : pas ≤ 0,
  année source vide, **année cible déjà budgétée** ; 0 % = idempotent (testé).
- [x] Writer budget chirurgical (`writer.rs`) : `set_cell` remplace **le seul token montant** de
  la ligne (octets ailleurs préservés — alignement compris ; round‑trip à valeur égale =
  fichier identique, testé) ; directive absente → insertion triée (date, compte) ; nouvelle
  année → création `budgets/AAAA.beancount` + include dans `main` **en une seule mutation
  multi‑fichiers** (`household_core::writer::edit_ledger_files`) ; `write_year` refuse
  d'écraser un fichier existant. Garanties héritées : verrou, pré‑vérification scratch,
  atomicité, revalidation.
- [x] Avertissements : `Comparison.errors` (`BudgetError { date, account, reason }`) remontés
  dédupliqués dans `BudgetGrid.warnings`.
- [x] Tests budget (17 verts) : février bissextile (daily 28 vs 29), supersession en cours de
  mois (split pro‑rata avril 10/20 jours = 230.00 exact), directive datée en milieu de mois
  (200.00), réel/fraction, parent+enfants (addition, pas absorption), rollover cumulé ±,
  duplication 0 % idempotente / +6 % pas 1 € et pas centime / refus d'écrasement, bornes de
  fenêtres ; writer : round‑trip identique, une‑seule‑ligne‑changée + grille suit, insertion
  triée avec supersession, génération 2027 (fichier + include + grille), refus d'écrasement.
- [x] Commit jalon 3.

**Découvertes du jalon 3** (source de vérité : crates 0.24.0 publiés) :
- **Span parseur** : la span d'une directive s'étend du début de la directive **au début de la
  suivante** (parité beancount) — elle avale lignes vides et commentaires de queue. Toute
  édition chirurgicale doit recaler le **bloc logique** : `household_core::writer::
  split_directive_block` (1ʳᵉ ligne + lignes indentées non‑commentaires).
- **Bug corrigé au passage (régression jalon 2)** : `rewrite_span` perdait le texte **avant** le
  bloc (introduit en supprimant un warning clippy) ; les tests de l'époque ne l'avaient pas
  attrapé (fichiers mono‑bloc). Ajout du test `suppression_preserve_commentaires_et_autres_blocs`.
- `Bucket::label` = libellé de ligne de total (« TOTAL », « TOTAL (Assets) ») ; pour un nom de
  catégorie, utiliser `AccountTypeKind::as_str()` (vocabulaire filaire).
- Budget mensuel Fava : en force **tous les mois suivants** jusqu'à supersession (décembre
  budgété même sans écriture) ; le réel ne compte qu'à partir du début d'effet.
- Les tests parallèles d'un même processus partagent l'horloge murale (résolution ~µs macOS) :
  répertoires scratch suffixés par un compteur atomique, sinon collisions/nettoyages croisés.

### Jalon 4 — GUI lecture seule (dashboard + journal)

- [x] Squelette `household-gui` eframe : crate **lib + bin** (`compta-foyer`, le lib expose
  les modules aux tests d'intégration) ; `FoyerApp` (état GUI sans logique métier), enum
  `Command { Reload }`, **worker thread unique** (canaux `mpsc` requête/réponse,
  `ctx.request_repaint()` à réception), `logic()` draine les résultats hors rendu ; onglets
  Tableau de bord / Journal, libellés français.
- [x] Rechargement au démarrage + **watcher `notify`** (8.2 : `recommended_watcher` + trait
  `Watcher` en import) sur le répertoire du ledger, récursif ; **debounce côté worker**
  (drain + 250 ms + drain) pour écrêter les rafales d'événements.
- [x] Écran **Tableau de bord** : soldes CC/LivretA/LDD, jauges de plafond (ÉtatJauge
  Ok < 90 % / Alerte ≥ 90 % / Dépassement > 100 %, cadre rouge bloquant), patrimoine net,
  carte budget vs réel du mois courant (8 postes les plus tendus, sur-budget en tête),
  courbe du patrimoine par fin de mois (`egui_plot`, X = index de mois avec libellés).
- [x] Écran **Journal** : `egui_extras::TableBuilder` (colonnes date/!/tiers/libellé/comptes/
  montant/tags), filtres texte/compte-préfixe/tag/dates/montants (parsing tolérant, dates
  invalides signalées), pagination 100/page bornée, sélection → détail (postings avec
  équilibrages, tags/liens, **origine fichier:ligne:entrée**), bouton Réinitialiser.
- [x] Bannière d'erreurs de validation (codes `LedgerError` + fichier:ligne, 5 premières + total)
  et bannière d'échec de chargement (l'état précédent reste affiché).
- [x] Commit jalon 4.

**Découvertes du jalon 4** :
- **egui 0.37** : `TopBottomPanel`/`SidePanel` unifiés en `egui::Panel` (+ `Panel::top/bottom`),
  `CentralPanel::show(ui)` prend directement le `Ui` monté (`show_inside` déprécié) ;
  `ProgressBar::new(f32)` ; pas de `Ui::colored_text` → `RichText::color()` ; `Label` sans
  `weak/strong` (passer par `WidgetText`) ; egui_plot 0.38 : `Line::new(nom, points)` +
  `Plot::x_axis_formatter(|GridMark, plage| String)` ; Decimal n'implémente **pas** `From<f64>`
  (par design) → `Decimal::try_from` dans les formateurs.
- **Patrimoine net = Σ Assets − Σ Liabilities, Equity exclu** : `Equity:Soldes-Ouverture`
  porte le capital historique et s'annulerait avec les assets d'ouverture (testé :
  36 946,87 € et non 5 780,97 €). À consigner dans l'ADR (jalon 7).
- Le view-model est **croisé avec `rledger report budget`** (Courses mars : 309,85 € identique)
  — le cumul `balances` (833,05) n'est PAS le réel du mois.
- `rledger` exige un `default.file` : ajout de `.rledger.toml` projet (`ledger/main.beancount`)
  pour que `rledger report budget` marche sans argument.
- Le crate GUI a une cible **lib** (`src/lib.rs`) + bin : les tests d'intégration ne peuvent pas
  lier un binaire pur.
- Tests parallèles : répertoires scratch suffixés d'un compteur atomique (l'horloge macOS ne
  suffit pas) — déjà rencontré au jalon 3, appliqué aussi ici.

### Jalon 5 — Saisie et écriture de transactions

- [x] Écran **Saisie** (`views/saisie.rs` + module pur `saisie.rs`) : date, flag `*`/`!`,
  tiers, libellé, tags `#` et liens `^` (préfixe facultatif, dédupliqués), lignes de postings
  dynamiques (＋/✕), montant par ligne (« 38.42 », « 10 USD », virgule acceptée), **ligne
  d'équilibrage = champ vide** (une seule), bouton « Reporter X sur une nouvelle ligne »,
  indicateur « reste à répartir » / « équilibrée ✓ » ; **autocomplétion** comptes (ouverts à
  la date, fermés exclus — `InfoCompte` joints opens+closes dans le view-model) et tiers
  (fréquence), suggestions cliquables sous le champ focalisé.
- [x] Validation en direct : **deux niveaux** — (a) validation de *forme* immédiate dans le
  module `saisie` (date, libellé, montants, équilibre PAR DEVISE — sémantique beancount v2+),
  (b) la validation **autorisée** reste la pré-vérification scratch du writer (compte fermé,
  devise interdite, assertion cassée → refus). Choix documenté dans la UI.
- [x] **Ligne d'équilibrage explicite** : la contrepartie calculée est écrite avec son montant
  (pas de posting élidé dans le fichier) — déterministe, zéro interpolation au chargement.
- [x] Écriture dans le bon fichier mensuel via le writer du jalon 2 (`Command::Ecrire`) ;
  le worker recharge un snapshot **frais** avant d'écrire, puis reconstruit le view-model
  (`LoadOutcome::Ecrit` : données fraîches + message du fichier cible, retour au journal,
  flash vert ; `LoadOutcome::Refuse` : flash rouge, ledger intact).
- [x] Édition/suppression depuis le Journal : boutons dans le détail, suppression en deux
  clics (confirmation locale) ; **relocalisation par empreinte** (date+tiers+libellé,
  `worker::localiser`) avant tout replace/remove — si le ledger a changé depuis le chargement,
  refus lisible au lieu d'éditer la mauvaise ligne.
- [x] Test bout en bout **headless** (`tests/e2e_saisie.rs`) : saisie → conversion → writer →
  snapshot → view-model (journal contient la ligne avec sa source, dashboard suit : 6877.39 →
  6852.39 → 6827.39 → 6877.39 après suppression) ; relocalisation (index erroné mais empreinte
  valide OK, empreinte absente refusée) ; refus d'écriture sur compte inexistant.
- [x] Tests pur saisie (11) : parsing montant/devise/virgule, tags/liens, erreurs de forme,
  double ligne d'équilibrage refusée, devises mélangées avec élidée refusée, équilibre par
  groupe de devise OK, pré-remplissage round-trip, complétion comptes (ouverture/fermeture)
  et payees (fréquence), empreinte. Total : 53 tests verts.
- [x] Commit jalon 5.

**Découvertes du jalon 5** :
- Beancount équilibre **par groupe de devise** (chaque devise doit sommer à zéro, sauf prix) —
  la validation de forme du brouillon suit cette sémantique ; une transaction −100 EUR /
  +90 USD sans annotation de prix n'est PAS équilibrée (casse-piège corrigé dans le test).
- `Transaction.tags`/`links` sont `Vec<Tag>`/`Vec<Link>` (newtypes) et le formateur canonique
  les rend dans le header (`#tag ^lien`) — l'aller-retour saisie→fichier préserve tout.
- `Close { date, account }` joint aux `Open` donne les bornes d'existence par compte
  (`InfoCompte`), base de l'autocomplétion « ouverts à la date ».

### Jalon 6 — Éditeur de budget

- [x] Écran **Budget** (`views/budget.rs` + module pur `budget.rs`) : grille postes × 12 mois,
  sélecteur d'année (◀/▶), **arbre hiérarchique repliable avec groupes SYNTHÉTISÉS** — le
  ledger réel ne budgete que les feuilles : un préfixe intermédiaire sans directive propre
  devient un nœud d'agrégation (somme des lignes budgétées **de tête** de sa sous-arborescence,
  chaque budget compté une fois — un intermédiaire budgété n'est jamais re-sommé).
- [x] Cellules éditable : budget cliquable (postes avec directive propre ; groupes en lecture
  seule avec agrégat affiché), **réel affiché dessous** coloré si dépassement, totaux par ligne
  (année) et par colonne (`grid.totals` — chaque budget compté une fois).
- [x] **Diff attendu avant écriture** : barre de confirmation sous la grille
  (« Habillement — mars : 75,00 € → 95,00 € ») + explication de la supersession ; puis
  `Command::CelluleBudget` → `household_budget::writer::set_cell` (token montant seul,
  round-trip identique à valeur égale).
- [x] **Duplication d'année** : bouton « Générer N+1… », dialogue (ajustement % DragValue,
  pas d'arrondi centimes/1 €/5 €) → `duplicate_year` + `write_year` (refus d'écrasement
  intact), grille N+1 disponible après reload.
- [x] **Lignes d'épargne** : note « intérêts inclus dans le réel » sur `Assets:Epargne:*`.
- [x] **Toggles rollover** (Q6 tranchée) : case à cocher sur les lignes de tête de l'arbre ;
  état persisté dans **`~/.config/compta-foyer.toml`** (jamais le ledger), tolérant (fichier
  absent/corrompu → défauts, `$HOME` absent → mémoire seule + flash) ; rollover actif =
  cellules affichent le **disponible cumulé** (report initial 0, report négatif autorisé,
  sémantique « cumul annuel » verrouillée au jalon 3) + colonne Année = report sortant.
- [x] View-model : `grilles: BTreeMap<i16, BudgetGrid>` par année budgétée +
  `directives_budget` triées (date, compte) — base du sélecteur et du diff ; la carte du
  tableau de bord dérive de la grille de l'année courante.
- [x] Tests : +16 GUI (7 pur budget : arbre/ancêtres/visibilité/têtes sans double-comptage/
  rollover/total-mois ; 3 config TOML round-trip/bascule/corrompu ; 5 budget_gui sur ledger
  réel : grille + hiérarchie synthétisée, directives triées, édition juin→supersession
  reflétée, génération 2027, rollover Vacances) — **69 tests verts** au total.
- [x] Commit jalon 6.

**Découvertes du jalon 6** :
- Les budgets du ledger réel sont posés sur les **feuilles** — `Expenses:Enfants` etc. n'ont
  PAS de ligne dans la grille (`compare` n'invente pas de ligne sans directive). L'éditeur
  synthétise donc l'arbre des préfixes : agrégat = somme des lignes **de tête** (une ligne
  dont un ancêtre est budgété est déjà couverte par son agrégat — jamais re-sommée).
- **Q6 tranchée** : préférences GUI (toggles rollover) dans `~/.config/compta-foyer.toml`,
  jamais dans le ledger — conformément à la décision pré-encadrée au jalon 1.
- egui : le même `&mut etat` ne peut pas être emprunté dans une closure de layout et
  modifié après — capturer les clics en variables locales puis appliquer hors closure.

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
- [x] **Jalon 2 — household-core : lecture et écriture du ledger (2026‑10‑29)** : `Snapshot`
  (chargement complet + requêtes) + writer transactionnel (append/replace/remove canoniques,
  scratch + pré-vérification + atomicité + verrou) + binaire `household-cli` branché hook/justfile ;
  10 tests verts (golden 4 + writer 5 + unitaire 1), clippy `-D warnings` vert, ledger revalidé
  par `rledger check`. Détails et écarts d'API 0.24.0 dans la checklist archivée du jalon (ci‑dessus).
- [x] **Jalon 3 — household-budget : logique budgétaire pure (2026‑10‑29)** : grille postes × 12
  mois (compare mensuel, children), rollover cumulé (report négatif autorisé), duplication
  d'année ±% avec arrondi au multiple supérieur paramétrable, writer chirurgical (token montant
  seul, round-trip identique, multi-fichiers pour nouvelle année + include) ; 27 tests verts
  au total (core 10 + budget 17), bug de régression corrigé
  (span parseur avale les commentaires de queue ; texte avant le bloc perdu).
- [x] **Jalon 4 — GUI lecture seule (2026‑10‑29)** : worker + watcher notify (debounce),
  tableau de bord (soldes, jauges de plafond à 3 états, patrimoine net Equity exclu, budget vs
  réel, courbe egui_plot), journal filtrable/paginé avec détail fichier:ligne, bannières
  d'erreurs ; crate lib+bin ; 12 tests GUI (6 unitaires filtres/fmt + 6 view-model croisés
  avec rledger) — total 39 tests verts. Découvertes egui 0.37 ci‑dessus.
- [x] **Jalon 5 — saisie et écriture de transactions (2026‑10‑29)** : écran Saisie (postings
  dynamiques, équilibrage par devise, contrepartie explicite, autocomplétion comptes ouverts à
  la date + tiers par fréquence, tags/liens), deux niveaux de validation (forme immédiate +
  pré-vérification scratch autoritaire), Command::Ecrire/Modifier/Supprimer dans le worker,
  relocalisation par empreinte, édition/suppression depuis le journal (double-clic de
  confirmation) ; test headless bout en bout ; 53 tests verts au total (26 GUI).
- [x] **Jalon 6 — éditeur de budget (2026‑10‑29)** : arbre hiérarchique repliable avec groupes
  synthétisés (agrégat = somme des têtes, jamais de double-comptage), cellules éditable avec
  diff de confirmation + supersession expliquée, réel sous budget, totaux ligne/colonne,
  duplication N+1 (dialogue %/pas), note intérêts sur l'épargne, rollover par famille
  persisté en TOML local (Q6 tranchée) ; 69 tests verts au total (42 GUI).

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
