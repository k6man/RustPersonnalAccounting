# Design

## Context

Voir `proposal.md` pour la motivation. L'écran Import charge les profils depuis `importers.toml` (via `charger_importers_pour_ledger`) et sélectionne le premier par défaut (index 0). La config applicative (`~/.config/rust-personal-accounting.toml`) est déjà chargée au démarrage dans `App::config`.

## Goals / Non-Goals

**Goals:**
- Ajouter `profil_import_defaut: Option<String>` dans `Config`
- Transmettre ce défaut à `UiState` de l'écran Import
- Modifier `charger_profils` pour sélectionner le profil nommé s'il existe

**Non-Goals:**
- Modifier le format de `importers.toml`
- Changer l'ordre des profils dans l'UI
- Ajouter une UI pour configurer le défaut (éditer le TOML directement suffit)

## Decisions

1. **Stockage dans `Config` (préférences applicatives)** — Le défaut est un réglage utilisateur, pas une donnée de ledger. ADR-005 décide que les préférences app vivent hors ledger.
2. **Propagation via `App::new`** — `App` crée `UiState::default()` puis charge `Config`. On assigne `app.import.profil_defaut = app.config.profil_import_defaut.clone()` après.
3. **Recherche par nom dans `charger_profils`** — Si `profil_defaut` est `Some(nom)`, on cherche `profils.iter().position(|p| p.name == nom)`. Si trouvé, `profil_selectionne = position`; sinon fallback index 0.
4. **Pas de migration de config** — Champ optionnel, absent = comportement actuel. `serde(default)` gère le cas fichier absent.

## Risks / Trade-offs

- [Risque] Profil nommé existe dans un `importers.toml` mais pas dans un autre (ledger pro vs perso) → Mitigation: fallback silencieux à l'index 0, pas d'erreur.
- [Risque] Utilisateur définit un nom qui n'existe plus après modification `importers.toml` → Mitigation: même fallback, l'utilisateur voit le premier profil et peut naviguer.