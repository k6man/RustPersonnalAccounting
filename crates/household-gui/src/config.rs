//! Préférences GUI persistées **hors ledger** (TODO.md Q6) : un fichier TOML
//! local — `~/.config/compta-foyer.toml` — ne contient que des réglages
//! d'interface (toggles rollover par famille…). Le ledger reste la seule
//! source de vérité des données.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Préférences de l'application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Rollover activé par famille de postes (clé = compte de tête, ex.
    /// `Expenses:Enfants`). Absent = désactivé.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rollover: BTreeMap<String, bool>,
    /// Chemin d'où cette config a été chargée (non sérialisé) : les
    /// modifications sont re-persistées à cet endroit.
    #[serde(skip)]
    pub chemin_charge: Option<PathBuf>,
}

impl Config {
    /// Chemin du fichier de préférences ; `None` si `$HOME` est absent
    /// (sandbox) — la config reste alors en mémoire, sans persistance.
    #[must_use]
    pub fn chemin() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("compta-foyer.toml")
        })
    }

    /// Charge les préférences ; fichier absent ou illisible → défauts
    /// (l'interface doit toujours démarrer, jamais planter sur une config).
    #[must_use]
    pub fn charger() -> Self {
        Self::charger_de(Self::chemin().as_deref())
    }

    /// Variante testable (chemin injecté) : retenu comme chemin d'enregistrement.
    #[must_use]
    pub fn charger_de(chemin: Option<&std::path::Path>) -> Self {
        let Some(chemin) = chemin else {
            return Self::default();
        };
        let mut config = match std::fs::read_to_string(chemin) {
            Ok(texte) => toml::from_str(&texte).unwrap_or_default(),
            Err(_) => Self::default(),
        };
        config.chemin_charge = Some(chemin.to_path_buf());
        config
    }

    /// Persiste les préférences (crée `~/.config/` au besoin). Erreur
    /// retournée pour l'affichage (flash) — jamais de panique.
    pub fn sauver(&self) -> Result<(), String> {
        self.sauver_vers(self.chemin_charge.as_deref().or(Self::chemin().as_deref()))
    }

    /// Variante testable (chemin injecté).
    pub fn sauver_vers(&self, chemin: Option<&std::path::Path>) -> Result<(), String> {
        let Some(chemin) = chemin else {
            return Err("aucun répertoire personnel ($HOME) : préférences non \
                        persistées"
                .to_string());
        };
        let texte = toml::to_string_pretty(self)
            .map_err(|e| format!("sérialisation des préférences : {e}"))?;
        if let Some(parent) = chemin.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("création de {} : {e}", parent.display()))?;
        }
        std::fs::write(chemin, texte).map_err(|e| format!("écriture de {} : {e}", chemin.display()))
    }

    /// Bascule le rollover d'une famille et persiste.
    pub fn basculer_rollover(&mut self, famille: &str) -> Result<(), String> {
        let actuel = self.rollover.get(famille).copied().unwrap_or(false);
        self.rollover.insert(famille.to_string(), !actuel);
        self.sauver()
    }

    /// Rollover actif pour une famille ?
    #[must_use]
    pub fn rollover_actif(&self, famille: &str) -> bool {
        self.rollover.get(famille).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chemin_temporaire(nom: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "compta-config-{nom}-{}-{:?}.toml",
            std::process::id(),
            std::time::SystemTime::now()
        ))
    }

    #[test]
    fn round_trip_toml() {
        let mut config = Config::default();
        config.rollover.insert("Expenses:Enfants".to_string(), true);
        config.rollover.insert("Assets:Epargne".to_string(), true);

        let chemin = chemin_temporaire("test");
        config.sauver_vers(Some(&chemin)).expect("sauvegarde");
        let relu = Config::charger_de(Some(&chemin));
        assert_eq!(relu.rollover, config.rollover);
        assert!(relu.rollover_actif("Expenses:Enfants"));
        assert!(!relu.rollover_actif("Expenses:Transport"));
        let _ = std::fs::remove_file(&chemin);
    }

    #[test]
    fn fichier_absent_ou_corrompu_defauts() {
        assert!(Config::charger_de(None).rollover.is_empty());
        let chemin = chemin_temporaire("casse");
        std::fs::write(&chemin, "=== pas du toml ===").unwrap();
        assert!(Config::charger_de(Some(&chemin)).rollover.is_empty());
        let _ = std::fs::remove_file(&chemin);
    }

    #[test]
    fn bascule_persiste_au_chemin_charge() {
        let chemin = chemin_temporaire("bascule");
        let mut config = Config::charger_de(Some(&chemin));
        config.basculer_rollover("Assets:Epargne").expect("bascule");
        assert!(Config::charger_de(Some(&chemin)).rollover_actif("Assets:Epargne"));
        config.basculer_rollover("Assets:Epargne").expect("bascule");
        assert!(!Config::charger_de(Some(&chemin)).rollover_actif("Assets:Epargne"));
        let _ = std::fs::remove_file(&chemin);
    }
}
