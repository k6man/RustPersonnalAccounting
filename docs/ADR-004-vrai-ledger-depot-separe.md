# ADR-004 — Le vrai ledger du foyer : dépôt git séparé, jamais dans le dépôt logiciel

**Statut** : acceptée (2026-11-07, jalon 8.3).

## Contexte

Le dépôt logiciel ne doit contenir que des **données fictives** :
`examples/ledger/` est la recette du logiciel (validé en CI, recopié en
scratch par les tests, jamais la comptabilité réelle). La comptabilité du
foyer, elle, existe déjà (usage réel) et doit vivre quelque part. Trois
voies :

1. **un répertoire du dépôt logiciel** (ex. `ledger/`, comme avant le jalon
   8) : données réelles versionnées avec le code — push accidentel de
   finances personnelles, CI du logiciel rejouant des données réelles,
   tests qui risquent le vrai ledger, fusion code/donnees dans les diffs ;
2. un répertoire externe **sans git** : pas d'historique, pas de diffs
   propres, pas de rollback — l'app offre pourtant l'écran Git (ADR-002) ;
3. **un dépôt git séparé, hors du dépôt logiciel**.

## Décision

3. **Le vrai ledger du foyer est un dépôt git indépendant** (propre
   arborescence, son propre historique), référencé par la chaîne de
   résolution partagée `household_core::resolution` (argument CLI >
   `RPA_LEDGER` > champ `chemin_ledger` de
   `~/.config/rust-personal-accounting.toml` > repli sur l'exemple avec
   badge). Convention de forme : un `main.beancount` à la racine du dépôt
   qui inclut le reste (même canon que l'exemple).

- La même **défense en profondeur** qu'ADR-002 s'applique aux commits faits
  à la main dans ce dépôt : le hook `pre-commit` (validation du ledger,
  refus si erreur) **fonctionne aussi hors du dépôt logiciel** — il cherche
  le point d'entrée dans le dépôt courant (`main.beancount` à la racine ;
  `examples/ledger/main.beancount` dans le dépôt logiciel lui-même).
- Son attachement est mécanisé : `just setup-ledger <dossier-du-ledger>`
  configure `core.hooksPath` du dépôt du ledger vers le `githooks/` du
  dépôt logiciel (chemin absolu) — un seul exemplaire du hook, le vrai
  ledger n'embarrasse pas le dépôt logiciel d'un fichier mineur ; à ne faire
  **qu'une première fois** après création du dépôt du ledger.
- L'écran Git de la GUI (ADR-002, git2/libgit2) opère sur le dépôt **du
  ledger** : les chemins du statut sont relatifs à sa racine ; la
  reproduction côté app de la validation du hook reste requise (libgit2
  n'exécute pas les hooks).

## Conséquences

- Le dépôt logiciel et le dépôt du foyer n'ont **aucun lien de versionnement**
  (pas de submodule, pas de remote partagé) : l'un est du code + des fix-
  tures, l'autre des données réelles. Un push de l'un ne peut jamais fuiter
  l'autre.
- `RPA_LEDGER` et le champ de config sont des chemins *côté machine* : le
  ledger résolu peut bouger sans impact code ; le repli (démo auto) est
  banni d'un chemin configuré mais non trouvé (`resolution`, pas de repli
  silencieux).
- Le hook commun valide toujours le dépôt *dans lequel on committe* :
  ajouter un troisième dépôt beancount (famille, associatif…) reviendrait à
  poser `main.beancount` à sa racine et à re-pointer `core.hooksPath`.
- La recette de tests du logiciel reste sur l'exemple ; les données réelles
  ne sont ni lues ni écrites par la CI.
