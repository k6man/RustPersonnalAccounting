//! Lecture et résolution de `importers.toml` pour `rledger extract`.
//!
//! Ce module fournit :
//! - La structure `ImportersConfig` : désérialisation du fichier TOML
//! - La structure `ImporterProfile` : un profil d'import (CSV/OFX) avec ses options
//! - La fonction `resoudre_importers()` : chaîne de résolution identique à celle
//!   du ledger (fichier ledger → config locale → défauts), pour que l'application
//!   et la CLI trouvent le même fichier.
//!
//! Ordre de recherche (le premier existant gagne) :
//!   1. `importers.toml` à la racine du ledger (à côté de `main.beancount`) ;
//!   2. `~/.config/rust-personal-accounting/importers.toml` ;
//!   3. Aucun fichier (retourne config vide — `rledger extract` utilisera ses
//!      moteurs intégrés CSV/OFX sans profils nommés).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Configuration complète désérialisée depuis `importers.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportersConfig {
    /// Liste des profils d'import (ordre = ordre d'affichage dans l'UI).
    #[serde(default)]
    pub importers: Vec<ImporterProfile>,
}

impl ImportersConfig {
    /// Charge la configuration depuis le chemin résolu.
    /// Fichier absent ou illisible → config vide (jamais de panique).
    #[must_use]
    pub fn charger(chemin: Option<&Path>) -> Self {
        let Some(chemin) = chemin else {
            return Self::default();
        };
        match std::fs::read_to_string(chemin) {
            Ok(texte) => match toml::from_str(&texte) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!(
                        "⚠ importers.toml illisible ({}) : {e} — config vide utilisée",
                        chemin.display()
                    );
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}

/// Un profil d'import nommé (moteur CSV ou OFX + options).
///
/// Les champs correspondent à la structure attendue par `rledger extract` :
/// `type` (pas `importer`), et les options CSV à plat (pas de table `options` imbriquée).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImporterProfile {
    /// Identifiant unique — utilisé par `--profile` / `--importer`.
    pub name: String,
    /// Moteur d'import : `"csv"` ou `"ofx"` (champ `type` dans le TOML).
    #[serde(rename = "type")]
    pub r#type: String,
    /// Compte cible par défaut (peut être surchargé par `--account`).
    pub account: String,
    /// Devise par défaut (peut être surchargée par `--currency`).
    #[serde(default)]
    pub currency: Option<String>,
    /// Options CSV — toutes au niveau de la table (aplaties via `flatten`).
    /// Inconnues ignorées grâce à `default` sur HashMap.
    #[serde(default, flatten)]
    pub options: HashMap<String, String>,
}

impl ImporterProfile {
    /// Vrai si le profil utilise le moteur CSV.
    #[must_use]
    pub fn est_csv(&self) -> bool {
        self.r#type.eq_ignore_ascii_case("csv")
    }

    /// Vrai si le profil utilise le moteur OFX/QFX.
    #[must_use]
    pub fn est_ofx(&self) -> bool {
        self.r#type.eq_ignore_ascii_case("ofx") || self.r#type.eq_ignore_ascii_case("qfx")
    }

    /// Construit les arguments `rledger extract` pour ce profil.
    /// Ne comprend PAS le fichier source (à ajouter par l'appelant).
    #[must_use]
    pub fn arguments_rledger(&self) -> Vec<String> {
        let mut args = vec![
            "--profile".to_string(),
            self.name.clone(),
            "--account".to_string(),
            self.account.clone(),
        ];
        if let Some(currency) = &self.currency {
            args.push("--currency".to_string());
            args.push(currency.clone());
        }
        // Options spécifiques : on les passe via --config (le fichier complet)
        // rledger extract lit les options depuis le profil dans importers.toml
        args
    }
}

/// Résultat de la résolution du fichier `importers.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionImporters {
    /// Chemin du fichier résolu (peut ne pas exister si aucun trouvé).
    pub chemin: Option<PathBuf>,
    /// Source qui a fourni ce chemin.
    pub source: SourceImporters,
    /// Avertissement si le fichier configuré est introuvable.
    pub avertissement: Option<String>,
}

impl ResolutionImporters {
    /// Vrai si un fichier a été trouvé et lu.
    #[must_use]
    pub fn fichier_trouve(&self) -> bool {
        self.chemin.as_ref().is_some_and(|p| p.exists())
    }
}

/// Échelon de résolution pour `importers.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceImporters {
    /// Fichier `importers.toml` à la racine du ledger (à côté de main.beancount).
    RacineLedger,
    /// Fichier `~/.config/rust-personal-accounting/importers.toml`.
    ConfigLocale,
    /// Aucun fichier trouvé (config vide).
    Aucun,
}

impl SourceImporters {
    #[must_use]
    pub fn libelle(self) -> &'static str {
        match self {
            Self::RacineLedger => "importers.toml à la racine du ledger",
            Self::ConfigLocale => "~/.config/rust-personal-accounting/importers.toml",
            Self::Aucun => "aucun (config vide, moteurs intégrés rledger)",
        }
    }
}

/// Résout le chemin de `importers.toml` selon la chaîne de priorité.
///
/// La résolution dépend du ledger actif : on cherche d'abord à côté du
/// `main.beancount` résolu, puis dans la config locale.
#[must_use]
pub fn resoudre_importers(chemin_ledger: &Path) -> ResolutionImporters {
    // 1. importers.toml à côté du main.beancount du ledger
    if let Some(parent) = chemin_ledger.parent() {
        let candidat = parent.join("importers.toml");
        if candidat.exists() {
            return ResolutionImporters {
                chemin: Some(candidat),
                source: SourceImporters::RacineLedger,
                avertissement: None,
            };
        }
    }

    // 2. ~/.config/rust-personal-accounting/importers.toml
    if let Some(home) = std::env::var_os("HOME") {
        let candidat = PathBuf::from(home)
            .join(".config")
            .join("rust-personal-accounting")
            .join("importers.toml");
        if candidat.exists() {
            return ResolutionImporters {
                chemin: Some(candidat),
                source: SourceImporters::ConfigLocale,
                avertissement: None,
            };
        }
    }

    // 3. Aucun fichier trouvé
    ResolutionImporters {
        chemin: None,
        source: SourceImporters::Aucun,
        avertissement: None,
    }
}

/// Charge la configuration d'importateurs résolue pour un ledger donné.
/// Combine résolution + lecture, avec diagnostic.
#[must_use]
pub fn charger_importers_pour_ledger(
    chemin_ledger: &Path,
) -> (ImportersConfig, ResolutionImporters) {
    let resolution = resoudre_importers(chemin_ledger);
    let config = ImportersConfig::charger(resolution.chemin.as_deref());
    (config, resolution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn creer_fichier_temporaire(contenu: &str) -> PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let chemin =
            std::env::temp_dir().join(format!("rpa-importers-{}-{seq}.toml", std::process::id()));
        fs::write(&chemin, contenu).expect("écriture fichier test");
        chemin
    }

    fn nettoyer(chemin: &Path) {
        let _ = fs::remove_file(chemin);
    }

    #[test]
    fn deserialisation_profil_csv_complet() {
        let toml = r#"
[[importers]]
name = "test-csv"
type = "csv"
account = "Assets:Test"
currency = "EUR"
date_column = "Date"
date_format = "%Y-%m-%d"
delimiter = ";"
"#;
        let config: ImportersConfig = toml::from_str(toml).expect("désérialisation");
        assert_eq!(config.importers.len(), 1);
        let p = &config.importers[0];
        assert_eq!(p.name, "test-csv");
        assert!(p.est_csv());
        assert_eq!(p.account, "Assets:Test");
        assert_eq!(p.currency, Some("EUR".to_string()));
        assert_eq!(p.options.get("date_column"), Some(&"Date".to_string()));
        assert_eq!(p.options.get("delimiter"), Some(&";".to_string()));
    }

    #[test]
    fn deserialisation_profil_ofx() {
        let toml = r#"
[[importers]]
name = "test-ofx"
type = "ofx"
account = "Liabilities:Card"
currency = "EUR"
"#;
        let config: ImportersConfig = toml::from_str(toml).expect("désérialisation");
        let p = &config.importers[0];
        assert!(p.est_ofx());
        assert!(!p.est_csv());
    }

    #[test]
    fn deserialisation_plusieurs_profils() {
        let toml = r#"
[[importers]]
name = "csv1"
type = "csv"
account = "Assets:A"
currency = "EUR"

[[importers]]
name = "ofx1"
type = "ofx"
account = "Liabilities:B"
currency = "USD"
"#;
        let config: ImportersConfig = toml::from_str(toml).expect("désérialisation");
        assert_eq!(config.importers.len(), 2);
        assert!(config.importers[0].est_csv());
        assert!(config.importers[1].est_ofx());
    }

    #[test]
    fn resolution_racine_ledger_prioritaire() {
        let ledger_dir =
            std::env::temp_dir().join(format!("rpa-ledger-test-{}", std::process::id()));
        fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_file = ledger_dir.join("main.beancount");
        fs::write(&ledger_file, "").unwrap();
        let importers_file = ledger_dir.join("importers.toml");
        fs::write(&importers_file, "").unwrap();

        let resolution = resoudre_importers(&ledger_file);
        assert_eq!(resolution.source, SourceImporters::RacineLedger);
        assert_eq!(resolution.chemin, Some(importers_file.clone()));
        assert!(resolution.fichier_trouve());

        let _ = fs::remove_file(&importers_file);
        let _ = fs::remove_file(&ledger_file);
        let _ = fs::remove_dir(&ledger_dir);
    }

    #[test]
    fn resolution_config_locale_si_pas_racine() {
        // On ne peut pas facilement tester le vrai ~/.config, on vérifie
        // que la logique ne panique pas et retourne Aucun si rien n'existe.
        let ledger_file = std::env::temp_dir().join(format!(
            "rpa-ledger-no-importers-{}.beancount",
            std::process::id()
        ));
        fs::write(&ledger_file, "").unwrap();

        let resolution = resoudre_importers(&ledger_file);
        // Soit ConfigLocale (si le fichier existe par hasard), soit Aucun
        assert!(matches!(
            resolution.source,
            SourceImporters::ConfigLocale | SourceImporters::Aucun
        ));

        let _ = fs::remove_file(&ledger_file);
    }

    #[test]
    fn charger_config_vide_si_fichier_absent() {
        let config = ImportersConfig::charger(None);
        assert!(config.importers.is_empty());
    }

    #[test]
    fn charger_config_vide_si_fichier_illlisible() {
        let chemin = creer_fichier_temporaire("=== pas du toml ===");
        let config = ImportersConfig::charger(Some(&chemin));
        assert!(config.importers.is_empty());
        nettoyer(&chemin);
    }

    #[test]
    fn arguments_rledger_de_base() {
        let profil = ImporterProfile {
            name: "mon-csv".to_string(),
            r#type: "csv".to_string(),
            account: "Assets:Test".to_string(),
            currency: Some("EUR".to_string()),
            options: HashMap::new(),
        };
        let args = profil.arguments_rledger();
        assert!(args.contains(&"--profile".to_string()));
        assert!(args.contains(&"mon-csv".to_string()));
        assert!(args.contains(&"--account".to_string()));
        assert!(args.contains(&"Assets:Test".to_string()));
        assert!(args.contains(&"--currency".to_string()));
        assert!(args.contains(&"EUR".to_string()));
    }

    #[test]
    fn arguments_rledger_sans_currency() {
        let profil = ImporterProfile {
            name: "mon-csv".to_string(),
            r#type: "csv".to_string(),
            account: "Assets:Test".to_string(),
            currency: None,
            options: HashMap::new(),
        };
        let args = profil.arguments_rledger();
        assert!(!args.contains(&"--currency".to_string()));
    }
}
