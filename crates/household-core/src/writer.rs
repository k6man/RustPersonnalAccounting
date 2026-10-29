//! Écriture transactionnelle du ledger : append canonique, remplacement/
//! suppression par **span exact**, écriture **atomique**.
//!
//! Conventions :
//! - Un bloc de transaction = une transaction, jamais de re-sérialisation du
//!   fichier entier (commentaires et mise en forme de l'utilisateur préservés).
//! - Le texte du bloc est produit par `rustledger_core::format`
//!   (`FormatConfig::default()`), ce qui le rend canonique et stable.
//! - Insertion datée : le bloc est inséré avant la première ligne commençant
//!   par une date ISO strictement supérieure (ordre stable, fichier trié).
//! - Les fichiers mensuels (`transactions/AAAA/MM.beancount`) sont créés à la
//!   volée ; `main.beancount` inclue des globs et n'est **jamais** modifié.
//! - Toute mutation est vérifiée **avant** application : le ledger est copié
//!   dans un scratch temporaire, l'écriture est jouée là, et le chargement
//!   complet doit être sans erreur — sinon refus (l'utilisateur n'a rien à
//!   défaire). Puis écriture réelle atomique (temporaire + rename) et
//!   revalidation du ledger réel.
//! - Verrou `.write.lock` (rename/mtime, anti-stale) : deux processus ne
//!   peuvent pas écrire en même temps (GUI + éditeur externe + hook).

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rustledger_core::format::{FormatConfig, format_directives};
use rustledger_core::{Directive, NaiveDate, Transaction};
use rustledger_loader::LedgerError;

use crate::error::CoreError;
use crate::snapshot::Snapshot;

pub const TRANSACTIONS_DIR: &str = "transactions";
const LOCK_FILE: &str = ".write.lock";
const LOCK_ATTEMPTS: usize = 60;
const LOCK_RETRY_MS: u64 = 50;
const LOCK_STALE_SECS: u64 = 120;

/// Fichier mensuel cible d'une transaction datée,
/// `…/ledger/transactions/YYYY/MM.beancount`.
#[must_use]
pub fn monthly_file(main: &Path, date: NaiveDate) -> PathBuf {
    let root = main.parent().unwrap_or_else(|| Path::new("."));
    root.join(TRANSACTIONS_DIR)
        .join(date.year().to_string())
        .join(format!("{:02}.beancount", date.month()))
}

/// Ajoute une transaction (bloc canonique) dans le bon fichier mensuel,
/// au bon ordre de date, après validation sur copie. Retourne le
/// [`Snapshot`] rechargé du ledger.
pub fn append_transaction(snapshot: &Snapshot, txn: &Transaction) -> Result<Snapshot, CoreError> {
    let main = snapshot.main.clone();
    let target = monthly_file(&main, txn.date);
    mutate(&main, target, |old| {
        Ok(Some(insert_block(old, txn, ioutil_header(txn.date))))
    })
}

/// Remplace la transaction à l'index d'entrée donné par `new_txn`.
///
/// Même date : remplacement **sur place** (span exact). Autre date :
/// suppression + insertion datée de l'ancien fichier/nouveau fichier mensuel.
pub fn replace_transaction(
    snapshot: &Snapshot,
    index: usize,
    new_txn: &Transaction,
) -> Result<Snapshot, CoreError> {
    let old = snapshot
        .transaction(index)
        .ok_or(CoreError::NoSource(index))?;
    if old.date == new_txn.date {
        // Remplacement sur place : même span, même fichier. Le corps de
        // l'ancien bloc est remplacé (salé), pas concaténé.
        let main = snapshot.main.clone();
        let (file_id, span) = snapshot.span_of(index)?;
        let source = snapshot.source_of(index)?;
        let target = source.file;
        let block = canonical_block(new_txn);
        mutate(&main, target, |old_text| {
            Ok(Some(rewrite_span(
                old_text,
                file_id,
                span,
                |_, _old, after| format!("{block}\n{after}"),
            )))
        })
    } else {
        // Changement de date : l'ordre de tri doit être rétabli.
        let removed = remove_transaction(snapshot, index)?;
        append_transaction(&removed, new_txn)
    }
}

/// Supprime la transaction à l'index d'entrée donné (span exact,
/// lignes vides en excès normalisées).
pub fn remove_transaction(snapshot: &Snapshot, index: usize) -> Result<Snapshot, CoreError> {
    let main = snapshot.main.clone();
    let (file_id, span) = snapshot.span_of(index)?;
    let source = snapshot.source_of(index)?;
    let target = source.file;
    mutate(&main, target, |old_text| {
        Ok(Some(rewrite_span(
            old_text,
            file_id,
            span,
            |_, _, after| after.to_string(),
        )))
    })
}

// ------------------------------------------------------------------ canonique ---

/// Texte canonique d'une transaction (bloc autonome, sans newline final).
pub fn canonical_block(txn: &Transaction) -> String {
    canonical_directive_block(&Directive::Transaction(txn.clone()))
}

/// Texte canonique de n'importe quelle directive simple (une `custom "budget"`
/// par exemple) — bloc autonome d'une ligne pour `open`/`balance`/`custom`.
pub fn canonical_directive_block(directive: &Directive) -> String {
    format_directives([directive.clone()].iter(), &write_config())
}

/// Édition générique d'un fichier du ledger, avec **toutes** les garanties du
/// writer (verrou, pré-vérification scratch, atomique, revalidation) : `patch`
/// reçoit le contenu actuel et renvoie le nouveau (`Ok(None)` = refus).
/// C'est le point d'extension des writers spécialisés (budgets au jalon 3).
pub fn edit_ledger_file(
    main: &Path,
    target: PathBuf,
    patch: impl FnOnce(&str) -> Result<Option<String>, CoreError>,
) -> Result<Snapshot, CoreError> {
    mutate(main, target, patch)
}

/// Remplace le bloc couvert par `span` (édition chirurgicale hors writer) :
/// `f` reçoit (début du bloc, corps, suite) et renvoie le nouveau texte de
/// `corps + suite`. Public pour les writers spécialisés.
pub fn rewrite_span_block(
    old: &str,
    span: rustledger_parser::Span,
    f: impl FnOnce(usize, &str, &str) -> String,
) -> String {
    rewrite_span(old, 0, span, f)
}

/// Insère `block` (texte déjà canonique) dans `old` avant la première ligne
/// datée ISO strictement supérieure à `iso` ; `header` sert si le fichier est
/// vide ou absent. Public pour les writers spécialisés (budgets).
pub fn insert_sorted(old: &str, block: &str, iso: &str, header: Option<&str>) -> String {
    if old.trim().is_empty() {
        let mut out = header.unwrap_or("").to_string();
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(block);
        out.push('\n');
        return out;
    }

    let pos = find_insertion_byte(old, iso);
    let before = &old[..pos];
    let (lead, trail) = if before.is_empty() {
        (String::new(), "\n")
    } else {
        let k = before.bytes().rev().take_while(|&b| b == b'\n').count();
        let lead = match k {
            0 => "\n\n".to_string(),
            1 => "\n".to_string(),
            _ => String::new(),
        };
        (lead, "\n\n")
    };
    format!("{before}{lead}{block}{trail}{rest}", rest = &old[pos..],)
}

fn write_config() -> FormatConfig {
    FormatConfig::default()
}

/// Texte d'en-tête d'un fichier mensuel non encore créé.
fn ioutil_header(date: NaiveDate) -> String {
    let iso = date.to_string();
    let month = &iso[..7];
    format!("# Transactions {month} — créé automatiquement. Un bloc = une transaction.\n")
}

/// Insère le bloc canonique avant la première ligne-caractère de date >
/// strictement supérieure à la date de la transaction (ordre stable).
fn insert_block(old: &str, txn: &Transaction, header: String) -> String {
    insert_sorted(
        old,
        &canonical_block(txn),
        &txn.date.to_string(),
        Some(&header),
    )
}

/// Décalage d'octet du point d'insertion : avant la première ligne dont les
/// 10 premiers caractères forment une date ISO vide? > `iso`. Aucune ligne
/// de métadonnée (indentée) n'est prise pour une ligne-directive.
fn find_insertion_byte(text: &str, iso: &str) -> usize {
    let bytes = text.as_bytes();
    let mut line_start = 0;
    for (idx, ch) in bytes.iter().enumerate() {
        if *ch == b'\n' {
            if let Some(d) = line_iso(&bytes[line_start..idx])
                && d > iso
            {
                return line_start;
            }
            line_start = idx + 1;
        }
    }
    if let Some(d) = line_iso(&bytes[line_start..])
        && d > iso
    {
        return line_start;
    }
    text.len()
}

/// Les 10 premiers caractères de `line` (sans le "\n final) forment-ils
/// une date ISO probable ("2026-03-26 suivi d'un séparateur ou non-alphanum")?
fn line_iso(line: &[u8]) -> Option<&str> {
    if line.len() < 10 {
        return None;
    }
    let digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
    if !digits(&line[..4]) || line[4] != b'-' || !digits(&line[5..7]) {
        return None;
    }
    if line[7] != b'-' || !digits(&line[8..10]) {
        return None;
    }
    // Le 11e caractère, s'il existe, doit être un séparateur (une donnée
    // "2026-01-02…" ne doit jamais se confondre avec une ligne-directive).
    if line.len() > 10 && line[10].is_ascii_alphanumeric() {
        return None;
    }
    std::str::from_utf8(&line[..10]).ok()
}

// ------------------------------------------------------------ span rewrite ---

/// Découpe `old` au **bloc logique** d'une directive : (avant, corps avec son
/// newline final, suite).
///
/// Le span du parseur s'étend du début de la directive **au début de la
/// suivante** (parité beancount : il avale les lignes vides et commentaires
/// de queue). Le corps est donc recalé ici sur la directive elle-même :
/// première ligne + lignes indentées (postings, métadonnées), en s'arrêtant
/// à une ligne vide, un commentaire (`#`/`;`) ou une nouvelle directive.
/// Les commentaires de queue restent dans la suite — jamais perdus.
pub fn split_directive_block(old: &str, span: rustledger_parser::Span) -> (usize, &str, &str) {
    let bytes = old.as_bytes();
    let start = span.start.min(bytes.len());
    let first_nl = old[start..]
        .find('\n')
        .map_or(bytes.len(), |i| start + i + 1);
    let mut end = first_nl;

    // Continuations : lignes indentées non vides qui ne sont pas des
    // commentaires autonomes.
    let mut cursor = end;
    while cursor < bytes.len() {
        let line_end = old[cursor..]
            .find('\n')
            .map_or(bytes.len(), |i| cursor + i + 1);
        let line = old[cursor..line_end].trim_end();
        if line.is_empty() {
            break;
        }
        let stripped = line.trim_start();
        let first = stripped.as_bytes()[0];
        if first == b'#' || first == b';' {
            break; // commentaire autonome → hors bloc
        }
        if !line.starts_with([' ', '\t']) {
            break; // nouvelle directive (colonne 0)
        }
        end = line_end;
        cursor = line_end;
    }

    (start, &old[start..end], &old[end..])
}

/// Remplace un bloc de directive défini par son `span` d'octets : `f` reçoit
/// (début du bloc, corps logique, suite) et renvoie le nouveau texte de
/// `corps + suite` ; le texte **avant** le bloc est conservé tel quel.
fn rewrite_span(
    old: &str,
    _file_id: usize,
    span: rustledger_parser::Span,
    f: impl FnOnce(usize, &str, &str) -> String,
) -> String {
    let (start, body, after) = split_directive_block(old, span);
    let result = f(start, body, after);
    collapse_blank_runs(&format!("{}{result}", &old[..start]))
}

/// Réduit les triplets+ de lignes vides à une seule ligne vide (post-removal).
fn collapse_blank_runs(text: &str) -> String {
    let mut out = text.to_string();
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out
}

// ------------------------------------------------------------ mutation core ---

/// Patch d'un fichier : contenu actuel → nouveau contenu (`Ok(None)` = refus).
pub type PatchFn<'a> = Box<dyn FnOnce(&str) -> Result<Option<String>, CoreError> + 'a>;

/// Un changement de fichier du ledger : cible + patch.
pub struct FileChange<'a> {
    pub target: PathBuf,
    pub patch: PatchFn<'a>,
}

impl<'a> FileChange<'a> {
    pub fn new(
        target: PathBuf,
        patch: impl FnOnce(&str) -> Result<Option<String>, CoreError> + 'a,
    ) -> Self {
        Self {
            target,
            patch: Box::new(patch),
        }
    }
}

/// Applique une mutation sur **plusieurs** fichiers du ledger d'un seul coup,
/// *avec garantie* : verrou → pré-vérification sur copie scratch (toutes les
/// mutations jouées ensemble) → écritures atomiques → revalidation du ledger
/// réel. Atomicité logique : si un fichier échoue, rien n'est écrit.
pub fn edit_ledger_files(main: &Path, changes: Vec<FileChange<'_>>) -> Result<Snapshot, CoreError> {
    with_ledger_lock(main, || {
        let mut updated: Vec<(PathBuf, String)> = Vec::with_capacity(changes.len());
        for change in changes {
            let current = match fs::read_to_string(&change.target) {
                Ok(text) => text,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(CoreError::Io(e)),
            };
            let Some(new_content) = (change.patch)(&current)? else {
                return Err(CoreError::Writer("aucun changement à écrire".to_string()));
            };
            updated.push((change.target, new_content));
        }

        // 1. Pré-vérification sur copie scratch : l'écriture ne se joue que
        //    si le ledger est valide *avec* toutes les mutations.
        precheck_with(main, &updated)?;

        // 2. Écritures réelles, atomiques (temporaire + rename).
        for (target, content) in &updated {
            atomic_write(target, content.as_bytes())?;
        }

        // 3. Revalidation réelle et état frais.
        let replay = Snapshot::load(main)?;
        if !replay.is_valid() {
            return Err(CoreError::Writer(format!(
                "ledger invalide après écriture : {}",
                summarize_errors(&replay.ledger().errors)
            )));
        }
        Ok(replay)
    })
}

/// Applique une mutation sur un seul fichier (cas le plus courant).
fn mutate(
    main: &Path,
    target: PathBuf,
    patch: impl FnOnce(&str) -> Result<Option<String>, CoreError>,
) -> Result<Snapshot, CoreError> {
    edit_ledger_files(main, vec![FileChange::new(target, patch)])
}

/// Canonisation robuste : fichier existant → canonisé ; sinon → parent
/// canonisé + nom de fichier (cas des fichiers mensuels non encore créés).
fn canonize(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let parent = path
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            parent.join(path.file_name().unwrap_or_default())
        }
    }
}

/// Pré-vérification : copie du répertoire ledger dans un scratch, application
/// de **tous** les changements sur la copie, chargement complet → `Ok(())`
/// seulement si zéro erreur.
fn precheck_with(main: &Path, updated: &[(PathBuf, String)]) -> Result<(), CoreError> {
    // Canonisation (macOS : /var est un symlink vers /private/var, le loader
    // peut stocker les chemins canoniques) — sans canonisation, strip_prefix
    // échoue sur des chemins équivalents écrits différemment.
    let main = main.canonicalize().unwrap_or_else(|_| main.to_path_buf());
    let root = main
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let scratch = temp_dir("household-precheck");
    fs::create_dir_all(&scratch)?;
    let outcome = (|| -> Result<(), CoreError> {
        copy_tree(&root, &scratch.join("ledger"))?;
        for (target, content) in updated {
            let target = canonize(target);
            let rel = target
                .strip_prefix(&root)
                .map_err(|e| CoreError::Writer(format!("cible hors ledger : {e}")))?;
            let scratch_target = scratch.join("ledger").join(rel);
            if let Some(parent) = scratch_target.parent() {
                fs::create_dir_all(parent)?;
            }
            if content.is_empty() {
                let _ = fs::remove_file(&scratch_target);
            } else {
                fs::write(&scratch_target, content)?;
            }
        }
        let copy_main = scratch.join("ledger").join(
            main.file_name()
                .ok_or_else(|| CoreError::Writer("main sans nom".to_string()))?,
        );
        let copy = Snapshot::load(&copy_main)?;
        if !copy.is_valid() {
            return Err(CoreError::Writer(format!(
                "l'écriture invaliderait le ledger: {}",
                summarize_errors(&copy.ledger().errors)
            )));
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&scratch);
    outcome
}

/// Copie récursive minimale (fichiers texte du répertoire ledger) ;
/// ignore le verrou, caches rledger et temporaires éventuels.
fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == LOCK_FILE || name.ends_with(".cache") || name.contains(".tmp-") {
            continue;
        }
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// Écriture atomique : fichier temporaire caché (hors permutation des globs
/// `*.beancount`) + `rename`. `sync_all` avant le rename.
fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!(
        ".{}.tmp-{}",
        target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("ledger"),
        nanos()
    ));
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, target)?;
    Ok(())
}

fn with_ledger_lock<T>(
    main: &Path,
    body: impl FnOnce() -> Result<T, CoreError>,
) -> Result<T, CoreError> {
    let root = main.parent().unwrap_or_else(|| Path::new("."));
    let lock_path = root.join(LOCK_FILE);
    for attempt in 0..LOCK_ATTEMPTS {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                let _ = writeln!(file, "{}", std::process::id());
                let _ = file.sync_all();
                let outcome = body();
                let _ = fs::remove_file(&lock_path);
                return outcome;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Verrou périmé ? Les mtime de l'horloge murale servent de garde-fou.
                if attempt > LOCK_ATTEMPTS / 2
                    && let Ok(meta) = fs::metadata(&lock_path)
                    && let Ok(modified) = meta.modified()
                    && modified
                        .elapsed()
                        .map(|d| d > Duration::from_secs(LOCK_STALE_SECS))
                        .unwrap_or(false)
                {
                    let _ = fs::remove_file(&lock_path);
                }
                thread::sleep(Duration::from_millis(LOCK_RETRY_MS));
            }
            Err(e) => return Err(CoreError::Io(e)),
        }
    }
    Err(CoreError::Writer(format!(
        "ledger verrouillé (toujours {LOCK_FILE} après {} tentatives)",
        LOCK_ATTEMPTS
    )))
}

fn summarize_errors(errors: &[LedgerError]) -> String {
    let mut out = errors
        .iter()
        .take(5)
        .map(|e| match &e.location {
            Some(loc) => format!(
                "[{} @ {}:{}{}] {}",
                e.code,
                loc.file.display(),
                loc.line,
                loc.column,
                e.message
            ),
            None => format!("[{}] {}", e.code, e.message),
        })
        .collect::<Vec<_>>()
        .join(" | ");
    if errors.len() > 5 {
        out.push_str(&format!(" … ({} erreurs au total)", errors.len()));
    }
    out
}

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

fn temp_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}-{}", std::process::id(), nanos()))
}
