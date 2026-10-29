//! Vues de l'application : présentation pure, aucune logique métier.
//!
//! Les données viennent de `crate::viewmodel::UiData` (assemblées dans le
//! worker) ; les vues lisent et dessinent.

pub mod dashboard;
pub mod journal;
pub mod saisie;

use egui::Color32;

/// Couleurs des états de jauge (partagées dashboard/journal).
#[must_use]
pub fn couleur_etat(etat: crate::viewmodel::EtatJauge) -> Color32 {
    match etat {
        crate::viewmodel::EtatJauge::Ok => Color32::from_rgb(0x3f, 0xa7, 0x60),
        crate::viewmodel::EtatJauge::Alerte => Color32::from_rgb(0xe2, 0x93, 0x1b),
        crate::viewmodel::EtatJauge::Depassement => Color32::from_rgb(0xd8, 0x3b, 0x2f),
    }
}
