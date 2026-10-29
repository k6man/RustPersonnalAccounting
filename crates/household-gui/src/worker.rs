//! Thread worker unique : rechargement du ledger hors UI thread **et**
//! écritures (jalon 5) via les garanties du writer de `household-core`
//! (pré-vérification sur copie scratch, atomicité, revalidation).
//!
//! Architecture :
//! - UI / watcher → `Sender<Command>` ; worker → `Receiver<LoadOutcome>` ;
//! - un seul worker : les commandes (rechargements et écritures) sont
//!   sérialisées, le debounce écrête les rafales du watcher ;
//! - le worker appelle `ctx.request_repaint()` à l'envoi de chaque résultat.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use rustledger_core::Directive;
use rustledger_core::NaiveDate;

use crate::saisie::{Brouillon, Empreinte};
use crate::viewmodel::{self, UiData};

/// Commandes de l'UI / du watcher vers le worker.
#[derive(Debug)]
pub enum Command {
    /// Recharger et revalider le ledger complet.
    Reload,
    /// Écrire une nouvelle transaction (fichier mensuel de sa date).
    Ecrire(Brouillon),
    /// Remplacer une transaction existante (même date : span exact ;
    /// autre date : suppression + réinsertion triée).
    Modifier {
        index: usize,
        empreinte: Empreinte,
        brouillon: Brouillon,
    },
    /// Supprimer une transaction existante.
    Supprimer { index: usize, empreinte: Empreinte },
    /// Éditeur de budget : modifier la cellule (1ᵉʳ jour du mois, compte) —
    /// remplacement chirurgical ou insertion de supersession.
    CelluleBudget {
        annee: i16,
        date: NaiveDate,
        compte: String,
        montant: rust_decimal::Decimal,
    },
    /// Éditeur de budget : générer l'année cible par duplication de la
    /// source (pour cent ±, arrondi au pas).
    GenererAnnee {
        source: i16,
        cible: i16,
        pourcent: rust_decimal::Decimal,
        pas: rust_decimal::Decimal,
    },
}

/// Réponses du worker vers l'UI.
#[derive(Debug, Clone)]
pub enum LoadOutcome {
    /// Ledger chargé et validé, view-model prêt.
    Pret(Arc<UiData>),
    /// Échec de chargement (include manquant, fichier corrompu…) : l'état
    /// précédent reste affiché avec l'erreur.
    Echec(String),
    /// Écriture réussie → données fraîches + message (fichier cible).
    Ecrit {
        donnees: Arc<UiData>,
        message: String,
    },
    /// Écriture refusée (validation locale ou pré-vérification) : le ledger
    /// est inchangé, un message lisible accompagne le refus.
    Refuse(String),
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
                Command::Ecrire(brouillon) => {
                    out_tx.send(ecrire(&main, &brouillon)).ok();
                    ctx.request_repaint();
                }
                Command::Modifier {
                    index,
                    empreinte,
                    brouillon,
                } => {
                    out_tx
                        .send(modifier(&main, index, &empreinte, &brouillon))
                        .ok();
                    ctx.request_repaint();
                }
                Command::Supprimer { index, empreinte } => {
                    out_tx.send(supprimer(&main, index, &empreinte)).ok();
                    ctx.request_repaint();
                }
                Command::CelluleBudget {
                    annee,
                    date,
                    compte,
                    montant,
                } => {
                    out_tx
                        .send(cellule_budget(&main, annee, date, &compte, montant))
                        .ok();
                    ctx.request_repaint();
                }
                Command::GenererAnnee {
                    source,
                    cible,
                    pourcent,
                    pas,
                } => {
                    out_tx
                        .send(generer_annee(&main, source, cible, pourcent, pas))
                        .ok();
                    ctx.request_repaint();
                }
            }
        }
        // Canal des commandes fermé (fermeture de l'app) : arrêt.
    });

    (cmd_tx, out_rx)
}

fn charger(main: &Path) -> LoadOutcome {
    let snapshot = household_core::Snapshot::load(main);
    match snapshot {
        Ok(snapshot) => match viewmodel::build(&snapshot, aujourd_hui()) {
            Ok(data) => LoadOutcome::Pret(Arc::new(data)),
            Err(e) => LoadOutcome::Echec(e),
        },
        Err(e) => LoadOutcome::Echec(e.to_string()),
    }
}

fn ecrire(main: &Path, brouillon: &Brouillon) -> LoadOutcome {
    let transaction = match brouillon.vers_transaction() {
        Ok(transaction) => transaction,
        Err(erreurs) => return LoadOutcome::Refuse(erreurs.join(" · ")),
    };
    let date = transaction.date;
    let snapshot = match household_core::Snapshot::load(main) {
        Ok(snapshot) => snapshot,
        Err(e) => return LoadOutcome::Echec(e.to_string()),
    };
    match household_core::append_transaction(&snapshot, &transaction) {
        Ok(nouveau) => match donnees_fraiches(&nouveau) {
            Ok(donnees) => LoadOutcome::Ecrit {
                donnees,
                message: format!(
                    "écrite dans {}",
                    household_core::monthly_file(main, date).display()
                ),
            },
            Err(e) => LoadOutcome::Echec(e),
        },
        Err(e) => LoadOutcome::Refuse(format!("écriture refusée : {e}")),
    }
}

fn modifier(
    main: &Path,
    index: usize,
    empreinte: &Empreinte,
    brouillon: &Brouillon,
) -> LoadOutcome {
    let transaction = match brouillon.vers_transaction() {
        Ok(transaction) => transaction,
        Err(erreurs) => return LoadOutcome::Refuse(erreurs.join(" · ")),
    };
    let snapshot = match household_core::Snapshot::load(main) {
        Ok(snapshot) => snapshot,
        Err(e) => return LoadOutcome::Echec(e.to_string()),
    };
    // L'index vient d'un snapshot antérieur : relocalisation par empreinte
    // (date + tiers + libellé), sinon refus — jamais la mauvaise ligne.
    let index_actuel = match localiser(&snapshot, index, empreinte) {
        Ok(index) => index,
        Err(e) => return LoadOutcome::Refuse(e),
    };
    match household_core::replace_transaction(&snapshot, index_actuel, &transaction) {
        Ok(nouveau) => match donnees_fraiches(&nouveau) {
            Ok(donnees) => LoadOutcome::Ecrit {
                donnees,
                message: format!(
                    "modifiée dans {}",
                    household_core::monthly_file(main, transaction.date).display()
                ),
            },
            Err(e) => LoadOutcome::Echec(e),
        },
        Err(e) => LoadOutcome::Refuse(format!("modification refusée : {e}")),
    }
}

fn supprimer(main: &Path, index: usize, empreinte: &Empreinte) -> LoadOutcome {
    let snapshot = match household_core::Snapshot::load(main) {
        Ok(snapshot) => snapshot,
        Err(e) => return LoadOutcome::Echec(e.to_string()),
    };
    let index_actuel = match localiser(&snapshot, index, empreinte) {
        Ok(index) => index,
        Err(e) => return LoadOutcome::Refuse(e),
    };
    match household_core::remove_transaction(&snapshot, index_actuel) {
        Ok(nouveau) => match donnees_fraiches(&nouveau) {
            Ok(donnees) => LoadOutcome::Ecrit {
                donnees,
                message: "transaction supprimée".to_string(),
            },
            Err(e) => LoadOutcome::Echec(e),
        },
        Err(e) => LoadOutcome::Refuse(format!("suppression refusée : {e}")),
    }
}

/// Relocalise une transaction après un rechargement : l'index d'origine est
/// accepté s'il pointe toujours sur la même empreinte (date + tiers +
/// libellé) ; sinon recherche dans tout le ledger ; sinon erreur lisible.
pub fn localiser(
    snapshot: &household_core::Snapshot,
    index: usize,
    empreinte: &Empreinte,
) -> Result<usize, String> {
    let correspond = |directive: &Directive| match directive {
        Directive::Transaction(t) => {
            t.date == empreinte.date
                && t.payee.as_ref().map_or(empreinte.tiers.is_none(), |p| {
                    Some(p.as_str().to_string()) == empreinte.tiers
                })
                && t.narration.as_str() == empreinte.libelle
        }
        _ => false,
    };
    if let Some(entree) = snapshot.entries().get(index)
        && correspond(&entree.value)
    {
        return Ok(index);
    }
    snapshot
        .entries()
        .iter()
        .position(|entree| correspond(&entree.value))
        .ok_or_else(|| {
            format!(
                "la transaction du {} ({:?}) n'est plus trouvée — le ledger a \
                 peut-être changé : rechargez",
                empreinte.date, empreinte.libelle
            )
        })
}

/// Édition d'une cellule budgétaire (writer chirurgical du jalon 3).
fn cellule_budget(
    main: &Path,
    annee: i16,
    date: NaiveDate,
    compte: &str,
    montant: rust_decimal::Decimal,
) -> LoadOutcome {
    let snapshot = match household_core::Snapshot::load(main) {
        Ok(snapshot) => snapshot,
        Err(e) => return LoadOutcome::Echec(e.to_string()),
    };
    match household_budget::writer::set_cell(&snapshot, annee, date, compte, montant) {
        Ok(nouveau) => match donnees_fraiches(&nouveau) {
            Ok(donnees) => LoadOutcome::Ecrit {
                donnees,
                message: format!("budget {compte} — {date} : {}", montant),
            },
            Err(e) => LoadOutcome::Echec(e),
        },
        Err(e) => LoadOutcome::Refuse(format!("budget refusé : {e}")),
    }
}

/// Génération d'une année budgétaire : duplication ± % puis écriture
/// (fichier + include, refus d'écrasement — jalon 3).
fn generer_annee(
    main: &Path,
    source: i16,
    cible: i16,
    pourcent: rust_decimal::Decimal,
    pas: rust_decimal::Decimal,
) -> LoadOutcome {
    let snapshot = match household_core::Snapshot::load(main) {
        Ok(snapshot) => snapshot,
        Err(e) => return LoadOutcome::Echec(e.to_string()),
    };
    let directives = household_budget::duplicate_year(
        &snapshot
            .entries()
            .iter()
            .map(|e| e.value.clone())
            .collect::<Vec<_>>(),
        source,
        cible,
        pourcent,
        pas,
    );
    let dupliquees = match directives {
        Ok(dupliquees) => dupliquees,
        Err(e) => return LoadOutcome::Refuse(format!("génération refusée : {e}")),
    };
    let nombre_postes = dupliquees.len();
    match household_budget::writer::write_year(&snapshot, cible, &dupliquees) {
        Ok(nouveau) => match donnees_fraiches(&nouveau) {
            Ok(donnees) => LoadOutcome::Ecrit {
                donnees,
                message: format!("année {cible} générée ({nombre_postes} postes)"),
            },
            Err(e) => LoadOutcome::Echec(e),
        },
        Err(e) => LoadOutcome::Refuse(format!("génération refusée : {e}")),
    }
}

/// View-model sur le snapshot **frais** post-écriture. Le writer a déjà
/// revalidé le ledger : un échec ici est un bug défensif (→ `Echec`).
fn donnees_fraiches(snapshot: &household_core::Snapshot) -> Result<Arc<UiData>, String> {
    viewmodel::build(snapshot, aujourd_hui()).map(Arc::new)
}

/// Surveille le répertoire du ledger (récursif) et demande un rechargement à
/// chaque événement (le worker débonce). Le watcher retourné doit rester
/// vivant : le détruire arrête la surveillance.
///
/// # Panics
/// Panique si le répertoire du ledger n'existe pas ou est inobservable :
/// sans watcher, la GUI doit s'arrêter tôt plutôt que se croire à jour.
pub fn demarrer_watcher(main: &Path, cmd_tx: Sender<Command>) -> notify::RecommendedWatcher {
    use notify::Watcher as _;

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
