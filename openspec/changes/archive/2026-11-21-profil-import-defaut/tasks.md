# Tasks

## 1. Config — ajouter le champ de préférence

- [x] 1.1 Ajouter `profil_import_defaut: Option<String>` dans `Config` (`crates/household-gui/src/config.rs`) avec `#[serde(default, skip_serializing_if = "Option::is_none")]` — vérifier compilation
- [x] 1.2 Étendre le test `round_trip_toml` pour inclure `profil_import_defaut` (config.rs tests) — `cargo test -p household-gui config::tests::round_trip_toml`
- [x] 1.3 Ajouter test `profil_import_defaut_round_trip` dédié : créer config avec `profil_import_defaut = "bnp-csv"`, sauvegarder, recharger, vérifier que le champ est préservé — `cargo test -p household-gui config::tests::profil_import_defaut_round_trip`

## 2. App — propager le défaut à l'écran Import

- [x] 2.1 Dans `App::new`, après chargement de `config`, assigner `self.import.profil_defaut = self.config.profil_import_defaut.clone()` — vérifier compilation
- [x] 2.2 Ajouter champ `profil_defaut: Option<String>` dans `UiState` (`crates/household-gui/src/views/import.rs`) — vérifier compilation

## 3. Import — utiliser le défaut lors du chargement des profils

- [x] 3.1 Modifier `charger_profils` pour accepter le défaut stocké dans `UiState` : chercher `profils.iter().position(|p| p.name == self.profil_defaut.as_ref().unwrap())` et assigner `profil_selectionne` si trouvé, sinon fallback 0 — vérifier compilation
- [x] 3.2 Ajouter test d'intégration dans `tests/gui_tui.rs` : configurer `profil_import_defaut = "bnp-csv"`, ouvrir l'écran Import, vérifier que `profil_selectionne` pointe vers `bnp-csv` — `cargo test -p household-gui --test gui_tui import_profil_defaut_bnp`
- [x] 3.3 Ajouter test d'intégration : défaut configuré mais absent des profils chargés → fallback index 0 — `cargo test -p household-gui --test gui_tui import_profil_defaut_absent_fallback`

## 4. Validation complète

- [x] 4.1 `just check` passe (fmt, clippy, tests, rledger check)
- [ ] 4.2 Test manuel : créer `~/.config/rust-personal-accounting.toml` avec `profil_import_defaut = "bnp-csv"`, lancer l'app, aller à l'écran Import → profil BNP pré-sélectionné

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived result with `openspec validate --all --strict`.