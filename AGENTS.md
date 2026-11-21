# AGENTS.md — instructions pour agents (OpenCode, etc.)

Point d'entrée « reprise à froid » : **ce fichier**, puis `openspec/project.md` (contexte, carte des
capabilities) et `openspec list` (changes ouverts et avancement de leurs `tasks.md`). Décisions
d'architecture : `docs/ADR-00*.md` ; notes techniques sur rustledger : `docs/rustledger-0.24.md`.
Il n'y a plus de `TODO.md` : son dernier état reste dans l'historique git
(`git log --diff-filter=D -- TODO.md`, puis `git show <hash>^:TODO.md`).
Comportements attendus du logiciel : **`openspec/`** (specs par parcours utilisateur et
changes en cours) — voir la section « OpenSpec » plus bas. Ordre de reprise à froid :
`openspec list` (changes ouverts), puis la spec de la capability concernée.

## Commandes de validation (toutes obligatoires avant chaque commit)

```sh
just check          # fmt-check + clippy -D warnings + cargo test --workspace + rledger check
just run            # TUI (cargo run -p household-gui)
cargo test -p household-gui --test gui_tui                # tous les tests TUI
cargo test -p household-gui --test budget_gui grille_2026 # un seul test
cargo run -q -p household-core --bin household-cli -- check examples/ledger/main.beancount  # repli si rledger absent
openspec validate --all --strict   # specs et changes valides (contrôle de forme uniquement)
```

- Clippy et tests doivent être verts avec `-D warnings` ; `unsafe_code = "forbid"` (lints workspace).
- **`just setup` à faire après un clone** : active `githooks/` (pre-commit valide le ledger).
- CI (`.github/workflows/ci.yml`) rejoue fmt, clippy, tests et `rledger check` sur main/develop.

## OpenSpec — comportements attendus et changes

L'état d'avancement vit dans les `tasks.md` cochés de `openspec/changes/<id>/` (il remplace l'ancien
`TODO.md`) ; `openspec/specs/` dit **ce que le logiciel doit faire** ; l'historique est dans `git log` ;
les décisions d'architecture sont dans `docs/`. **Ne recréez pas de fichier de suivi parallèle.**

- `openspec/project.md` : contexte, conventions, carte parcours → capability → crate.
- `openspec/specs/<capability>/spec.md` : 10 capabilities **par parcours utilisateur** — ouvrir-ledger,
  naviguer-application, consulter-tableau-de-bord, parcourir-journal, saisir-transaction,
  garantir-integrite-ecritures, piloter-budget, analyser-annee, valider-et-versionner,
  regler-application. Reconstruites à partir du code (backfill) : elles décrivent le comportement
  **actuel**, pas forcément l'intention d'origine.
- `openspec/changes/<id>/` : évolutions proposées (`proposal.md`, `tasks.md`, deltas de specs).
- `openspec/GAPS.md` : écarts connus entre specs, code et documentation — à traiter, pas à ignorer.
- Si un bloc géré par OpenSpec (balises `<!-- OPENSPEC:START -->` … `<!-- OPENSPEC:END -->`) est présent
  dans ce fichier, il est réécrit par `openspec update` : ne l'éditez pas, ajoutez les règles en dehors.

### Quand un change est obligatoire (avant d'écrire du code)

- Nouvelle capacité ou nouveau comportement visible de l'utilisateur.
- Modification d'un comportement existant, y compris une règle de validation ou d'écriture.
- Changement du format du ledger écrit, des fichiers de budget ou du TOML de préférences.
- Tout ce qui touche `garantir-integrite-ecritures`.

Pas de change : correction d'un bug qui rétablit un comportement **déjà spécifié** (ajoutez le test de
non-régression), refactor sans effet observable, typos / commentaires / docs, tests seuls, mise à jour
de dépendance sans effet de comportement. **En cas de doute, ouvrez un change.**

### Workflow

1. `openspec list` et `openspec list --specs` ; `openspec show <capability>` pour une spec.
2. Proposer : créer `openspec/changes/<id>/` (`<id>` en kebab-case, descriptif) avec `proposal.md`,
   `tasks.md` et les deltas de specs. **S'arrêter et demander la relecture humaine : ne pas coder
   tant que le change n'est pas approuvé.**
3. Implémenter les tâches de `tasks.md` dans l'ordre, les cocher au fur et à mesure, `just check` à chaque étape.
4. Archiver : `openspec archive <id>` (fusionne les deltas dans `specs/`), puis
   `openspec validate --all --strict`.

### Format des specs (le validateur est strict)

- `### Requirement: <nom>` suivi d'une phrase contenant **SHALL** ou **MUST** ; au moins un
  `#### Scenario: <nom>` par requirement, écrit en `- **WHEN** …` / `- **THEN** …` (`AND` possible).
- Mots-clés structurels en anglais (Requirement, Scenario, WHEN, THEN, SHALL, MUST, ADDED, MODIFIED,
  REMOVED) ; le contenu est en français, comme le reste du dépôt.
- Dans un delta, `## MODIFIED Requirements` recopie le requirement **complet**, avec **tous** ses
  scénarios : un scénario omis disparaît à l'archive.
- Un requirement = un comportement vérifiable. Pas de détail d'implémentation (noms de fonctions, de
  crates) : il va dans `design.md` ou dans `openspec/project.md`.

### Cohérence specs ↔ code ↔ tests

- Ne modifiez pas `openspec/specs/` à la main pour « faire coller » : un écart est soit un bug du code,
  soit une spec fausse. Dans les deux cas, ouvrez un change (le delta corrige la spec ou la tâche
  corrige le code).
- Chaque `#### Scenario` doit avoir un test. Convention : un commentaire
  `// spec: <capability>/<Requirement>/<Scenario>` au-dessus du test. Un scénario sans test est une
  dette à lister dans le change.
- `garantir-integrite-ecritures` est transversale : tout nouveau chemin d'écriture (import, nouvelle
  vue) passe par les writers existants et respecte ses scénarios.
- Les conventions non négociables ci-dessous (Decimal, dates, langue, rustledger épinglé) s'appliquent
  aussi aux changes et à leur code.

## Le ledger d'exemple vs le vrai ledger du foyer

- **`examples/ledger/`** (données **fictives**) est le ledger de **recette** du
  logiciel : validé par `rledger check` en CI, recopié en scratch par les tests,
  jamais édité à la main hors writer. Ce n'est **pas** la comptabilité réelle —
  uniquement des données fictives.
- **Le vrai ledger du foyer vit hors du dépôt** : dépôt git **séparé**
  (ADR-004), `main.beancount` à sa racine. Il est désigné par la chaîne de
  résolution `household-core::resolution` : argument CLI > `RPA_LEDGER` >
  champ `chemin_ledger` de `~/.config/RustPersonnalAccounting/config.toml` > repli
  sur l'exemple avec badge. Après création de ce dépôt :
  `just setup-ledger <dossier-du-ledger>` attache une fois pour toutes le
  hook `pre-commit` commun (le hook auto-détecte le point d'entrée dans le
  dépôt où on committe).
- `examples/ledger/main.beancount` (options + includes globs) : **jamais édité par les outils**, sauf
  l'ajout d'un include budget. Une directive par ligne dans `examples/ledger/budgets/AAAA.beancount`.
- Toute écriture passe par les writers de `household-core` / `household-budget` :
  copie scratch + pré-validation + écriture atomique (tmp + rename) + blocs canoniques
  déterministes (diffs git propres). Ne réinventez pas ce chemin.
- Les tests ne touchent **jamais à l'exemple du dépôt** : chaque suite d'intégration recopie le
  ledger dans un scratch temporaire via `tests/common/mod.rs` (suffixed par un compteur
  atomique — la résolution de l'horloge macOS ne suffit pas pour des tests parallèles).
- `examples/ledger/transactions/AAAA/MM.beancount` : un fichier par mois ; assertions de contrôle
  au 2026-04-01 (CC = 6877.39) dans le ledger d'exemple.

## Conventions non négociables

- **Montants** : `rust_decimal::Decimal` de bout en bout — jamais `f64` pour une somme,
  `Decimal::try_from(f64)` à la frontière d'un tracé si nécessaire.
- **Dates** : `rustledger_core::NaiveDate` (jiff) — pas de chrono.
- **Langue** : code, commentaires, UI, comptes en français, sans accents dans les
  identifiants (`Expenses:Alimentation:Courses`).
- **rustledger épinglé exactement `=0.24.0`** : l'API publiée diverge du dépôt `main`
  (ex. `BookingEngine::apply`, `Open.meta` = `Metadata` FxHashMap, `Commit::summary()`
  → `Result<Option>` côté git2). En cas de doute, lisez les sources dans
  `~/.cargo/registry/src/…/rustledger-*-0.24.0/`, pas la doc du dépôt. Écarts
  documentés dans `docs/rustledger-0.24.md` ; ne `cargo update` pas sans lecture du CHANGELOG.

## Architecture (ce que les noms de fichiers ne disent pas)

- `crates/household-core` : wrapper rustledger — `Snapshot::load` (balances/validation)
  + writer transactionnel (`append_transaction`, `replace_transaction`, `remove_transaction`,
  `monthly_file`) + binaire `household-cli` (hook pre-commit).
- `crates/household-budget` : logique budgétaire **pure** (`compare`/`rollover`/
  `duplicate_year`/`writer::set_cell`, `write_year`). Tout testé sans GUI.
- `crates/household-git` : git2 (libgit2, pas de binaire git). **Un commit via libgit2
  n'exécute pas le hook pre-commit** → la validation du ledger est reproduite côté app
  (ADR-002). Les chemins du statut sont relatifs à la **racine du dépôt**.
- `crates/household-gui` (binaire `compta-foyer`) : **TUI ratatui plein écran**
  (une seule « fenêtre » terminal, pas d'egui). Le worker (`worker.rs`) fait
  **tout** le chargement/l'écriture dans un thread dédié via
  `Command`/`LoadOutcome` (mpsc) ; la boucle de `App::run` ne fait que
  rendu + événements. Les vues (`views/`) sont de la présentation pure —
  calculs dans `viewmodel.rs` (UiData en `Arc`), dans les modules purs
  (`saisie.rs`) ou dans les autres crates. Pas de logique métier dans les
  vues. Watcher `notify` → Reload (débounce 250 ms).
- **Écran unique à onglets** : `ecran_actif` dans `app.rs` (index dans
  `Onglet::TOUS`), Ctrl+1..7 **ou F1..F7** dans la barre du haut ; **Esc = retour au
  Tableau de bord, jamais quitter** (`q`/Ctrl+C quittent) ; flash en bas
  d'écran (succès 4 s, erreur persistante) ; modale de confirmation de
  suppression. La souris passe par les **zones cliquables enregistrées au
  rendu** (`zones_cliquables: Vec<(Rect, ActionClique)>`) — jamais de
  coordonnées magiques. `COMPTA_ECRAN=<ecran>` ouvre seulement l'écran
  demandé (affordance de test/captures).
- Préférences app (chemin du vrai ledger, rollover par famille) :
  `~/.config/RustPersonnalAccounting/config.toml`, **jamais dans le ledger**.

## Pièges ratatui 0.26 / crossterm 0.27

- **`KeyEventKind::Press` filtré** dans `App::run` : sans filtre, les
  terminaux/OS qui émettent Press+Release traitent chaque touche deux fois.
- **Routing clavier par priorité** : modale de confirmation > Ctrl+1..7 /
  F1..F7 > écran actif > commandes globales (q/r/l/Esc). Les écrans à champ texte
  (Saisie, Git) et les éditeurs (posting, cellule budgétaire) consomment
  **toutes** les touches — les lettres r/l/e/x y sont du texte.
- **Colonnes de `Table`** : le `column_spacing` est soustrait **avant** la
  répartition (vérifié dans les sources épinglées 0.26.3,
  `widgets/table/table.rs`) ; pour une grille large →
  `Constraint::Percentage` (jamais des `Length` figés : ≈161 colonnes
  mettaient Total/Écart hors écran dès 120) + cellules compactes — le clip
  droit perd d'abord les décimales, dégradation gracieuse.
- **Focus des champs** : `champ_actif` est la seule source de vérité,
  `sync_focus` appelé au rendu — les TextInput ne conservent pas le focus
  entre deux dessins.
- **Tests de rendu** : `TestBackend` + lecture du buffer
  (`backend.buffer().content()`), helper `render_safe` dans
  `tests/gui_tui.rs`. Le fixture doit contenir des lignes réelles (une
  grille vide rend « Aucune ligne… » et le test passe à côté de son objet)
  ; en cas d'échec, afficher le buffer dans le message (`{texte:?}`).
- ratatui n'est pas seul épinglé : **ratatui « 0.26 »** (0.26.3 résolu),
  crossterm 0.27 — lisez les sources dans
  `~/.cargo/registry/src/…/ratatui-0.26.3/` plutôt que la doc du dépôt.

## Git

- Branche de travail : `develop` (le travail courant est committé là, specs et
  `tasks.md` à jour dans le même commit). Tag `v0.1.0` existe ; push non encore effectué.
- Un change : proposal, tasks et deltas de specs committés **avant** le code ; `openspec archive`
  dans le dernier commit du change, avec ses `tasks.md` cochés. Message : `Change <id> — <titre>`.
- Message type pour le ledger : `ledger: AAAA-MM — saisie/édition (N fichiers)`
  (`household_git::suggere_message`).

## Licence

GPL-3.0-only (dérivé au sens linking de rustledger, GPL-3.0) — ADR-001. Ne remplacez pas
la licence et n'ajoutez pas de dépendance GPL-incompatible.
