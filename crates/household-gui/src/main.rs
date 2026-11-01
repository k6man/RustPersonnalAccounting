//! Point d'entrée de l'application « Comptabilité du foyer ».
//!
//! Usage : `compta-foyer [chemin/vers/main.beancount]` (défaut :
//! `ledger/main.beancount` relatif au répertoire courant).
//! Pour du test : `COMPTA_ECRAN=journal|saisie|budget|rapports|git` ouvre la
//! fenêtre correspondante au démarrage (sans le tableau de bord) — défaut :
//! tableau de bord quasi plein écran.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() -> eframe::Result {
    let main = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ledger/main.beancount"));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1300.0, 800.0])
            .with_min_inner_size([1060.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Comptabilité du foyer",
        options,
        Box::new(move |cc| Ok(Box::new(household_gui::app::FoyerApp::new(cc, main)))),
    )
}
