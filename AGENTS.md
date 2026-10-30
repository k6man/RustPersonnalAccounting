# AGENTS.md — instructions pour agents (OpenCode, etc.)

Point d'entrée « reprise à froid » : **`TODO.md`** (état des jalons, décisions, écarts d'API
trouvés en chemin). Décisions d'architecture : `docs/ADR-00*.md`. **Lisez TODO.md avant
toute modification** — l'état courant et la prochaine action y sont toujours à jour.

## Commandes de validation (toutes obligatoires avant chaque commit)

```sh
just check          # fmt-check + clippy -D warnings + cargo test --workspace + rledger check
just run            # GUI (cargo run -p household-gui)
cargo test -p household-gui --test budget_gui grille_2026   # un seul test
cargo run -q -p household-core --bin household-cli -- check ledger/main.beancount  # repli si rledger absent
```

- Clippy et tests doivent être verts avec `-D warnings` ; `unsafe_code = "forbid"` (lints workspace).
- **`just setup` à faire après un clone** : active `githooks/` (pre-commit valide le ledger).
- CI (`.github/workflows/ci.yml`) rejoue fmt, clippy, tests et `rledger check` sur main/develop.

## Le ledger est la source de vérité — ne l'éditez pas arbitrairement

- `ledger/main.beancount` (options + includes globs) : **jamais édité par les outils**, sauf
  l'ajout d'un include budget. Une directive par ligne dans `ledger/budgets/AAAA.beancount`.
- Toute écriture passe par les writers de `household-core` / `household-budget` :
  copie scratch + pré-validation + écriture atomique (tmp + rename) + blocs canoniques
  déterministes (diffs git propres). Ne réinventez pas ce chemin.
- Les tests ne touchent **jamais au ledger réel** : chaque suite d'intégration recopie le
  ledger dans un scratch temporaire via `tests/common/mod.rs` (suffixed par un compteur
  atomique — la résolution de l'horloge macOS ne suffit pas pour des tests parallèles).
- `ledger/transactions/AAAA/MM.beancount` : un fichier par mois ; assertions de contrôle
  au 2026-04-01 (CC = 6877.39) dans le ledger d'exemple.

## Conventions non négociables

- **Montants** : `rust_decimal::Decimal` de bout en bout — jamais `f64` pour une somme,
  `Decimal::try_from(f64)` à la frontière d'un tracé egui si nécessaire.
- **Dates** : `rustledger_core::NaiveDate` (jiff) — pas de chrono.
- **Langue** : code, commentaires, UI, comptes en français, sans accents dans les
  identifiants (`Expenses:Alimentation:Courses`).
- **rustledger épinglé exactement `=0.24.0`** : l'API publiée diverge du dépôt `main`
  (ex. `BookingEngine::apply`, `Open.meta` = `Metadata` FxHashMap, `Commit::summary()`
  → `Result<Option>` côté git2). En cas de doute, lisez les sources dans
  `~/.cargo/registry/src/…/rustledger-*-0.24.0/`, pas la doc du dépôt. Écarts
  documentés dans TODO.md ; ne `cargo update` pas sans lecture du CHANGELOG.

## Architecture (ce que les noms de fichiers ne disent pas)

- `crates/household-core` : wrapper rustledger — `Snapshot::load` (balances/validation)
  + writer transactionnel (`append_transaction`, `replace_transaction`, `remove_transaction`,
  `monthly_file`) + binaire `household-cli` (hook pre-commit).
- `crates/household-budget` : logique budgétaire **pure** (`compare`/`rollover`/
  `duplicate_year`/`writer::set_cell`, `write_year`). Tout testé sans GUI.
- `crates/household-git` : git2 (libgit2, pas de binaire git). **Un commit via libgit2
  n'exécute pas le hook pre-commit** → la validation du ledger est reproduite côté app
  (ADR-002). Les chemins du statut sont relatifs à la **racine du dépôt**.
- `crates/household-gui` (binaire `compta-foyer`) : le worker (`worker.rs`) fait **tout**
  le chargement/l'écriture hors UI thread via `Command`/`LoadOutcome` (mpsc) ; les vues
  (`views/`) sont de la présentation pure — calculs dans `viewmodel.rs` (UiData en `Arc`)
  ou dans les modules purs des autres crates. Pas de logique métier dans les vues.
  Watcher `notify` → Reload (débounce 250 ms).
- Préférences GUI (rollover par famille) : `~/.config/compta-foyer.toml`, **jamais dans le ledger**.

## Git

- Branche de travail : `develop` (le jalon courant est committé là avec TODO.md à jour
  dans le même commit). Tag `v0.1.0` existe ; push non encore effectué.
- Message type pour le ledger : `ledger: AAAA-MM — saisie/édition (N fichiers)`
  (`household_git::suggere_message`). Commits de jalon : `Jalon N — <titre>`.

## Licence

GPL-3.0-only (dérivé au sens linking de rustledger, GPL-3.0) — ADR-001. Ne remplacez pas
la licence et n'ajoutez pas de dépendance GPL-incompatible.
