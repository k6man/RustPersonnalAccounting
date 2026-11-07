//! Préférences GUI persistées **hors ledger** (TODO.md Q6) : un fichier TOML
//! local — `~/.config/rust-personal-accounting.toml` — ne contient que des
//! réglages applicatifs (chemin du vrai ledger, rollover par famille…).
//! Le ledger reste la seule source de vérité des données.
//!
//! Migration (2026-11-07) : l'ancien `~/.config/compta-foyer.toml` (v0.1) est
//! lu en repli si le nouveau n'existe pas ; son contenu est re-persisté dans
//! le nouveau fichier à la première sauvegarde. L'ancien fichier n'est
//! jamais supprimé automatiquement.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Préférences de l'application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Chemin du `main.beancount` du vrai ledger (dernier échelon actif de
    /// la chaîne de résolution : argument > `RPA_LEDGER` > ce champ >
    /// exemple du dépôt). Absent = aucun vrai ledger configuré.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chemin_ledger: Option<PathBuf>,
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
                .join("rust-personal-accounting.toml")
        })
    }

    /// Ancien emplacement (v0.1) : lu uniquement si le nouveau est absent
    /// (migration douce, jamais supprimé automatiquement).
    #[must_use]
    pub fn ancien_chemin() -> Option<PathBuf> {
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
        Self::charger_avec(Self::chemin().as_deref(), Self::ancien_chemin().as_deref())
    }

    /// Variante testable : nouveau fichier et ancien (migration) injectés.
    /// Le nouveau est retenu comme chemin d'enregistrement ; l'ancien n'est
    /// lu que si le nouveau n'existe pas, et un chargement via l'ancien
    /// re-pointe l'enregistrement vers le nouveau (migration au premier
    /// `sauver`).
    #[must_use]
    pub fn charger_avec(
        nouveau: Option<&std::path::Path>,
        ancien: Option<&std::path::Path>,
    ) -> Self {
        if let Some(chemin) = nouveau
            && chemin.exists()
        {
            return Self::charger_de(Some(chemin));
        }
        if let Some(ancien) = ancien
            && ancien.exists()
        {
            // Migration : contenu de l'ancien, mais enregistrement vers
            // le nouveau (l'ancien n'est jamais supprimé ni réécrit).
            let mut config = Self::charger_de(Some(ancien));
            config.chemin_charge = nouveau.map(PathBuf::from);
            return config;
        }
        Self::charger_de(nouveau)
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
            "rpa-config-{nom}-{}-{:?}.toml",
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

    #[test]
    fn chemin_ledger_round_trip() {
        let config = Config {
            chemin_ledger: Some(PathBuf::from("/Users/foyer/LedgerFoyer/main.beancount")),
            ..Config::default()
        };

        let chemin = chemin_temporaire("vrailedger");
        config.sauver_vers(Some(&chemin)).expect("sauvegarde");
        let relu = Config::charger_de(Some(&chemin));
        assert_eq!(relu.chemin_ledger, config.chemin_ledger);
        let _ = std::fs::remove_file(&chemin);
    }

    #[test]
    fn migration_lit_ancien_fichier_et_donne_le_nouveau_comme_cible() {
        // Nouveau absent, ancien présent : contenu repris, enregistrement migré.
        let nouveau = chemin_temporaire("nouveau"); // volontairement non écrit
        let ancien = chemin_temporaire("ancien");
        std::fs::write(&ancien, "rollover = { \"Expenses:Enfants\" = true }").unwrap();

        let config = Config::charger_avec(Some(&nouveau), Some(&ancien));
        assert!(config.rollover_actif("Expenses:Enfants"));
        assert_eq!(config.chemin_charge, Some(nouveau.clone()));
        assert!(config.chemin_ledger.is_none());
        // L'ancien n'est jamais supprimé ni réécrit automatiquement.
        assert!(ancien.exists());
        let _ = std::fs::remove_file(&nouveau);
        let _ = std::fs::remove_file(&ancien);
    }

    #[test]
    fn migration_nouveau_prioritaire_sil_existe() {
        // Les deux existent : le nouveau gagne, l'ancien n'est même pas lu.
        let nouveau = chemin_temporaire("nouveau-existe");
        let ancien = chemin_temporaire("ancien-existe");
        std::fs::write(&nouveau, "rollover = { \"Assets:Epargne\" = true }").unwrap();
        std::fs::write(&ancien, "rollover = { \"Expenses:Enfants\" = true }").unwrap();

        let config = Config::charger_avec(Some(&nouveau), Some(&ancien));
        assert!(config.rollover_actif("Assets:Epargne"));
        assert!(!config.rollover_actif("Expenses:Enfants"));
        assert_eq!(config.chemin_charge, Some(nouveau.clone()));
        let _ = std::fs::remove_file(&nouveau);
        let _ = std::fs::remove_file(&ancien);
    }

    #[test]
    fn contenu_migre_re_persiste_dans_le_nouveau() {
        let nouveau = chemin_temporaire("nouveau-sauvegarde");
        let ancien = chemin_temporaire("ancien-sauvegarde");
        std::fs::write(
            &ancien,
            "rollover = { \"Assets:Epargne\" = true }\nchemin_ledger = \"/vrai/main.beancount\"\n",
        )
        .unwrap();

        let mut config = Config::charger_avec(Some(&nouveau), Some(&ancien));
        assert_eq!(
            config
                .chemin_ledger
                .clone()
                .map(|p| p.to_string_lossy().into_owned()),
            Some("/vrai/main.beancount".to_string())
        );
        config
            .basculer_rollover("Assets:Epargne")
            .expect("bascule déclenche la sauvegarde");
        let texte = std::fs::read_to_string(&nouveau).expect("nouveau fichier créé");
        assert!(texte.contains("/vrai/main.beancount"), "{texte}");
        let _ = std::fs::remove_file(&nouveau);
        let _ = std::fs::remove_file(&ancien);
    }
}
