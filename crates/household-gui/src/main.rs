//! Point d'entrée de l'application « Comptabilité du foyer » (TUI ratatui).
//!
//! Usage : `compta-foyer [chemin/vers/main.beancount]` — sinon, chaîne de
//! résolution standard (variable `RPA_LEDGER`, puis champ `chemin_ledger` du
//! fichier de préférences `~/.config/rust-personal-accounting.toml`, puis le
//! ledger d'exemple du dépôt avec badge « données fictives »).
//! Pour du test : `COMPTA_ECRAN=journal|saisie|budget|rapports|graphiques|git`
//! ouvre l'écran correspondant au démarrage (sans le tableau de bord),
//! `COMPTA_ECRAN=aucun` n'ouvre aucun écran — défaut : tableau de bord.
//! `COMPTA_FENETRE_LEDGER=1` ouvre le panneau « Ledger actif » au démarrage.

use std::error::Error;
use std::io;
use std::path::Path;

use crossterm::{
    ExecutableCommand,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use household_core::{VAR_ENV, resoudre_ledger};
use ratatui::{Terminal, backend::CrosstermBackend};

use household_gui::app::{App, Onglet};
use household_gui::config::Config;

fn run() -> Result<(), Box<dyn Error>> {
    // Chaîne de résolution partagée (household-core::resolution) : jamais de
    // chemin codé en dur dans la GUI.
    let config = Config::charger();
    let resolu = resoudre_ledger(
        std::env::args().nth(1).as_deref().map(Path::new),
        std::env::var(VAR_ENV).ok().as_deref(),
        config.chemin_ledger.as_deref(),
    );
    if let Some(avertissement) = &resolu.avertissement {
        eprintln!("⚠ {avertissement}");
    }

    // Écran alternatif + mode raw.
    io::stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;
    io::stdout().execute(EnableMouseCapture)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let ecran_env = std::env::var("COMPTA_ECRAN").ok();
    let ecran_demande = ecran_env.as_deref().map(str::trim);
    let onglet_initial = match ecran_demande {
        Some("aucun") => None,
        Some(v) => Onglet::depuis_env(Some(v)),
        None => Some(Onglet::TableauDeBord),
    };

    let params_au_demarrage =
        std::env::var("COMPTA_FENETRE_LEDGER").is_ok_and(|v| !v.is_empty() && v != "0");

    let mut app = App::new(resolu, onglet_initial, params_au_demarrage);

    let resultat = app.run(&mut terminal);

    // Restauration du terminal.
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(resultat?)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Erreur fatale : {e}");
        std::process::exit(1);
    }
}
