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

use rustledger_core::{Directive, IncompleteAmount, NaiveDate};

use crate::saisie::{Brouillon, Empreinte};
use crate::viewmodel::{self, UiData};

/// Envoie une valeur sur un canal sans paniquer si le recepteur est ferme.
/// Loggue un avertissement si le message est perdu (canal deconecte).
fn send_sans_panique<T>(tx: &Sender<T>, valeur: T, contexte: &str) {
    if let Err(e) = tx.send(valeur) {
        eprintln!("⚠ canal du worker ferme ({contexte}), message perdu : {e}");
    }
}

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
    /// Écran Git : charger le statut / diff résumé / historique.
    GitStatut,
    /// Écran Git : charger les données, éventuellement commiter.
    ///
    /// `message = Some` ⇒ indexer + commiter (après validation du ledger —
    /// git2 n'exécute pas les hooks, cf. ADR-002) ; `None` ⇒ lecture seule.
    GitCommit {
        /// Message de commit.
        message: String,
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
    /// Données git chargées (écran Git) + flash (témoin de succès/erreur).
    Git {
        donnees: GitDonnees,
        /// Message flash : `Some((texte, est_erreur))`.
        message: Option<(String, bool)>,
    },
}

/// Données de l'écran **Git** (chargées hors UI thread).
#[derive(Debug, Clone, Default)]
pub struct GitDonnees {
    /// Le dépôt git englobe-t-il le ledger ? (`false` → écran informatif.)
    pub disponible: bool,
    /// Fichiers modifiés (index + arborescence).
    pub fichiers: Vec<household_git::FichierStatut>,
    /// Résumé du diff HEAD → index + arborescence.
    pub resume: household_git::ResumeDiff,
    /// Historique des commits touchant le ledger.
    pub historique: Vec<household_git::CommitInfo>,
    /// Message de commit suggéré (mois des transactions modifiées).
    pub message_suggere: String,
}

/// Délai de « settle » avant rechargement après un événement du watcher :
/// les éditeurs (et nos propres écritures atomiques) génèrent plusieurs
/// événements rapprochés.
const DEBOUNCE: Duration = Duration::from_millis(250);

/// Démarre le worker. Il charge immédiatement puis boucle sur les commandes.
/// Le récepteur des résultats est retourné pour l'UI.
///
/// La boucle UI (ratatui) redessine à chaque itération : le worker n'a pas
/// besoin de demander un repaint explicite.
pub fn demarrer(main: PathBuf) -> (Sender<Command>, Receiver<LoadOutcome>) {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Command>();
    let (out_tx, out_rx) = std::sync::mpsc::channel::<LoadOutcome>();
    // Le worker peut remettre des commandes en file (drain du debounce) —
    // il lui faut son propre émetteur vers le canal des commandes.
    let remise_en_file = cmd_tx.clone();

    thread::spawn(move || {
        // Premier chargement : la GUI a quelque chose à montrer tout de suite.
        send_sans_panique(&out_tx, charger(&main), "chargement initial");

        while let Ok(commande) = cmd_rx.recv() {
            match commande {
                Command::Reload => {
                    // Debounce : écrête la rafale d'événements du watcher
                    // SANS jeter les commandes utilisateur — une mutation
                    // (écriture, édition, suppression…) arrivée pendant la
                    // rafale est remise en fin de file et traitée dans
                    // l'ordre, avec son propre résultat. (Régression
                    // corrigée : le drain d'origine aspirait TOUTES les
                    // commandes en attente, écritures comprises.)
                    let mut conservees: Vec<Command> = Vec::new();
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        if !matches!(cmd, Command::Reload) {
                            conservees.push(cmd);
                        }
                    }
                    thread::sleep(DEBOUNCE);
                    while let Ok(cmd) = cmd_rx.try_recv() {
                        if !matches!(cmd, Command::Reload) {
                            conservees.push(cmd);
                        }
                    }
                    for cmd in conservees {
                        // Remise en file : le rechargement ci-dessous part
                        // d'abord ; ces commandes seront ensuite traitées
                        // dans l'ordre, chacune avec son propre résultat.
                        send_sans_panique(&remise_en_file, cmd, "remise en file commande debounce");
                    }

                    send_sans_panique(&out_tx, charger(&main), "chargement apres debounce");
                }
                Command::Ecrire(brouillon) => {
                    send_sans_panique(&out_tx, ecrire(&main, &brouillon), "ecriture transaction");
                }
                Command::Modifier {
                    index,
                    empreinte,
                    brouillon,
                } => {
                    send_sans_panique(
                        &out_tx,
                        modifier(&main, index, &empreinte, &brouillon),
                        "modification transaction",
                    );
                }
                Command::Supprimer { index, empreinte } => {
                    send_sans_panique(
                        &out_tx,
                        supprimer(&main, index, &empreinte),
                        "suppression transaction",
                    );
                }
                Command::CelluleBudget {
                    annee,
                    date,
                    compte,
                    montant,
                } => {
                    send_sans_panique(
                        &out_tx,
                        cellule_budget(&main, annee, date, &compte, montant),
                        "cellule budget",
                    );
                }
                Command::GenererAnnee {
                    source,
                    cible,
                    pourcent,
                    pas,
                } => {
                    send_sans_panique(
                        &out_tx,
                        generer_annee(&main, source, cible, pourcent, pas),
                        "generation annee",
                    );
                }
                Command::GitStatut => {
                    send_sans_panique(&out_tx, git_resultat(&main, None), "statut git");
                }
                Command::GitCommit { message } => {
                    send_sans_panique(&out_tx, git_resultat(&main, Some(&message)), "commit git");
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
                && match empreinte.montant {
                    // Quand le montant est connu, il discrimine les jumelles.
                    // Même logique de montant que LigneJournal (absolu du
                    // plus grand posting) pour que la comparaison soit
                    // cohérente avec l'affichage du journal.
                    Some(m) => t.postings.iter().any(|p| {
                        matches!(
                            &p.units,
                            Some(IncompleteAmount::Complete(a))
                                if a.number.abs() == m
                        )
                    }),
                    None => true, // rétro-compatible : critère ignoré
                }
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

/// Écran Git : charger les données, et si `message` est fourni, **valider
/// le ledger → indexer → commiter** puis renvoyer les données fraîches.
///
/// Validation avant commit côté app : git2 (libgit2) n'exécute pas les
/// hooks pre-commit — cf. ADR-002 — donc l'écran refuse de commiter si le
/// ledger ne se charge pas sans erreur.
fn git_resultat(main: &Path, message: Option<&str>) -> LoadOutcome {
    let repo = match household_git::decouvrir(main) {
        Ok(repo) => repo,
        Err(e) => {
            return LoadOutcome::Git {
                donnees: GitDonnees::default(),
                message: Some((e.to_string(), true)),
            };
        }
    };

    if let Some(message) = message {
        // 1. Rien à commiter ?
        match household_git::statut(&repo) {
            Ok(statuts) => {
                if statuts.is_empty() {
                    return LoadOutcome::Refuse(
                        "rien à commiter — aucun fichier modifié".to_string(),
                    );
                }
            }
            Err(e) => {
                return LoadOutcome::Git {
                    donnees: GitDonnees::default(),
                    message: Some((e.to_string(), true)),
                };
            }
        }
        // 2. Validation du ledger (comme le hook pre-commit, mais côté app).
        if let Err(e) = household_core::Snapshot::load(main) {
            return LoadOutcome::Refuse(format!(
                "ledger invalide — commit refusé : {e}  |  corrigez avant de commiter"
            ));
        }
        // 3. Index + commit.
        if let Err(e) = household_git::tout_indexer(&repo) {
            return LoadOutcome::Git {
                donnees: GitDonnees::default(),
                message: Some((e.to_string(), true)),
            };
        }
        return match household_git::commit(&repo, message) {
            Ok(oid) => match git_donnees(&repo) {
                Ok(donnees) => LoadOutcome::Git {
                    donnees,
                    message: Some((format!("commité {oid}"), false)),
                },
                Err(e) => LoadOutcome::Git {
                    donnees: GitDonnees::default(),
                    message: Some((e.to_string(), true)),
                },
            },
            Err(e) => LoadOutcome::Git {
                donnees: GitDonnees::default(),
                message: Some((e.to_string(), true)),
            },
        };
    }

    // Sans message = simple lecture du statut (le déclencheur de l'écran).
    match git_donnees(&repo) {
        Ok(donnees) => LoadOutcome::Git {
            donnees,
            message: None,
        },
        Err(e) => LoadOutcome::Git {
            donnees: GitDonnees::default(),
            message: Some((e, true)),
        },
    }
}

/// Assemble les données git (statut + diff résumé + historique filtré).
fn git_donnees(repo: &household_git::Repository) -> Result<GitDonnees, String> {
    let fichiers = household_git::statut(repo).map_err(|e| e.to_string())?;
    let message_suggere = household_git::suggere_message(&fichiers);
    let resume = household_git::resume_diff(repo).map_err(|e| e.to_string())?;
    let historique =
        household_git::historique(repo, Some("ledger"), 20).map_err(|e| e.to_string())?;
    Ok(GitDonnees {
        disponible: true,
        fichiers,
        resume,
        historique,
        message_suggere,
    })
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
/// Panique si le répertoire du ledger *existe* mais est inobservable
/// (sandbox, répertoire protégé) : sans surveillance possible, mieux vaut
/// s'arrêter tôt. En revanche, si le répertoire n'existe pas encore (ledger
/// configuré mais absent au démarrage — ex. chemin fantôme, disque démonté),
/// cette fonction dégrade élégamment : pas de surveillance, la GUI démarre
/// quand même avec sa bannière d'échec de chargement et « Recharger ».
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
                send_sans_panique(&cmd_tx, Command::Reload, "reload watcher");
            }
        })
        .expect("watcher du ledger");
    if dossier.exists() {
        watcher
            .watch(&dossier, notify::RecursiveMode::Recursive)
            .expect("surveillance du répertoire ledger");
    } else {
        // Rétablissement possible : le watcher n'est pas lancé ; si le
        // dossier apparaît ensuite, le watcher ne le verra pas — le bouton
        // « Recharger » fait le job (l'échec au premier chargement est
        // déjà en bannière).
        eprintln!(
            "⚠ surveillance désactivée (répertoire absent) : {}",
            dossier.display()
        );
    }
    watcher
}

/// Date du jour (injectée dans le view-model pour la testabilité).
#[must_use]
pub fn aujourd_hui() -> NaiveDate {
    jiff::Zoned::now().date()
}
