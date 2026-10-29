//! Thread worker unique : rechargement du ledger hors UI thread.
//!
//! Architecture (cf. TODO jalon 4) :
//! - UI / watcher → `Sender<Command>` ; worker → `Receiver<LoadOutcome>` ;
//! - un seul worker : les commandes sont sérialisées, le debounce écrête les
//!   rafales du watcher (une sauvegarde de fichier = plusieurs événements) ;
//! - le worker appelle `ctx.request_repaint()` à l'envoi du résultat : la GUI
//!   se redessine sans poller plus vite que nécessaire.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use rustledger_core::NaiveDate;

use notify::Watcher as _;

use crate::viewmodel::{self, UiData};

/// Commandes de l'UI / du watcher vers le worker.
#[derive(Debug, Clone, Copy)]
pub enum Command {
    /// Recharger et revalider le ledger complet.
    Reload,
}

/// Réponses du worker vers l'UI.
#[derive(Debug, Clone)]
pub enum LoadOutcome {
    /// Ledger chargé et validé, view-model prêt.
    Pret(std::sync::Arc<UiData>),
    /// Échec de chargement (include manquant, fichier corrompu…) : l'état
    /// précédent reste affiché avec l'erreur.
    Echec(String),
}

/// Délai de « settle » avant rechargement après un événement du watcher :
/// les éditeurs (et nos propres écritures atomiques) génèrent plusieurs
/// événements rapprochés.
const DEBOUNCE: Duration = Duration::from_millis(250);

/// Démarre le worker. Il charge immédiatement puis boucle sur les commandes.
/// Le récepteur des résultats est retourné pour l'UI.
pub fn demarrer(ctx: egui::Context, main: PathBuf) -> (Sender<Command>, Receiver<LoadOutcome>) {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Command>();
    let (out_tx, out_rx) = std::sync::mpsc::channel::<LoadOutcome>();

    thread::spawn(move || {
        // Premier chargement : la GUI a quelque chose à montrer tout de suite.
        out_tx.send(charger(&main)).ok();
        ctx.request_repaint();

        while let Ok(commande) = cmd_rx.recv() {
            match commande {
                Command::Reload => {
                    // Debounce : écrête la rafale d'événements en attente.
                    while cmd_rx.try_recv().is_ok() {}
                    thread::sleep(DEBOUNCE);
                    while cmd_rx.try_recv().is_ok() {}

                    out_tx.send(charger(&main)).ok();
                    ctx.request_repaint();
                }
            }
        }
        // Canal des commandes fermé (fermeture de l'app) : arrêt.
    });

    (cmd_tx, out_rx)
}

fn charger(main: &Path) -> LoadOutcome {
    let aujourdhui = aujourd_hui();
    let snapshot = household_core::Snapshot::load(main);
    match snapshot {
        Ok(snapshot) => match viewmodel::build(&snapshot, aujourdhui) {
            Ok(data) => LoadOutcome::Pret(std::sync::Arc::new(data)),
            Err(e) => LoadOutcome::Echec(e),
        },
        Err(e) => LoadOutcome::Echec(e.to_string()),
    }
}

/// Surveille le répertoire du ledger (récursif) et demande un rechargement à
/// chaque événement (le worker débonce). Le watcher retourné doit rester
/// vivant : le détruire arrête la surveillance.
///
/// # Erreurs
/// Panique si le répertoire du ledger n'existe pas ou est inobservable :
/// sans watcher, la GUI doit s'arrêter tôt plutôt que se croire à jour.
pub fn demarrer_watcher(main: &Path, cmd_tx: Sender<Command>) -> notify::RecommendedWatcher {
    let dossier = main
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut watcher =
        notify::recommended_watcher(move |resultat: Result<notify::Event, notify::Error>| {
            if resultat.is_ok() {
                // Toute modification du ledger → rechargement (déboncé côté worker).
                cmd_tx.send(Command::Reload).ok();
            }
        })
        .expect("watcher du ledger");
    watcher
        .watch(&dossier, notify::RecursiveMode::Recursive)
        .expect("surveillance du répertoire ledger");
    watcher
}

/// Date du jour (injectée dans le view-model pour la testabilité).
#[must_use]
pub fn aujourd_hui() -> NaiveDate {
    jiff::Zoned::now().date()
}
