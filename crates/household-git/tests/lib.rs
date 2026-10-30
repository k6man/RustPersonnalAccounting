//! Tests d'intégration de `household-git` sur des **dépôts temporaires
//! réels** (libgit2, pas de simulation).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use household_git::{self, EtatFichier};

fn scratch() -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let root = std::env::temp_dir().join(format!(
        "household-git-tests-{}-{nanos}-{seq}",
        std::process::id()
    ));
    // Dépôt = racine du scratch (ledger dans ledger/), initialisé via libgit2.
    let repo_dir = root.join("depot");
    fs::create_dir_all(repo_dir.join("ledger/transactions/2026")).expect("arborescence");
    git2::Repository::init(&repo_dir).expect("init du dépôt de test");
    repo_dir
}

fn ecrire(repo_dir: &Path, chemin: &str, contenu: &str) {
    let cible = repo_dir.join(chemin);
    if let Some(parent) = cible.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(cible, contenu).unwrap();
}

#[test]
fn cycle_complet_statut_index_commit_historique() {
    let repo_dir = scratch();
    ecrire(
        &repo_dir,
        "ledger/main.beancount",
        "option \"title\" \"Test\"\n",
    );
    let repo = household_git::decouvrir(&repo_dir.join("ledger")).expect("dépôt découvert");

    // Statut : tout nouveau (non suivi).
    let statuts = household_git::statut(&repo).expect("statut");
    assert!(statuts
        .iter()
        .any(|f| f.chemin == "ledger/main.beancount" && f.etats.contains(&EtatFichier::Nouveau)));

    // Indexation + commit initial.
    household_git::tout_indexer(&repo).expect("indexation");
    let oid = household_git::commit(&repo, "ledger: initial").expect("commit");
    assert_eq!(oid.len(), 7);

    // Plus rien à commiter.
    assert!(household_git::statut(&repo).expect("statut").is_empty());

    // Historique filtré sur le chemin du ledger.
    let historique =
        household_git::historique(&repo, Some("ledger/main.beancount"), 10).expect("historique");
    assert_eq!(historique.len(), 1);
    assert_eq!(historique[0].sujet, "ledger: initial");

    // Modification → statut modifié, diff non nul, message suggéré.
    ecrire(
        &repo_dir,
        "ledger/main.beancount",
        "option \"title\" \"Test v2\"\n",
    );
    ecrire(
        &repo_dir,
        "ledger/transactions/2026/04.beancount",
        "2026-04-10 * \"Test\"\n  Assets:Banque  -10.00 EUR\n  Expenses:Divers\n",
    );
    let statuts = household_git::statut(&repo).expect("statut");
    let main = statuts
        .iter()
        .find(|f| f.chemin == "ledger/main.beancount")
        .expect("main modifié");
    assert!(main.etats.contains(&EtatFichier::Modifie));

    let diff = household_git::resume_diff(&repo).expect("diff");
    assert_eq!(diff.fichiers, 2);
    assert!(diff.insertions > 0);

    assert_eq!(
        household_git::suggere_message(&statuts),
        "ledger: 2026-04 — saisie/édition (2 fichiers)"
    );

    // Commit des changements → l'historique filtré les contient.
    household_git::tout_indexer(&repo).expect("indexation");
    household_git::commit(&repo, "ledger: 2026-04 — saisie/édition").expect("commit");
    let historique = household_git::historique(&repo, Some("ledger"), 10).expect("historique");
    assert_eq!(historique.len(), 2);
    assert_eq!(
        historique[0].sujet, "ledger: 2026-04 — saisie/édition",
        "récent d'abord"
    );

    // Le diff est redevenu nul après commit.
    assert_eq!(
        household_git::resume_diff(&repo).expect("diff"),
        household_git::ResumeDiff::default()
    );

    let _ = fs::remove_dir_all(repo_dir.parent().unwrap());
}

#[test]
fn decouverte_echoue_hors_depot() {
    let racine = scratch();
    // Un sous-répertoire SANS dépôt parent : la scratch racine n'a pas de .git
    // (seul depot/ en aurait un).
    let erreur = household_git::decouvrir(racine.parent().unwrap());
    assert!(erreur.is_err(), "aucun .git au-dessus");
    let _ = fs::remove_dir_all(racine);
}

#[test]
fn historique_limite() {
    let repo_dir = scratch();
    let repo = household_git::decouvrir(&repo_dir).expect("dépôt");
    for i in 0..3 {
        ecrire(&repo_dir, "ledger/notes.txt", &format!("version {i}\n"));
        household_git::tout_indexer(&repo).expect("indexation");
        household_git::commit(&repo, &format!("ledger: étape {i}")).expect("commit");
    }
    let historique = household_git::historique(&repo, None, 2).expect("historique");
    assert_eq!(historique.len(), 2, "limite appliquée");
    assert_eq!(historique[0].sujet, "ledger: étape 2", "du plus récent");

    // Filtrage par chemin : les commits touchant ledger/notes.txt.
    let notes =
        household_git::historique(&repo, Some("ledger/notes.txt"), 10).expect("historique filtré");
    assert_eq!(notes.len(), 3);
    let _ = fs::remove_dir_all(repo_dir.parent().unwrap());
}
