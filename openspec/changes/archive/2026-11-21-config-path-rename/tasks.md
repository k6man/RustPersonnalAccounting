# Tasks

## 1. Core config.rs — new path, remove migration

- [ ] 1.1 Update `Config::chemin()` to return `~/.config/RustPersonnalAccounting/config.toml` — verify `Config::chemin()` returns new path
- [ ] 1.2 Remove `Config::ancien_chemin()` (was for `compta-foyer.toml`) — verify method no longer exists
- [ ] 1.3 Remove `Config::charger_avec(nouveau, ancien)` migration logic — verify `Config::charger()` calls `charger_de(chemin())` directly
- [ ] 1.4 Update `Config::sauver()` fallback to use new `chemin()` — verify `sauver()` works with new path
- [ ] 1.5 Remove migration tests: `migration_lit_ancien_fichier_et_donne_le_nouveau_comme_cible`, `migration_nouveau_prioritaire_sil_existe`, `contenu_migre_re_persiste_dans_le_nouveau` — verify tests deleted
- [ ] 1.6 Update remaining tests to use new path construction — verify `cargo test -p household-gui config` passes

## 2. Importer resolution — new path, remove fallback

- [ ] 2.1 Update `resoudre_importers()` step 2: change from `~/.config/rust-personal-accounting/importers.toml` to `~/.config/RustPersonnalAccounting/importers.toml` — verify resolution finds file at new location
- [ ] 2.2 Update `SourceImporters::ConfigLocale` label to reflect new path — verify `libelle()` returns new path string
- [ ] 2.3 Remove fallback to old `rust-personal-accounting` directory entirely — verify old path no longer checked
- [ ] 2.4 Update tests in `importer.rs` for new path — verify `cargo test -p household-core importer` passes

## 3. UI strings & error messages

- [ ] 3.1 Update `crates/household-gui/src/views/import.rs` line 395: `~/.config/rust-personal-accounting/` → `~/.config/RustPersonnalAccounting/` — verify string updated
- [ ] 3.2 Update `crates/household-gui/src/worker.rs` line 761: error message path — verify message updated
- [ ] 3.3 Update `crates/household-gui/src/main.rs` doc comment (line 5) — verify comment updated
- [ ] 3.4 Update `crates/household-core/src/resolution.rs` doc comment (line 9) — verify comment updated

## 4. Documentation updates

- [ ] 4.1 Update `AGENTS.md` lines 98, 154 — verify both references updated
- [ ] 4.2 Update `README.md` line 55 — verify reference updated
- [ ] 4.3 Update `docs/ADR-004-vrai-ledger-depot-separe.md` line 27 — verify reference updated
- [ ] 4.4 Update `docs/ADR-005-conventions-du-ledger.md` line 33 — verify reference updated
- [ ] 4.5 Update `examples/ledger/README.md` line 22 — verify reference updated

## 5. Spec updates (deltas already created — verify they apply correctly)

- [ ] 5.1 Verify `ouvrir-ledger` delta updates config path in requirement "Chaîne de résolution par priorité" — run `openspec validate --all --strict` passes
- [ ] 5.2 Verify `regler-application` delta removes migration requirement — run `openspec validate --all --strict` passes

## 6. Integration verification

- [ ] 6.1 Run `just check` — verify fmt, clippy, tests, rledger check all pass
- [ ] 6.2 Run `cargo test -p household-gui --test gui_tui` — verify TUI tests pass
- [ ] 6.3 Manual test: create `~/.config/RustPersonnalAccounting/config.toml` with `chemin_ledger`, launch app, verify ledger loads — verify config read from new location
- [ ] 6.4 Manual test: create `~/.config/RustPersonnalAccounting/importers.toml`, open Import screen, verify profiles loaded — verify importers read from new location

## Workflow follow-up

- Archive the change after review requirements are satisfied: `openspec archive config-path-rename`
- Verify archived specs match deltas: `openspec show "ouvrir-ledger" --type spec` and `openspec show "regler-application" --type spec`