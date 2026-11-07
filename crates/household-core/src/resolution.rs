//! Résolution du ledger actif : chaîne unique partagée par la GUI, la CLI
//! et tout futur frontal.
//!
//! Ordre de priorité (le premier chemin *configuré* gagne, qu'il existe ou
//! non) :
//!
//! 1. argument explicite (`compta-foyer <chemin>`, `household-cli <chemin>`) ;
//! 2. variable d'environnement [`VAR_ENV`] (`RPA_LEDGER`) ;
//! 3. champ `chemin_ledger` de `~/.config/rust-personal-accounting.toml`
//!    (lu par la couche appelante — ici, la config de la GUI) ;
//! 4. repli : ledger d'exemple du dépôt ([`EXEMPLE_RELATIF`], mode
//!    [`ModeLedger::Exemple`] — l'interface affiche alors le badge
//!    « données fictives »).
//!
//! Garde-fous (cf. audit RGPD du 2026-11-07) :
//!
//! - un chemin configuré mais **introuvable** n'est jamais remplacé
//!   silencieusement par l'exemple : on renvoie ce chemin (le chargement
//!   échouera avec bannière) et un [`ResolutionLedger::avertissement`]. Sinon,
//!   on risquerait d'éditer les données fictives en croyant travailler sur le
//!   vrai foyer ;
//! - le vrai ledger vit **hors du dépôt de code** : le répertoire `examples/`
//!   du dépôt ne contient que des données fictives.

use std::fmt;
use std::path::{Path, PathBuf};

/// Variable d'environnement désignant le ledger actif.
pub const VAR_ENV: &str = "RPA_LEDGER";

/// Chemin du ledger d'exemple (recette), relatif à la racine du dépôt —
/// résolu relativement au répertoire courant (convention : lancer depuis la
/// racine du dépôt, ou passer un chemin explicite).
pub const EXEMPLE_RELATIF: &str = "examples/ledger/main.beancount";

/// Échelon qui a déterminé le chemin résolu (traçabilité, affichages et
/// réglages de l'interface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceResolution {
    /// Montrer explicitement sur la ligne de commande.
    Argument,
    /// Variable d'environnement [`VAR_ENV`].
    VariableEnv,
    /// Champ `chemin_ledger` de la config locale (fichier de préférences).
    ChampConfig,
    /// Repli automatique : ledger d'exemple du dépôt (données fictives).
    RepliExemple,
}

impl SourceResolution {
    /// Libellé humain de l'échelon.
    #[must_use]
    pub fn libelle(self) -> &'static str {
        match self {
            Self::Argument => "argument de la ligne de commande",
            Self::VariableEnv => "variable `RPA_LEDGER`",
            Self::ChampConfig => "champ `chemin_ledger` de la config locale",
            Self::RepliExemple => "repli : ledger d'exemple du dépôt",
        }
    }

    /// Vrai si l'échelon gagnant surcharge le champ de config (argument ou
    /// variable `RPA_LEDGER`) : la GUI désactive alors ses réglages
    /// persistés tant que cette source est en place.
    #[must_use]
    pub fn surcharge_config(self) -> bool {
        matches!(self, Self::Argument | Self::VariableEnv)
    }
}

/// Nature du ledger résolu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeLedger {
    /// Ledger réel du foyer (argument, [`VAR_ENV`] ou config). Peut être
    /// momentanément introuvable : voir [`ResolutionLedger::avertissement`].
    Reel,
    /// Ledger d'exemple du dépôt (données fictives) — repli sans
    /// configuration.
    Exemple,
}

impl ModeLedger {
    /// Vrai si le chemin pointe sur l'exemple (badge « données fictives »).
    #[must_use]
    pub fn est_exemple(self) -> bool {
        self == Self::Exemple
    }
}

impl fmt::Display for ModeLedger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Reel => "ledger du foyer",
            Self::Exemple => "ledger d'exemple (données fictives)",
        })
    }
}

/// Résultat de la résolution : chemin, mode, source et avertissement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionLedger {
    /// Chemin du `main.beancount` à charger.
    pub chemin: PathBuf,
    /// Nature du ledger auquel ce chemin correspond.
    pub mode: ModeLedger,
    /// Échelon gagnant de la chaîne de résolution.
    pub source: SourceResolution,
    /// Non vide si la résolution a un problème à signaler à l'utilisateur
    /// (chemin configuré introuvable, exemple absent…). Ne remplace pas
    /// l'erreur de chargement : le lecteur affichera la sienne en bannière.
    pub avertissement: Option<String>,
}

impl fmt::Display for ResolutionLedger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({})",
            self.chemin.display(),
            match &self.avertissement {
                Some(avertissement) => format!("{} — {avertissement}", self.mode),
                None => self.mode.to_string(),
            }
        )
    }
}

/// Résout le ledger à charger (infaillible : renvoie toujours un chemin à
/// tenter ; les problèmes sont portés par le champ `avertissement`).
#[must_use]
pub fn resoudre_ledger(
    argument: Option<&Path>,
    var_env: Option<&str>,
    champ_config: Option<&Path>,
) -> ResolutionLedger {
    // 1..=3 : chemins explicites en ordre de priorité ; le premier non vide
    // gagne, qu'il existe ou non (jamais de repli silencieux sur l'exemple).
    let explicites = [
        (argument.map(Path::to_path_buf), SourceResolution::Argument),
        (
            var_env_nettoyee(var_env).map(PathBuf::from),
            SourceResolution::VariableEnv,
        ),
        (
            champ_config.map(Path::to_path_buf),
            SourceResolution::ChampConfig,
        ),
    ];
    for (chemin_opt, source) in explicites {
        let Some(chemin) = chemin_opt else { continue };
        if chemin.exists() {
            return ResolutionLedger {
                chemin,
                mode: ModeLedger::Reel,
                source,
                avertissement: None,
            };
        }
        return ResolutionLedger {
            avertissement: Some(format!(
                "configuré via {} mais introuvable — pas de repli sur les données fictives",
                source.libelle()
            )),
            chemin,
            mode: ModeLedger::Reel,
            source,
        };
    }

    // 4 : repli sur l'exemple du dépôt.
    let exemple = PathBuf::from(EXEMPLE_RELATIF);
    let avertissement = if exemple.exists() {
        None
    } else {
        Some(format!(
            "ni `{VAR_ENV}`, ni `chemin_ledger`, et l'exemple `{EXEMPLE_RELATIF}` est introuvable depuis ici"
        ))
    };
    ResolutionLedger {
        chemin: exemple,
        mode: ModeLedger::Exemple,
        source: SourceResolution::RepliExemple,
        avertissement,
    }
}

/// La variable d'environnement est ignorée si absente ou vide/espaces.
fn var_env_nettoyee(var: Option<&str>) -> Option<String> {
    match var.map(str::trim) {
        None | Some("") => None,
        Some(non_vide) => Some(non_vide.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fichier temporaire pour les tests d'existence (nettoyé en fin de test).
    fn temporaire(nom: &str) -> PathBuf {
        let chemin = std::env::temp_dir().join(format!(
            "rpa-resolution-{nom}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ));
        std::fs::write(&chemin, "").expect("écriture du fichier test");
        chemin
    }

    fn nettoyer(chemin: &Path) {
        let _ = std::fs::remove_file(chemin);
    }

    #[test]
    fn argument_passe_avant_tout() {
        let env_chemin = temporaire("env");
        let config_chemin = temporaire("config");
        let resolu = resoudre_ledger(
            Some(env_chemin.as_path()),
            Some(env_chemin.to_str().unwrap()),
            Some(&config_chemin),
        );
        assert_eq!(resolu.mode, ModeLedger::Reel);
        assert_eq!(resolu.source, SourceResolution::Argument);
        assert_eq!(resolu.chemin, env_chemin);
        assert_eq!(resolu.avertissement, None);
        nettoyer(&env_chemin);
        nettoyer(&config_chemin);
    }

    #[test]
    fn variable_env_passe_avant_config() {
        let env_chemin = temporaire("env");
        let config_chemin = temporaire("config");
        let resolu = resoudre_ledger(None, env_chemin.to_str(), Some(&config_chemin));
        assert_eq!(resolu.chemin, env_chemin);
        assert_eq!(resolu.mode, ModeLedger::Reel);
        assert_eq!(resolu.source, SourceResolution::VariableEnv);
        assert_eq!(resolu.avertissement, None);
        nettoyer(&env_chemin);
        nettoyer(&config_chemin);
    }

    #[test]
    fn config_utilisee_en_referentiel() {
        let config_chemin = temporaire("config");
        let resolu = resoudre_ledger(None, None, Some(&config_chemin));
        assert_eq!(resolu.chemin, config_chemin);
        assert_eq!(resolu.mode, ModeLedger::Reel);
        assert_eq!(resolu.source, SourceResolution::ChampConfig);
        assert_eq!(resolu.avertissement, None);
        nettoyer(&config_chemin);
    }

    #[test]
    fn variable_env_vide_ignoree() {
        let config_chemin = temporaire("config");
        for vide in ["", "   "] {
            let resolu = resoudre_ledger(None, Some(vide), Some(&config_chemin));
            assert_eq!(resolu.chemin, config_chemin, "var env {vide:?} ignorée");
        }
        nettoyer(&config_chemin);
    }

    #[test]
    fn chemin_configure_absent_ne_replie_pas_sur_lexemple() {
        let resolu = resoudre_ledger(None, Some("/non/existant/main.beancount"), None);
        assert_eq!(resolu.mode, ModeLedger::Reel);
        assert!(resolu.chemin.ends_with("main.beancount"));
        let avertissement = resolu.avertissement.as_ref().expect("avertissement requis");
        assert!(avertissement.contains("pas de repli"), "{avertissement}");
    }

    #[test]
    fn sans_configuration_repli_exemple() {
        let resolu = resoudre_ledger(None, None, None);
        assert_eq!(resolu.mode, ModeLedger::Exemple);
        assert_eq!(resolu.chemin, PathBuf::from(EXEMPLE_RELATIF));
        assert_eq!(resolu.source, SourceResolution::RepliExemple);
        assert!(resolu.mode.est_exemple());
    }

    #[test]
    fn mode_exemple_detecte_le_badge() {
        assert!(!ModeLedger::Reel.est_exemple());
        assert!(ModeLedger::Exemple.est_exemple());
    }

    #[test]
    fn surcharge_de_la_config_uniquement_par_arg_ou_env() {
        assert!(SourceResolution::Argument.surcharge_config());
        assert!(SourceResolution::VariableEnv.surcharge_config());
        assert!(!SourceResolution::ChampConfig.surcharge_config());
        assert!(!SourceResolution::RepliExemple.surcharge_config());
    }
}
