# Proposal

## Why

Renommer le fichier de configuration et son répertoire pour utiliser la casse PascalCase (`RustPersonnalAccounting`) au lieu de kebab-case (`rust-personal-accounting`), et uniformiser le nom du fichier (`config.toml` au lieu de `rust-personal-accounting.toml`). Cela aligne la configuration avec le nom du projet et simplifie la structure (un répertoire unique pour toute la config applicative).

Supprimer toute logique de migration/fallback vers les anciens chemins — changement **BREAKING** : les utilisateurs existants doivent déplacer manuellement leurs fichiers.

## What Changes

- **BREAKING** : Fichier de config déplacé de `~/.config/rust-personal-accounting.toml` → `~/.config/RustPersonnalAccounting/config.toml`
- **BREAKING** : Répertoire importers déplacé de `~/.config/rust-personal-accounting/` → `~/.config/RustPersonnalAccounting/` (fichier `importers.toml` inchangé)
- **BREAKING** : Suppression complète de la logique de migration :
  - Plus de lecture de `~/.config/rust-personal-accounting.toml` (ancien nom)
  - Plus de lecture de `~/.config/compta-foyer.toml` (v0.1)
  - Plus de fallback vers `~/.config/rust-personal-accounting/importers.toml`
- Mise à jour de toutes les références dans le code, la documentation, les specs
- Tests mis à jour pour refléter les nouveaux chemins

## Capabilities

### Modified Capabilities

- `ouvrir-ledger` : La chaîne de résolution cite le champ `chemin_ledger` dans `~/.config/rust-personal-accounting.toml` — doit pointer vers `~/.config/RustPersonnalAccounting/config.toml`
- `importer-releve` : Le profil d'import par défaut est stocké dans le même fichier de config ; la spec mentionne "préférences applicatives" — le chemin change implicitement
- `regler-application` : Les préférences (rollover, chemin ledger, profil import) vivent dans ce fichier — le chemin change

## Impact

**Code** :
- `crates/household-gui/src/config.rs` — logique complète de chemin, suppression migration
- `crates/household-core/src/importer.rs` — résolution `importers.toml`, suppression fallback
- `crates/household-gui/src/views/import.rs` — string UI
- `crates/household-gui/src/worker.rs` — message d'erreur
- `crates/household-core/src/resolution.rs` — doc comment
- `crates/household-gui/src/main.rs` — doc comment

**Documentation** :
- `AGENTS.md`, `README.md`, `docs/ADR-004-vrai-ledger-depot-separe.md`, `docs/ADR-005-conventions-du-ledger.md`, `examples/ledger/README.md`

**Specs** :
- `openspec/specs/ouvrir-ledger/spec.md`
- `openspec/specs/importer-releve/spec.md` (si mention explicite du chemin)
- `openspec/specs/regler-application/spec.md` (si mention explicite du chemin)

**Tests** : Tests dans `config.rs` et `importer.rs` mis à jour pour nouveaux chemins