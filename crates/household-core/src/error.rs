//! Erreurs `household-core` (épinglées, explicites, thiserror).

use thiserror::Error;

/// Erreurs de la couche core : chargement, calcul de soldes, écriture.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Fichier/répertoire illisible, écriture impossible, etc.
    #[error("I/O : {0}")]
    Io(#[from] std::io::Error),

    /// Le pipeline complet rustledger a échoué (include manquant, etc.).
    /// Les erreurs de validation **soft** restent dans [`crate::Snapshot::errors`]
    /// ; celle-ci fatale.
    #[error("chargement du ledger : {0}")]
    Load(#[from] rustledger_loader::ProcessError),

    /// Rejeu du booking pour les soldes : la transaction rejouée est invalide.
    #[error("booking : {0}")]
    Booking(#[from] rustledger_booking::BookingError),

    /// Arrondi/précision sous-enterré pendant la présentation des inventaires.
    #[error("présentation inventaires : {0}")]
    Presentation(#[from] rustledger_core::OverflowError),

    /// Directive sans source (directive synthétisée, ex. transaction de pad).
    #[error("directive sans localisation source (index {0})")]
    NoSource(usize),

    /// Erreur d'écriture (span incohérente, erreur vérification post-écriture…).
    #[error("écriture du ledger : {0}")]
    Writer(String),

    /// Date non structurée (chaîne CLI, nom de fichier).
    #[error("date invalide : {0}")]
    Date(String),
}
