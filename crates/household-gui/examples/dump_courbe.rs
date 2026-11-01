//! Diagnostic : affiche la courbe de patrimoine telle que construite par le
//! view-model (les données que la vue transforme en points egui_plot).
//! Lancer : cargo run -p household-gui --example dump_courbe

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let chemin = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("ledger/main.beancount"));
    let snapshot = household_core::Snapshot::load(&chemin).expect("snapshot");
    let courbe = household_gui::viewmodel::build(&snapshot, household_gui::worker::aujourd_hui())
        .expect("ui-data")
        .tableau_de_bord
        .courbe;
    for (mois, valeur) in &courbe {
        println!("{mois} => {valeur}");
    }
}
