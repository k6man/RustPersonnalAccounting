//! Point d'entrée de l'application « Comptabilité du foyer ».
//!
//! Usage : `compta-foyer [chemin/vers/main.beancount]` — sinon, chaîne de
//! résolution standard (variable `RPA_LEDGER`, puis champ `chemin_ledger` du
//! fichier de préférences `~/.config/rust-personal-accounting.toml`, puis le
//! ledger d'exemple du dépôt avec badge « données fictives »).
//! Pour du test : `COMPTA_ECRAN=journal|saisie|budget|rapports|graphiques|git`
//! ouvre la fenêtre correspondante au démarrage (sans le tableau de bord) —
//! défaut : tableau de bord quasi plein écran.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use household_core::{VAR_ENV, resoudre_ledger};

fn main() -> eframe::Result {
    // Chaîne de résolution partagée (household-core::resolution) : jamais de
    // chemin codé en dur dans la GUI.
    let config = household_gui::config::Config::charger();
    let resolu = resoudre_ledger(
        std::env::args().nth(1).as_deref().map(std::path::Path::new),
        std::env::var(VAR_ENV).ok().as_deref(),
        config.chemin_ledger.as_deref(),
    );
    if let Some(avertissement) = &resolu.avertissement {
        // Rendu aussi en bannière par le lecteur si le chargement échoue.
        eprintln!("⚠ {avertissement}");
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1300.0, 800.0])
            .with_min_inner_size([1060.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Comptabilité du foyer",
        options,
        Box::new(move |cc| Ok(Box::new(household_gui::app::FoyerApp::new(cc, resolu)))),
    )
}
