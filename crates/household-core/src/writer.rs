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
    format_directives(
        [Directive::Transaction(txn.clone())].iter(),
        &write_config(),
    )
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
    if old.trim().is_empty() {
        let mut out = header;
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(&canonical_block(txn));
        out.push('\n');
        return out;
    }

    let pos = find_insertion_byte(old, &txn.date.to_string());
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
    format!(
        "{before}{lead}{}{trail}{rest}",
        canonical_block(txn),
        rest = &old[pos..],
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

/// Remplace un bloc de directive défini par son `span` d'octets : `f` reçoit
/// (avant du bloc, bloc entier?, après) ; on construit le nouveau contenu
/// du fichier (fichier réel, span de la copie scratch identique).
#[allow(clippy::needless_pass_by_value)]
fn rewrite_span(
    old: &str,
    _file_id: usize,
    span: rustledger_parser::Span,
    f: impl FnOnce(usize, &str, &str) -> String,
) -> String {
    let bytes = old.as_bytes();
    // Début : la directive commence au début de sa ligne.
    let start = bytes[..span.start.min(bytes.len())]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    // Fin : le span couvre la directive ; on avale son newline final.
    let mut end = span.end.min(bytes.len());
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    }
    // (Le texte avant le bloc est conservé tel quel par construction :
    // `f` ne reçoit que le corps et la suite.)
    let body = &old[start..end];
    let after = &old[end..];
    let result = f(start, body, after);
    collapse_blank_runs(&result)
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

/// Applique une mutation sur un fichier du ledger, *avec garantie* :
/// verrou → prévérification sur copie scratch → écriture atomique →
/// revalidation du ledger réel. Le nouveau [`Snapshot`] est retourné.
///
/// `patch` reçoit le contenu actuel du fichier cible ("" si absent) et
/// renvoie le nouveau contenu, ou `Ok(None)` pour demander un refus.
fn mutate(
    main: &Path,
    target: PathBuf,
    patch: impl FnOnce(&str) -> Result<Option<String>, CoreError>,
) -> Result<Snapshot, CoreError> {
    with_ledger_lock(main, || {
        let current = match fs::read_to_string(&target) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(CoreError::Io(e)),
        };
        let Some(new_content) = patch(&current)? else {
            return Err(CoreError::Writer("aucun changement à écrire".to_string()));
        };

        // 1. Pré-vérification sur copie scratch : l'écriture ne se joue que
        //    si le ledger est valide *avec* la mutation.
        precheck_with(main, &target, new_content.as_bytes())?;

        // 2. Écriture réelle, atomique (temporaire + rename).
        atomic_write(&target, new_content.as_bytes())?;

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
/// du patch sur la copie, chargement complet → `Ok(())` seulement si zéro erreur.
fn precheck_with(main: &Path, target: &Path, new_content: &[u8]) -> Result<(), CoreError> {
    // Canonisation (macOS : /var est un symlink vers /private/var, le loader
    // peut stocker les chemins canoniques) — sans canonisation, strip_prefix
    // échoue sur des chemins équivalents écrits différemment.
    let main = main.canonicalize().unwrap_or_else(|_| main.to_path_buf());
    let target = canonize(target);
    let root = main
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let scratch = temp_dir("household-precheck");
    fs::create_dir_all(&scratch)?;
    let outcome = (|| -> Result<(), CoreError> {
        copy_tree(&root, &scratch.join("ledger"))?;
        let rel = target
            .strip_prefix(root)
            .map_err(|e| CoreError::Writer(format!("cible hors ledger : {e}")))?;
        let scratch_target = scratch.join("ledger").join(rel);
        if let Some(parent) = scratch_target.parent() {
            fs::create_dir_all(parent)?;
        }
        if new_content.is_empty() {
            let _ = fs::remove_file(&scratch_target);
        } else {
            fs::write(&scratch_target, new_content)?;
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
