//! Application « Comptabilité du foyer » — squelette du jalon 1.
//!
//! API eframe/egui 0.37 : le trait `eframe::App` exige `logic` (optionnel)
//! puis `ui`, à qui l'on remet un `Ui` déjà monté (voir docs epi).
//! L'architecture complète (état applicatif / commandes / vues, worker de
//! rechargement, watcher du ledger) arrive au jalon 4 ; cette fenêtre ne sert
//! qu'à valider la toolchain GUI et servir de point d'amarrage.

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Comptabilité du foyer",
        options,
        Box::new(|_cc| Ok(Box::new(Squelette))),
    )
}

struct Squelette;

impl eframe::App for Squelette {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Comptabilité du foyer — Beancount · rustledger");
            ui.separator();
            ui.label(
                "Squelette (jalon 1). Le dashboard, le journal, les budgets et la \
                 saisie arrivent aux jalons 4 à 6.",
            );
        });
    }
}
