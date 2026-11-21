# Design

## Context

See `proposal.md` for motivation. Current state:
- Config file at `~/.config/rust-personal-accounting.toml` with migration from `~/.config/compta-foyer.toml`
- Importers at `~/.config/rust-personal-accounting/importers.toml` with fallback from ledger root
- `config.rs` contains `ancien_chemin()`, `charger_avec()` migration logic, tests for migration
- `importer.rs` checks `~/.config/rust-personal-accounting/importers.toml` as second priority
- Multiple docs and specs reference the old paths

## Goals / Non-Goals

**Goals:**
- Single config directory: `~/.config/RustPersonnalAccounting/` containing `config.toml` and `importers.toml`
- Zero fallback/migration logic — clean break
- All references updated in code, docs, specs, tests

**Non-Goals:**
- No automated migration tool (users move files manually)
- No symlink/compat layer
- No deprecation period

## Decisions

### 1. New paths

| Artifact | Old | New |
|----------|-----|-----|
| Config file | `~/.config/rust-personal-accounting.toml` | `~/.config/RustPersonnalAccounting/config.toml` |
| Importers file | `~/.config/rust-personal-accounting/importers.toml` | `~/.config/RustPersonnalAccounting/importers.toml` |

**Rationale**: PascalCase matches project name (`RustPersonnalAccounting`). `config.toml` is generic/standard. Single directory groups all app config.

### 2. Remove all migration code in `config.rs`

- Delete `ancien_chemin()` — was for `compta-foyer.toml` v0.1
- Delete `charger_avec(nouveau, ancien)` — migration logic
- Simplify `charger()` → calls `charger_de(Self::chemin())` directly
- Update `chemin()` to return new path
- Update `sauver()` to use new path (already uses `chemin_charge` or `chemin()`)

**Rationale**: Clean break. No dead code. Tests for migration removed.

### 3. Remove fallback in `importer.rs`

- `resoudre_importers()`: remove step 2 (`~/.config/rust-personal-accounting/importers.toml`)
- `SourceImporters::ConfigLocale` enum variant → remove or rename to reflect new path
- Priority becomes: 1. ledger root, 2. new config dir, 3. none

**Rationale**: Single source of truth. No confusing dual locations.

### 4. Spec deltas

- `ouvrir-ledger`: Update requirement "Chaîne de résolution par priorité" — replace old path with new
- `regler-application`: **REMOVE** requirement "Migration de l'ancien fichier" entirely (no replacement)
- `importer-releve`: No delta needed — says "préférences applicatives" abstractly

### 5. Documentation updates

Update all `.md` files referencing old paths (listed in proposal.md Impact section).

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Existing users lose config silently | Document clearly in release notes; config file is small (ledger path, rollover map, import profile) — easy to recreate |
| Tests fail if they assume old paths | Update all tests in this change (config.rs, importer.rs tests use temp files, so just update path construction) |
| `regler-application` spec loses a requirement | Intentional — migration was for v0.1→v0.2, now v0.2→new is clean break |