# rustledger 0.24.0 — notes techniques

Référence issue de la reconnaissance du 2026-10-29 et des écarts découverts pendant les jalons 2 à 7
(extraite de l'ancien `TODO.md`). **Source de vérité : les sources publiées**
(`~/.cargo/registry/src/…/rustledger-*-0.24.0/`), pas la documentation du dépôt `main`. À revoir si
l'épinglage `=0.24.0` est un jour déplacé (voir ADR-001) ; ne pas faire de `cargo update` sans lecture
du CHANGELOG, avec les tests golden en garde-fou.

## Capacités vérifiées (0.24.0, publié le 2026-09-06)

- Crates : `rustledger-{core,parser,loader,booking,validate,query,plugin,budget,returns,ops,importer,completion,lsp,wasm}`
  et le CLI `rledger` (`check`, `query`, `format`, `report budget`, `extract`…).
- `custom` : `Directive::Custom(Custom { date, custom_type, values: Vec<MetaValue>, meta })` ; la
  sérialisation est préservée (`format_custom_lines`, `format_directives`).
- `rustledger-budget` : modèle Fava complet (`read_budget`, `Budgets::from_directives`, `accrue`,
  `compare(directives, types, from, to, children, filter) -> Comparison`) ; accrual par jour au
  dénominateur calendaire exact, supersession par `(account, currency)`, ancrage calendaire, agrégation
  hiérarchique (`covers`), non-rétroactivité. **Le rollover n'est pas fourni** : il est calculé dans
  `household-budget`.
- Loader : `load(path, &LoadOptions) -> Ledger` = includes + plugins (33 natifs) + booking (7 méthodes)
  + validation ; `Ledger { directives: Vec<Spanned<Directive>>, options, errors, source_map,
  display_context, … }`, `balance_view()` pour la vue comptable des pads, `canonical_sort_key`.
- Types : `Decimal = rust_decimal`, `NaiveDate = jiff::civil::Date` (chrono écarté).
- BQL : `rustledger-query` est disponible, mais les rapports maison itèrent `Ledger.directives`.
- MSRV : stable récente (dépôt épinglé sur 1.98.1). Licence : GPL-3.0-only (option commerciale en parallèle).

## Écarts d'API : 0.24.0 publiée vs dépôt `main`

- `BookingEngine` n'a **pas** `for_ledger` ni `into_presented_inventories` : utiliser `with_method` +
  `register_account_methods(view.iter())` + `apply(txn)` + `into_inventories()`. La présentation et
  l'arrondi relèvent de l'affichage (0.24 ne les expose pas).
- `Ledger.options.booking_method` est la **chaîne d'option** beancount (« STRICT »…) : passer par
  `BookingMethod::from_str` (`std::str::FromStr`), repli sur Strict.
- `MetaMap` (struct interne du dépôt `main`) n'est pas ré-exporté : `open.meta: Metadata` =
  `FxHashMap<String, MetaValue>` ; lecture via `meta.get("ceiling")`.
- `Spanned` se prend dans `rustledger_core` (ré-export, même type que celui du loader, dont le
  ré-export est privé).
- `ErrorLocation { file, line, column }` (`column`, pas `col`) ; `LedgerError` n'a **pas** de champ `hint` ;
  `errors: Vec<LedgerError>`.
- `rust_decimal_macros` : pas de 1.43 publiée, **1.40** épinglée (dev-dependency).
- git2 0.21 : `StatusEntry::path()` renvoie un `Result` ; `Commit::summary()` renvoie
  `Result<Option<&str>>` (`ok().flatten()`) ; `DiffOptions::include_untracked(true)` est indispensable pour
  que `resume_diff` voie les nouveaux fichiers non suivis.

## Pièges de comportement

- **Chemins de la `SourceMap`** : tels qu'inclus (relatifs possibles) **ou canoniques** (macOS : `/var`
  stocké en `/private/var`). Toute comparaison de chemins se fait après canonisation des deux côtés
  (`Snapshot::resolve_source`, `canonize` dans le writer), sinon `strip_prefix` échoue ou un même fichier
  passe pour deux fichiers distincts.
- **Span d'une directive** : elle s'étend du début de la directive au début de la **suivante** (parité
  beancount) et avale lignes vides et commentaires de queue. Toute édition chirurgicale recale le bloc
  logique avec `household_core::writer::split_directive_block` (première ligne + lignes indentées non
  commentaires).
- **Équilibre** : beancount équilibre **par groupe de devise** ; une transaction −100 EUR / +90 USD sans
  annotation de prix n'est pas équilibrée. La validation de forme du brouillon suit cette sémantique.
- **Assertions `balance`** : vérifiées à 00:00 de leur date (voir ADR-005).
- `Bucket::label` produit un libellé de ligne de total (« TOTAL », « TOTAL (Assets) ») ; pour un nom de
  catégorie, utiliser `AccountTypeKind::as_str()` (vocabulaire filaire).
- **Budget mensuel Fava** : en force tous les mois suivants jusqu'à supersession (décembre est budgété
  même sans écriture) ; le réel ne compte qu'à partir du début d'effet.
- `compare` n'invente pas de ligne sans directive : les budgets du ledger sont posés sur les **feuilles**.
  L'arbre des préfixes de l'éditeur est synthétisé ; l'agrégat est la somme des lignes **de tête**
  (jamais re-sommée sous un ancêtre déjà budgété).
- `Transaction.tags` / `links` sont des `Vec<Tag>` / `Vec<Link>` (newtypes) ; le formateur canonique les
  rend dans l'en-tête (`#tag ^lien`), donc l'aller-retour saisie → fichier les préserve.
- `Close { date, account }` joint aux `Open` donne les bornes d'existence par compte (base de
  l'autocomplétion « comptes ouverts à la date »).
- `Repository::discover` marche depuis n'importe quel chemin englobé (fichier du ledger compris). Un commit
  via libgit2 n'exécute **pas** le hook pre-commit (voir ADR-002). Les chemins du statut sont relatifs à la
  racine du dépôt.

## Outillage

- `rledger` exige un `default.file` : `.rledger.toml` à la racine pointe sur le ledger d'exemple pour que
  `rledger report budget` marche sans argument.
- Le cache binaire de rledger (`.main.beancount.cache`) est ignoré (`*.cache`) ; le CLI l'invalide dès qu'un
  fichier source change.
- Les tests parallèles d'un même processus partagent l'horloge murale (résolution de l'ordre de la
  microseconde sous macOS) : les scratch de test sont suffixés d'un compteur atomique
  (`tests/common/mod.rs`).
- Le view-model du tableau de bord est recoupé avec `rledger report budget` (le cumul `balances` n'est
  **pas** le réel du mois).
