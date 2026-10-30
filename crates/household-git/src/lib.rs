//! Opérations git sur le dépôt du ledger, via **git2** (libgit2 en
//! processus — pas de binaire `git`, cf. ADR-001).
//!
//! ⚠ **Choix documenté (ADR-001)** : git2 n'exécute pas les hooks ; la
//! validation du ledger **avant** commit est faite côté application
//! (worker : `Snapshot::load` sans erreur), en complément du hook
//! pre-commit qui couvre les commits passant par la CLI.
//!
//! Tout est testé sur des dépôts temporaires réels — aucune simulation.

use std::path::Path;

pub use git2::Repository;
use git2::{Sort, Status};
use thiserror::Error;

/// Erreurs git (emballage lisible de `git2::Error`).
#[derive(Debug, Error)]
pub enum GitErreur {
    /// Dépôt introuvable au-dessus du chemin fourni.
    #[error("dépôt git introuvable au-dessus de {0}")]
    Introuvable(String),
    /// Erreur libgit2.
    #[error("git : {0}")]
    Git(#[from] git2::Error),
}

/// État d'un fichier par rapport à l'index et au dernier commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtatFichier {
    /// Nouveau (non suivi / dans l'index).
    Nouveau,
    /// Modifié (index ou arborescence).
    Modifie,
    /// Supprimé.
    Supprime,
    /// Renommé.
    Renomme,
}

impl EtatFichier {
    /// Libellé français pour l'écran Git.
    #[must_use]
    pub fn libelle(self) -> &'static str {
        match self {
            Self::Nouveau => "nouveau",
            Self::Modifie => "modifié",
            Self::Supprime => "supprimé",
            Self::Renomme => "renommé",
        }
    }
}

/// Un fichier modifié, avec ses états (index **et** arborescence peuvent
/// cumuler : « modifié, indexé »).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichierStatut {
    pub chemin: String,
    pub etats: Vec<EtatFichier>,
}

/// Une entrée d'historique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitInfo {
    /// OID court (7 caractères).
    pub oid: String,
    /// Première ligne du message.
    pub sujet: String,
    /// Date au format ISO (`2026-10-29 14:32`).
    pub date: String,
    pub auteur: String,
}

/// Résumé du diff index+arborescence vs HEAD.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResumeDiff {
    pub fichiers: usize,
    pub insertions: usize,
    pub suppressions: usize,
}

/// Découvre le dépôt contenant `chemin` (remonte jusqu'au `.git`).
///
/// # Erreurs
/// [`GitErreur::Introuvable`] si aucun dépôt n'englobe le chemin.
pub fn decouvrir(chemin: &Path) -> Result<Repository, GitErreur> {
    Repository::discover(chemin).map_err(|_| GitErreur::Introuvable(chemin.display().to_string()))
}

/// Statut de tous les fichiers modifiés (index + arborescence).
///
/// # Erreurs
/// Propage les erreurs libgit2.
pub fn statut(repo: &Repository) -> Result<Vec<FichierStatut>, GitErreur> {
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut options))?;
    let mut out = Vec::new();
    for entree in statuses.iter() {
        let Some(chemin) = entree.path().ok() else {
            continue;
        };
        let s = entree.status();
        let mut etats = Vec::new();
        if s.intersects(Status::WT_NEW | Status::INDEX_NEW) {
            etats.push(EtatFichier::Nouveau);
        }
        if s.intersects(Status::WT_MODIFIED | Status::INDEX_MODIFIED) {
            etats.push(EtatFichier::Modifie);
        }
        if s.intersects(Status::WT_DELETED | Status::INDEX_DELETED) {
            etats.push(EtatFichier::Supprime);
        }
        if s.intersects(Status::WT_RENAMED | Status::INDEX_RENAMED) {
            etats.push(EtatFichier::Renomme);
        }
        if etats.is_empty() {
            continue;
        }
        out.push(FichierStatut {
            chemin: chemin.to_string(),
            etats,
        });
    }
    Ok(out)
}

/// Indexe tout (fichiers modifiés + nouveaux, pas les suppressions
/// manuelles d'arborescence) et écrit l'index.
///
/// # Erreurs
/// Propage les erreurs libgit2.
pub fn tout_indexer(repo: &Repository) -> Result<(), GitErreur> {
    let mut index = repo.index()?;
    index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
    // Les suppressions de l'arborescence doivent aussi passer dans l'index.
    index.update_all(["*"], None)?;
    index.write()?;
    Ok(())
}

/// Crée un commit de l'index courant sur la branche HEAD.
///
/// # Erreurs
/// Propage les erreurs libgit2 (index vide compris — l'app vérifie le statut
/// avant d'appeler).
pub fn commit(repo: &Repository, message: &str) -> Result<String, GitErreur> {
    let mut index = repo.index()?;
    index.write()?;
    let tree_oid = index.write_tree()?;
    let arbre = repo.find_tree(tree_oid)?;

    let (nom, courriel) = identite(repo);
    let signature = git2::Signature::now(&nom, &courriel)?;

    // Parent = HEAD (peut ne pas exister : premier commit).
    let head_commit = repo
        .head()
        .ok()
        .and_then(|reference| reference.peel_to_commit().ok());
    let parents_refs: Vec<&git2::Commit> = head_commit.iter().collect();

    let oid = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &arbre,
        &parents_refs,
    )?;
    Ok(oid.to_string()[..7].to_string())
}

/// Identité du committer : config git du dépôt/global, repli local si absente.
fn identite(repo: &Repository) -> (String, String) {
    let config = repo.config().ok();
    let lire = |cle: &str| -> Option<String> {
        config
            .as_ref()
            .and_then(|c| c.get_string(cle).ok())
            .filter(|v| !v.trim().is_empty())
    };
    let nom = lire("user.name").unwrap_or_else(|| "Comptabilité du foyer".to_string());
    let courriel = lire("user.email").unwrap_or_else(|| "ledger@local".to_string());
    (nom, courriel)
}

/// Historique (du plus récent), éventuellement limité aux commits touchant
/// `chemin` (préfixe de chemin relatif au dépôt).
///
/// # Erreurs
/// Propage les erreurs libgit2.
pub fn historique(
    repo: &Repository,
    chemin: Option<&str>,
    limite: usize,
) -> Result<Vec<CommitInfo>, GitErreur> {
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TIME)?;
    revwalk.push_head()?;

    let mut out = Vec::new();
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        if let Some(chemin) = chemin
            && !touche_chemin(&commit, chemin)?
        {
            continue;
        }
        let time = commit.time();
        // Horodatage UTC, format « 2026-10-29 14:32 ».
        let date = jiff::Timestamp::from_second(time.seconds())
            .map(|t| t.to_zoned(jiff::tz::TimeZone::UTC))
            .map(|z| z.strftime("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        out.push(CommitInfo {
            oid: oid.to_string()[..7].to_string(),
            sujet: commit.summary().ok().flatten().unwrap_or("").to_string(),
            date,
            auteur: commit.author().name().unwrap_or("").to_string(),
        });
        if out.len() >= limite {
            break;
        }
    }
    Ok(out)
}

/// Le commit touche-t-il ce chemin ? Comparaison de l'entrée d'arbre avec
/// celle du premier parent (création/modification/suppression = touché).
fn touche_chemin(commit: &git2::Commit, chemin: &str) -> Result<bool, GitErreur> {
    let oid_noeud = |arbre: &git2::Tree| -> Option<git2::Oid> {
        arbre
            .get_path(Path::new(chemin))
            .ok()
            .map(|entree| entree.id())
    };
    let courant = oid_noeud(&commit.tree()?);
    let parent = commit
        .parent(0)
        .ok()
        .and_then(|p| p.tree().ok())
        .and_then(|t| oid_noeud(&t));
    Ok(courant != parent)
}

/// Résumé du diff HEAD → index+arborescence (fichiers, insertions,
/// suppressions).
///
/// # Erreurs
/// Propage les erreurs libgit2.
pub fn resume_diff(repo: &Repository) -> Result<ResumeDiff, GitErreur> {
    let head_arbre = repo
        .head()
        .ok()
        .and_then(|reference| reference.peel_to_tree().ok());
    let mut options = git2::DiffOptions::new();
    // Les fichiers non suivis comptent aussi (nouvelles transactions).
    options.include_untracked(true).recurse_untracked_dirs(true);
    let diff = repo.diff_tree_to_workdir_with_index(head_arbre.as_ref(), Some(&mut options))?;
    let stats = diff.stats()?;
    Ok(ResumeDiff {
        fichiers: stats.files_changed(),
        insertions: stats.insertions(),
        suppressions: stats.deletions(),
    })
}

/// Suggère un message de commit d'après les fichiers modifiés :
/// mois des transactions (`…/transactions/AAAA/MM.beancount`, quel que soit
/// le préfixe — `ledger/transactions/…` dans le dépôt réel), budgets, etc.
/// Format : `ledger: AAAA-MM — saisie/édition (N fichier(s))`.
#[must_use]
pub fn suggere_message(fichiers: &[FichierStatut]) -> String {
    let mois_fichier = |chemin: &str| -> Option<String> {
        // …/transactions/2026/04.beancount → 2026-04
        let morceaux: Vec<&str> = chemin.split('/').collect();
        let position = morceaux.iter().position(|m| *m == "transactions")?;
        let annee = morceaux.get(position + 1)?;
        let fichier = morceaux.get(position + 2)?;
        if annee.len() == 4 && fichier.ends_with(".beancount") && fichier.len() >= 2 {
            Some(format!("{annee}-{}", &fichier[..2]))
        } else {
            None
        }
    };
    let mut mois: Vec<String> = fichiers
        .iter()
        .filter_map(|f| mois_fichier(&f.chemin))
        .collect();
    mois.sort();
    mois.dedup();

    if let Some(premier) = mois.first() {
        let n = fichiers.len();
        let pluriel = if n > 1 { "s" } else { "" };
        if mois.len() == 1 {
            format!("ledger: {premier} — saisie/édition ({n} fichier{pluriel})")
        } else {
            format!(
                "ledger: {} → {} — saisie/édition ({} fichiers)",
                premier,
                mois.last().unwrap_or(premier),
                n
            )
        }
    } else if fichiers.iter().any(|f| f.chemin.starts_with("budgets/")) {
        format!("ledger: budgets — édition ({})", fichiers.len())
    } else {
        format!("ledger: mise à jour ({} fichiers)", fichiers.len())
    }
}
