# ADR-002 — Versionnement du ledger : git2 (libgit2), validation côté app

**Statut** : acceptée (2026-10-29, jalon 7).

## Contexte

Le ledger est versionné git (source de vérité, diffs propres). Trois voies
pour la GUI :

1. sheller le binaire `git` (coquille fragile, pas portable) ;
2. **git2** (libgit2 en processus) ;
3. laisser l'utilisateur commiter à la main, la GUI étant purement
   consultative.

Par ailleurs, le hook `pre-commit` existant valide le ledger avec
`household-cli` — mais libgit2 (comme les API git programmatiques) **n'exécute
pas les hooks**.

## Décision

- **git2 (libgit2) en processus** : pas de dépendance à un binaire externe,
  statut/stage/commit testés unitairement sur dépôts réels
  (`crate household-git`).
- **L'application valide le ledger AVANT de commiter** (chargement complet
  `Snapshot::load`, refus si une erreur). Ce contrôle côté app est
  *complémentaire* du hook : les commits faits depuis la GUI n'exécutent pas
  le hook, donc la même garantie est reproduite dans le worker. Un commit
  fail-safe : message suggéré `ledger: AAAA-MM — saisie/édition (N fichiers)`,
  stade-index complet avant commit.
- Le message et le flux restent visibles/éditables à l'écran Git : pas de
  commit silencieux.

## Conséquences

- La GUI fonctionne sans `git` installé (libgit2 vendu statiquement).
- Un commit via la GUI et un commit via la CLI partagent la même propriété
  de validation, par deux voies — documentée ici pour le rapprocher des
  audits.
- Historique limité aux chemins du `ledger/` (filtres par arbre, pas de
  diff complet affiché).
