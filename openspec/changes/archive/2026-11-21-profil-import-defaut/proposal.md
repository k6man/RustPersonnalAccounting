# Proposal

## Why

Les utilisateurs BNP (ou d'autres banques) importent régulièrement des relevés et doivent à chaque fois sélectionner manuellement le bon profil d'import (ex: `bnp-csv` ou `bnp-ofx`) via les touches ←/→. Un profil par défaut configurable dans les préférences applicatives éviterait cette friction répétitive.

## What Changes

- Ajout d'un champ optionnel `profil_import_defaut` dans `~/.config/rust-personal-accounting.toml`
- L'écran Import sélectionne automatiquement ce profil s'il existe dans les profils chargés
- Aucun changement destructif : si le profil nommé est absent, le comportement actuel (premier profil) s'applique

## Capabilities

### Modified Capabilities

- `importer-releve` : Ajout d'une exigence pour le profil d'import par défaut configurable

## Impact

- `crates/household-gui/src/config.rs` : Nouveau champ `profil_import_defaut` dans `Config`
- `crates/household-gui/src/app.rs` : Transmission du défaut à l'écran Import au démarrage
- `crates/household-gui/src/views/import.rs` : Stockage et utilisation du défaut dans `charger_profils`
- `openspec/specs/importer-releve/spec.md` : Delta de spec pour la nouvelle exigence