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

**État du projet** : **jalons 1→7 terminés — projet complet (v0.1.0)**. Ledger d'exemple
validé (`rledger check` ✓), fmt/clippy `-D warnings`/tests verts.
Passe UI « largeurs » appliquée (2026-10-30), **puis passe de vérification visuelle**
(captures d'écran de chaque onglet, 2026-10-31) avec corrections.
**Revue de code Rust complète (2026-11-01)** : trois correctifs de fiabilité appliqués
(drain du debounce, brouillon conservé au refus, `replace_transaction` atomique) —
reste au backlog « Revue de code » (À faire).
**Fenêtre « Graphiques » (2026-11-01/02)** : 7ᵉ écran flottant implémenté et
visuellement vérifié (répartition, cascade annuelle, budget vs réel) — 80
tests verts, `just check` ✓ ; livré dans le commit suivant.
**Jalon 8 (2026-11-07)** : séparation ledger d'exemple (`examples/ledger/`,
données fictives), vrai ledger du foyer configuré hors dépôt
(`household-core::resolution` : argument > `RPA_LEDGER` > champ
`chemin_ledger` de `~/.config/rust-personal-accounting.toml`), config
renommée avec migration, badge GUI. Suite de l'audit RGPD du dépôt.
**Prochaine action : usage réel** (saisie quotidienne, budgets 2027 via l'éditeur).

---

## En cours

_Fenêtre « Graphiques » — 7ᵉ écran flottant (2026-11-01/02) — code, tests,
vérification visuelle des trois états (captures ✓) : `just check` vert
(80 tests) ; livré dans le commit suivant._

**Décisions tranchées avec la maîtresse d'ouvrage** : une seule fenêtre « Graphiques »
avec des mini-onglets internes (pas une fenêtre par graphe) ; trois graphes
retenus — répartition des dépenses (bascule année / mois, défaut année),
waterfall annuel, budget vs réel du mois. **Waterfall sans double-compte** :
l'identité comptable est Revenus − Dépenses = Δ patrimoine ; l'épargne nette
n'est **pas** additive (intérêts comptés dans les revenus, transferts
Assets→Assets s'annulés) → 3 barres d'identité + la barre « Épargne (info) »
séparée. Ligne de base du Δ patrimoine = fin N−1 via `balances_at(2025-12-31)
fin de journée` (ouverture du ledger au 2025-01-01). Golden : Δ = 36 946,87 −
32 014,71 = **4 932,16 €**. Position dans `Onglet::TOUS` : Graphiques avant
Git → Ctrl+6 = Graphiques, Ctrl+7 = Git. Pas de donut : egui_plot 0.38 n'a pas
de `Pie` (vérifié dans les sources épinglées).

**Implémentation** :
- `viewmodel.rs` : `GraphesData` (année, mois, `repartition_annee/mois` top 8 +
  « Autres (N postes) », `cascade: CascadeAnnee`), `TOP_CATEGORIES = 8` ;
  helpers purs `repartition()` (filtre zéros, tri `Reverse`, agrégat Autres),
  `cascade()` (agrégats Rapports + Δ patrimoine aux deux bornes), 
  `soldes_en_map()` ; champ `graphiques: GraphesData` dans `UiData`. Budget vs
  réel **relu** depuis `DashboardData.budget_mois` (même tri « plus tendus en
  tête » — zéro duplication).
- `views/graphiques.rs` (présentation pure, `UiState` gardé entre ouvertures) :
  sélecteur `selectable_value` + **un seul `Plot` actif à la fois** (zéro
  collision d'IDs, `id_salt` stables) ; répartition = barres horizontales
  peintes à la main `rect_filled(rect, 3u8, couleur)` (largeur fixe 320 px —
  piège egui 0.37 de la courbe vide) + `fmt::eur`/`fmt::pourcent` ; cascade =
  `BarChart` de 4 charts colorés + formateurs d'axes (X = libellés, Y =
  `fmt::nombre` via `Decimal::try_from` — f64 seulement au tracé) ; budget vs
  réel = 2 `BarChart` groupés + rappel des montants sous le tracé (rouge si
  dépassement).
- `app.rs` : variante `Onglet::Graphiques`, `TOUS: [Onglet; 7]`, `TOUCHES` +
  `Num7`, texte « Ctrl+1 à Ctrl+7 », `depuis_env` accepte `graphiques` /
  `graphes`, champ `graphiques: UiState` + bras `vue()`.
- Tests `tests/graphes.rs` (3) : identité comptable **exacte** Revenus −
  Dépenses == Δ patrimoine (== golden 4 932,16), couverture répartition
  (somme == `depenses_annee()` / total du mois, tri décroissant, zéros
  filtrés), cohérence budget vs réel ↔ cellules de la grille + tri tension.
  Aucun test sur le ledger réel sans scratch — ici lecture seule, comme
  `tests/viewmodel.rs`.

**Validation** : `just check` vert ✓ (fmt sur les tests corrigé au passage ;
clippy : `sort_unstable_by_key(Reverse)` et un borrow inutile dans le bras
`vue()`). Pas de surprise comptable : l'identité du waterfall est passée
**exacte** au premier essai.

**Passe de vérification visuelle (captures `COMPTA_ECRAN=graphes`, 2026-11-02)** —
deux découvertes supplémentaires, corrigées :
- **Barres « creuses »** : `BarChart::color()` ne teinte le **remplissage**
  qu'à 20 % et met la couleur en **contour** (source épinglée
  `egui_plot-0.38.0/src/items/bar_chart.rs:63` : `fill = plot_color
  .linear_multiply(0.2)`, stroke = couleur) → contour seule visible à l'écran.
  Correctif : `Bar::fill(couleur)` explicite sur chaque barre (le `.color()`
  du chart reste porteur de la légende).
- **Étiquettes d'axe X silencieusement absentes (barres groupées)** : la
  grille par défaut est log-10 → marques **sous-entières** ; l'axe X cache
  les étiquettes plus rapprochées que 60 px (`axis.rs::AxisHints::new_x`,
  `label_spacing: Rangef::new(60.0, 80.0)`) → un seul libellé affiché, et
  8 noms hiérarchiques ne tiennent pas côte à côte de toute façon.
  Correctif : `x_grid_spacer(uniform_grid_spacer(|_| [8.0, 4.0, 1.0]))`
  (marques aux entiers) + axe numéroté via `custom_x_axes` avec
  `label_spacing(16.0..80.0)` (numéro n+1 = poste n+1, la marque 0 porte le
  poste 1) ; liste détaillée **numérotée** sous le tracé ; noms complets au
  survol des barres (`Bar::name` + `nom_court`, deux derniers segments).
  Même `x_grid_spacer` appliqué à la cascade (robustesse).

Captures finales des trois états ✓ : répartition (total 8 393,29 € ; top
Logement 4 077,51 € 48,6 % ; barres horizontales proportionnelles, palette
fixe par index), cascade (13 325,45 − 8 393,29 = 4 932,16 visible à l'écran,
Épargne info 900,00 €), budget vs réel (2026-11 : budgeté 4,90/15 €, réel 0 —
cohérent au début du mois ; numéros 1..8 alignés sous les barres,
légende Budgeté (gris) / Réel (bleu)).

_Revue de code Rust complète (2026-11-01) — correctifs #1/#2/#4 appliqués,
#3/#5 restent au backlog « Revue de code » (À faire) — archivée ci-dessous._

**Constat général** : code idiomatique solide — `Decimal`/`NaiveDate` de bout en bout
vérifié, `thiserror` partout, séparation vues/view-model/logique pure réelle, writer
avec garanties « pro » (scratch, atomique, verrou, revalidation). Panics localisés aux
tests et à trois `expect` documentés. Trouvailles classées par gravité, numérotées —

- [x] **#1 — Le drain du debounce jette les commandes utilisateur** (`worker.rs`) :
  le drain post-`Reload` aspirait TOUTES les commandes en attente, écritures
  comprises → une action utilisateur (Ecrire/Modifier/Supprimer/CelluleBudget)
  arrivée pendant une rafale du watcher était perdue silencieusement (brouillon
  déjà remis à zéro côté UI). **Correctif** : le drain ne jette plus que les
  `Reload` redondants ; toute mutation rencontrée au passage est remise en fin de
  file via un émetteur `remise_en_file` (clone de `cmd_tx`) et traitée dans
  l'ordre, avec son propre résultat.
- [x] **#2 — Brouillon perdu en cas de refus** (`app.rs`) : `envoyer_brouillon`
  réinitialisait la saisie AVANT le résultat ; un `Refuse` (compte fermé,
  pré-vérification, empreinte introuvable) laissait l'écran Saisie vide alors que
  c'est précisément le cas où l'utilisateur veut corriger. **Correctif** : la remise
  à zéro n'a lieu qu'à la réception de `Ecrit` (et seulement si une saisie était en
  vol — un `Ecrit` budgétaire ne doit pas effacer un brouillon en cours) ; refus et
  échec conservent la saisie ; garde `saisie_en_vol` contre le double-clic ;
  `Annuler` déverrouille aussi la saisie.
- [x] **#4 — Replace à date changée non atomique** (`household-core/writer.rs`) :
  l'ancienne cascade `remove_transaction` + `append_transaction` = **deux** cycles
  complets (verrou/precheck/écriture/revalidation) — un crash entre les deux
  laissait la transaction supprimée sans jamais être réécrite. **Correctif** :
  une seule `edit_ledger_files` avec deux `FileChange` pré-vérifiés et écrits
  ensemble ; changement de jour **dans le même mois** = un seul patch composé
  (retrait puis insertion triée). Test de régression
  `replace_changement_de_date_une_seule_mutation` (intra-mois + inter-mois +
  soldes).
- [ ] **#3 — Empreinte ambiguë** : voir backlog ci-dessous.
- [ ] **#5 — Pré-vérification trop stricte** : voir backlog ci-dessous.

**Découverte nouvelle de cette passe** (à noter avec « écarts d'API jalon 2 ») :
la comparaison de deux chemins de la source map exige la **canonisation des deux
côtés** — la source map peut stocker des chemins canoniques (macOS :
`/var` → `/private/var/…`) pendant que `monthly_file` est construit sur le
chemin tel que chargé ; une égalité naive fait passer « même fichier » pour
« deux fichiers distincts » (corrigé dans `replace_transaction` via le
`canonize` existant).

_Passe UI « onglets → fenêtres flottantes » (2026-11-01) — terminée (commit ci-dessous)._

**Décisions** (tranchées avec la maîtresse d'ouvrage) : les six écrans deviennent des
`egui::Window` flottants ; le tableau de bord démarre quasi plein écran comme les
autres ; position/taille persistées par egui/eframe (zéro code) ; raccourcis Ctrl+1..6.

**Implémentation** (essentiellement `app.rs`) :
- `onglet: Onglet` (sélection unique) → `fenetres_ouvertes: [bool; 6]` aligné sur
  `Onglet::TOUS` ; `ouvrir`/`basculer` ; `vue(&mut self, ui, Onglet, &Arc<UiData>)`
  = extraction de l'ancien match (les vues restent inchangées).
- Barre du haut : `ui.toggle_value(&mut sel, libellé)` (signature 0.37 = `&mut bool`,
  pas `checked: bool`) ; l'ouverture de l'écran Git déclenche `GitStatut` à chaque
  ouverture.
- `LoadOutcome::Ecrit` : ancien `onglet = Journal` → `ouvrir(Journal)` (re-branché),
  l'annulation de saisie ferme la fenêtre Saisie et ouvre le Journal.
- Fenêtres : `Window::new(libellé).open(&mut ouvert).resizable(true).default_size(...)`
  — **egui 0.37 n'a pas de `maximized` pour les fenêtres internes** : effet « maximisé »
  = `default_size(viewport − 40 px)` pour le tableau de bord, 900×620 pour les autres.
  `default_size` n'est appliqué que hors mémoire (redimensionnements utilisateur
  respectés, y compris entre sessions via la persistance eframe). `min_width(460)`.
- Tampon d'ouverture/fermeture (croix) : egui écrit `open` → bool local reporté dans
  l'état après `show` (emprunt partagé).
- `COMPTA_ECRAN=<tab>` : ouvre **seulement** la fenêtre demandée (affordance de test,
  captures prévisibles) — doc `main.rs` mise à jour.
- `Ui::screen_rect` n'existe plus en 0.37 → `ctx.viewport_rect()`.
- `UiState` des vues vivent dans `FoyerApp` : fermer une fenêtre conserve le brouillon/
  filtres/année ; ré-ouverture les retrouve.
- Validation : `just check` vert (76 tests) + captures (tab de bord quasi plein écran,
  Git + 900×620 sans warning d'IDs, Journal avec filtres sur 2 rangées lisibles).

_Passe UI « vérification visuelle » (2026-10-31) — archivée ci-dessous._

**Méthode** : app lancée depuis le shell sur chaque onglet (`COMPTA_ECRAN=<tab>`),
capture `screencapture` après 6 s. La fenêtre d'un lancement via `open` panique dans
winit avant d'apparaître — lancer directement `./target/debug/compta-foyer`. Le
SIGTERM d'une app LaunchServices génère un dialogue de crash qui vole le premier plan
→ `kill -9` + `pkill -9 -f UserNotificationCenter` en début/fin d'itération. NB :
l'écran Mac peut se verrouiller entre deux itérations — `caffeinate -dims` pour la
durée d'une campagne de captures.

**Deux bugs trouvés par cette passe, hors dimensionnement statique** :
- **Courbe du tableau de bord silencieusement vide (cause racine trouvée par
  instrumentations `eprintln`, retirées après diagnostic)** : `desired_width(
  f32::INFINITY)` (jauges du dashboard, champ message de l'écran Git) dans un
  `ScrollArea::vertical` **dilate la largeur du contenu du scroll à l'infini**
  (egui 0.37) ; le `Plot` du dashboard grandit sur `Ui::available_width()` → il
  alloue un `plot_rect` infini (constaté : `[[52.7 571.5] - [inf 777.5]]`) →
  échelle X infinie, sommets/tessellation à l'infini, **rien ne se peint** (seul
  l'axe Y sur la tranche de gauche reste). Correctif : largeurs fixes
  (jauges 240 px, champ git 480 px) ; la courbe, la grille et les libellés X
  reviennent. Réplique `examples/plot_sanity.rs` fait le tour (montage +
  formateurs OK) puis **supprimé** ; `examples/dump_courbe.rs` gardé comme
  outil de dumping des données de la courbe.
- **Collision d'IDs de widgets aux captures d'écran de l'écran Git** (egui peint
  en rouge « First/Second use of Table/ScrollArea/widget ID… ») : deux
  `TableBuilder` dans le même `Ui` partagent l'id par défaut
  `ui.scope_id().with("__table_state")` (et les ids dérivés de leurs
  `ScrollArea` internes) → `id_salt` explicites `fichiers-git` /
  `historique-git`.

**Règle egui 0.37 complétée** : `desired_width(f32::INFINITY)` est interdit **aussi
bien dans un `egui::Grid` que dans tout contenu de `ScrollArea::vertical`** —
dans une cellule de table ou un slot de layout horizontal réel (parent fini),
il remplit le rect ; sinon, tout widget à largeur infinie dilate le conteneur.

**Corrections de dimensionnement vérifiées à la capture** (six onglets) :
- Tableau de bord : carte « Patrimoine net » en pile verticale (plus de
  `Grid`, le montant heading se repliait sur 3 lignes) ; jauges à largeur fixe
  240 px ; courbe peinte (X = libellés de mois via formateur, Y via `fmt::nombre`,
  couleur explicite & épaisseur 2.0, id stable `courbe-epargne`).
- Journal :angles `panneau_filtres` en **deux rangées explicites** `ui.horizontal`
  (recherche/compte/tag puis dates/montants) — le repli libre donnait un alignement
  chaotique en fin de ligne ; tableau enroulé horizontalement OK.
- Saisie : en-tête et lignes de postings en rangées `ui.horizontal` (plus de
  `Grid` — TextEdit coincide à ~interact_size ≈ 40 px dans un Grid, cf.
  `egui-0.37.0/src/grid.rs::available_rect` + `text_edit/builder.rs:572`) ;
  suppression des postings différée après la boucle (emprunt) ; « ＋ ligne » et
  « équilibrée ✓ » sur une ligne, plus de superposition.

`just check` vert (fmt + clippy `-D warnings` + 76 tests + `rledger check`).

_Passe UI « largeurs » (2026-10-30) — archivée ci-dessous._

**Découvertes egui 0.37 de la passe largeurs** (source de vérité :
`~/.cargo/registry/…/egui_extras-0.37.0/src/{table,sizing}.rs`) :
- `TableBuilder` crée son `ScrollArea` en `[false, vscroll]` — **pas de scroll horizontal
  intégré** ; une grille plus large que le panneau central est coupée, pas scrollée.
  Contournement (vue Budget) : enrouler la table dans un `egui::ScrollArea::horizontal()
  .auto_shrink([false; 2])` — le nesting vertical (table) / horizontal (extérieur) est
  supporté.
- `Column::remainder()` calcule le reste comme `max(0, disponible − autres colonnes)` :
  quand la somme des colonnes `auto()` dépasse la largeur disponible, une colonne
  remainder sans plancher s'écrase à **0**. Toujours `.at_least(min)` sur un remainder
  de contenu important (`sizing.rs::to_lengths` : le min est ensuite clampé dans le
  range et le total peut déborder → c'est le scroll horizontal extérieur qui absorbe).
- Colonnes à contenu variable : `Column::auto().at_least(min)` (exact fige et coupe) ;
  `at_least` suit la même voie que pour les cellules de la grille.
- `desired_width(f32::INFINITY)` : OK dans une cellule de table ou un slot de layout
  horizontal à parent fini (remplit le rect réel) ; **interdit dans un `egui::Grid`**
  (mesure à 2 passes, peut dépasser la fenêtre) et **interdit dans un
  `ScrollArea::vertical`** (dilate la largeur du contenu à l'infini — cf. la passe
  de vérification visuelle, tracé du dashboard vide). Dans les grilles et scrolls,
  élargir les valeurs fixes.
- `ProgressBar.text(...)` est **tronqué à la largeur de la barre** : mettre le texte
  dans un `ui.label` avant la barre, et la barre à **largeur fixe** (la pleine
  largeur infinie a causé le bug du ScrollArea, cf. ci-dessus).

**Détail des réglages appliqués** : colonnes budget `exact(56/72/76)` →
`auto().at_least(52/88/84)` + remainder `.at_least(180)` ; champ d'édition de cellule
`desired_width(f32::INFINITY)` ; historique Git `exact(76/110/150)` →
`auto().at_least(80/120/140)` ; filtres journal 180→240 / 200→300 / 110→130, montants
90→110, dates 105→130 ; saisie libellé/tags/liens 340→420, tiers 240→320, montant
120→150, date 110→130 ; dashboard jauges : texte sorti de la barre + barre pleines
largeur, barres carte budget 180→240 ; fenêtre 1200×760 → 1300×800 (min 980×620 →
1060×680). **Second tour (retour utilisateur : Journal + Saisie)** :
- Tableau du journal enroulé dans `ScrollArea::horizontal().auto_shrink([false; 2])`
  (mêmes causes que la grille budget : colonnes auto longues, pas de scroll
  horizontal dans `TableBuilder`, remainder du libellé qui s'écroule à 0) + minima
  `at_least` sur toutes les colonnes (date 88, tiers 100, libellé remainder 150,
  comptes 160, montant 90, tags 120).
- Champ compte de la Saisie élargi à 460 px : `champ_suggestions` devient
  paramétrable en largeur (un compte complet ≠ un tiers).
- Filtre compte du journal 260→300 (préfixe complet attendu).
- Cartes « Soldes des comptes » : chaque carte devient un `Frame::group` teinté
  avec `ui.set_min_size(210×…)` (egui 0.37 : `Frame` n'a pas de `min_size`,
  passer par `Ui::set_min_size` dans le contenu) —montant en taille heading
  jamais contraint, taille de carte stable.
`cargo check`/fmt/clippy/tests/`rledger check` verts après passe.

## À faire

### Séparation exemple / vrai ledger — restes (jalon 8, 2026-11-07)

- [x] Sélecteur de fichier pour `chemin_ledger` dans la GUI (2026-11-07) :
  fenêtre « Ledger actif » (bouton `Ledger…` de la barre d'en-tête) —
  résolution affichée (chemin, source, avertissement), `rfd` picker
  « Choisir un main.beancount… », bouton « Utiliser le ledger d'exemple »
  (efface le champ), effet au prochain lancement ; réglages désactivés si la
  source est argument/`RPA_LEDGER` (`SourceResolution::surcharge_config`).
  Dép. `rfd 0.17` (MIT/Apache-2.0).
- [ ] Vérifications visuelles des quatre échelons de résolution (argument >
  `RPA_LEDGER` > champ config > exemple) : badge, bannière d'échec du
  chargement, flash d'avertissement.
- [ ] ADR (ou complément ADR-002) : le vrai ledger du foyer est un dépôt git
  séparé — y relancer `just setup` une première fois pour ses hooks.

### Revue de code — suite (2026-11-01, post-v0.1.0)

Trouvailles de la revue complète non encore traitées (le détail est dans l'entrée
« Revue de code Rust complète » de « En cours » ; #1/#2/#4 sont déjà corrigés).
Ordre de priorité proposé :

- [ ] **#5 — Pré-vérification trop stricte** (`household-core/writer.rs`) : la
  précheck exige `is_valid()` = zéro diagnostic, warnings compris — un W
  préexistant sans rapport avec l'écriture bloquerait toute mutation pour
  toujours. Filtrer sur les erreurs fatales et comparer le diagnostic
  avant/après (nuisance des diagnostics **nouveaux** seulement).
- [ ] **#3 — Empreinte ambiguë** (`worker.rs::localiser`) : deux transactions
  jumelles le même jour (même date + tiers + libellé) satisfont la même
  empreinte et `position()` peut rewriter la mauvaise ligne. Ajouter un
  discriminant à l'empreinte (montant principal, ou `fichier:ligne` d'origine
  en secours quand l'index est encore valide).
- [ ] **#6 — Verrou sans garde Drop** (`household-core/writer.rs`) : un panic
  dans le corps de `with_ledger_lock` laisse `.write.lock` orphelin
  (nettoyage anti-stale repoussé à 120 s + demi-attente). Un petit guard
  struct avec `Drop` qui retire le fichier de verrou.
- [ ] **#8 — Préférences non atomiques** (`household-gui/config.rs`) : écriture
  directe (pas de tmp+rename) et TOML corrompu → défauts silencieux puis
  réécriture (préférences écrasées sans trace). Réutiliser une écriture
  atomique + un diagnostic au démarrage.
- [ ] **#10 — Courbe du patrimoine O(mois × transactions)** (`viewmodel.rs`) :
  `courbe_patrimoine` relance un `balances_at` complet par point (rejeu du
  booking sur TOUT le ledger à chaque mois). Une seule passe incrémentale
  le long de la chronologie suffit (appliquer les bookings et consigner le
  dernier jour de chaque mois). Même axe pour `balance_of`.
- [ ] **#7 — Calendrier maison** (`viewmodel.rs`) : `fin_de_mois`/
  `est_bissextile` réimplémentent jiff avec `expect` (mois 0 → panic sur
  une API `pub`) ; `month_start_exclusive` (`household-budget/grid.rs`) fait
  `unwrap_or(year)` — année fausse silencieuse à `i16::MAX`. Passer à jiff
  (`days_in_month`, arithmétique de dates) et retourner `Option`.
- Nits à grappiller (sans urgence) :
  - `viewmodel::rapports` classe par `starts_with("Expenses")` sans `:` —
    utiliser `AccountTypeKind` comme le fait déjà `patrimoine_net` ;
    `"Assets:Epargne"`/`"EUR"` hardcodés vs options du ledger.
  - `household-core/writer.rs` : `fn ioutil_header` (goïsme → `entete_mensuel`)
    ; paramètre `_file_id` vestige de `rewrite_span`.
  - `household-budget/writer.rs` : `ensure_include` insère après le dernier
    include de budget sans trier par année.
  - `household-gui/saisie.rs` : la contrepartie d'équilibrage avec résidu déjà
    nul écrit un posting `0.00 EUR` (bruit dans le fichier).
  - `household-git` : troncature d'OID `[..7]` littérale ; `touche_chemin` ne
    compare que le parent 0 (commits de fusion) ; TOCTOU statut→commit.
  - `household-gui/worker.rs` : `cmd_tx.send(...).ok()` ignore la fermeture du
    canal (déférence — le worker ne panic plus, à reconsidérer quand même).

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

- [x] Écran **Rapports** (`views/rapports.rs` + `viewmodel::rapports`) : synthèse de l'année
  (dépenses, revenus, épargne, taux d'épargne), **comparaison N vs N−1 en barres**
  (`egui_plot 0.38` : 2 × `BarChart` de 12 barres, noms = mois, bandeau absent si N−1 sans
  dépenses), **heatmap** catégories × mois (`TableBuilder`, intensité relative au max de la
  ligne) + ligne TOTAL et colonne Année ; conversion `f64` uniquement au tracé.
- [x] `household-git` (git2 0.21, libgit2 statique) : `decouvrir`, `statut`
  (nouveau/modifié/supprimé/renommé, index + arbre cumulés), `tout_indexer` (add_all +
  update_all — les suppressions passent), `commit` (config git du dépôt/global, replis
  locaux ; premier commit sans parent supporté), `historique` (revwalk TIME, filtre par
  chemin via comparaison d'entrée d'arbre parent/enfant — création/modif/suppression,
  dates jiff UTC), `resume_diff` (**include_untracked** sinon les nouvelles transactions
  manquent au décompte), `suggere_message` (`ledger: AAAA-MM — saisie/édition`, agnostique
  au préfixe `ledger/`). 3 tests sur dépôts temporaires **réels** (libgit2).
- [x] Écran **Git** (`views/git.rs` + `worker::git_resultat`) : statut par fichier (tableau),
  résumé du diff (fichiers/insertions/suppressions), message de commit **prérempli par la
  suggestion** (re-préremplie quand elle change ; tampon utilisateur respecté sinon),
  bouton commit désactivé si rien à commiter, historique filtré sur `ledger`
  (OID/date/auteur/sujet), bouton Actualiser.
- [x] **Validation du ledger avant commit côté app** (`Command::GitCommit` →
  `Snapshot::load`, refus en flash si le ledger est cassé) — git2 n'exécute pas les hooks ;
  choix documenté dans `docs/ADR-002` et rappelé dans l'écran.
- [x] Gestion d'erreurs UI homogène : flash (succès vert / erreur rouge, clic =
  acquittement) partout, chaînes `thiserror` → `LoadOutcome::Refuse/Echec`, écran Git
  `disponible=false` = message informatif (jamais d'écran de mort).
- [x] README complet (installation, lancement, 6 écrans décrits, conventions, exemple de
  budget, workflow git, rôle du TODO.md).
- [x] `/docs` : ADR-001 (rustledger + GPL-3.0), ADR-002 (git2 + validation côté app),
  ADR-003 (budget Fava + rollover cumul + épargne destinataire + patrimoine Equity exclu).
- [x] CI complète verte + **tag `v0.1.0`**.
- [x] Commit jalon 7.

**Découvertes du jalon 7** :
- git2 0.21 : `StatusEntry::path()` renvoie `Result`, `Commit::summary()` renvoie
  `Result<Option<&str>>` (`ok().flatten()`) ; `DiffOptions::include_untracked(true)`
  indispensable pour que `resume_diff` voie les nouveaux fichiers non suivis.
- `Repository::discover` marche depuis n'importe quel chemin englobé (fichier du ledger
  compris) ; **un commit via libgit2 n'exécute pas le hook pre-commit** — d'où la
  revalidation dans le worker (choix ADR-002).
- Les chemins du statut sont **relatifs à la racine du dépôt**
  (`ledger/transactions/2026/04.beancount`) : le message suggéré parse donc n'importe quel
  préfixe devant `transactions/AAAA/MM.beancount`.

## Fait

- [x] **Jalon 8 — Séparation ledger d'exemple / vrai ledger du foyer (2026‑11‑07)** :
  suite à l'audit PII/RGPD du dépôt (0 secret, 0 email nominatif ; question du ledger
  réel vs exemple soulevée). `/ledger` → `examples/ledger/` (fail-safe), bannières
  « données fictives » + `examples/ledger/README.md` ; module pur
  `household-core::resolution` (`resoudre_ledger` : argument > `RPA_LEDGER` >
  champ `chemin_ledger` > repli exemple ; jamais de repli silencieux si le
  chemin configuré est introuvable) ; CLI `household-cli` sans argument résout
  via la chaîne ; config GUI renommée `~/.config/rust-personal-accounting.toml`
  (migration douce depuis `compta-foyer.toml`, champ `chemin_ledger`) ; badge
  GUI « LEDGER D'EXEMPLE — données fictives » ; chemins mis à jour dans
  justfile / hook pre-commit / CI / tests. 14 nouveaux tests verts.
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
- [x] **Jalon 7 — rapports, git, polish (2026‑10‑30)** : écran Rapports (heatmap catégories
  × mois, barres N vs N−1, revenus/épargne/taux d'épargne), crate `household-git` (git2 :
  statut/index/commit/historique filtré/résumé diff/message suggéré — 3 tests sur dépôts
  réels), écran Git (commit avec suggestion + **validation du ledger préalable côté app**,
  ADR-002), gestion d'erreurs UI homogène, README complet, `/docs` (3 ADR), tag `v0.1.0` ;
  76 tests verts au total.

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
