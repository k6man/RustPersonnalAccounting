//! Cas de test partagés des suites d'intégration : copie scratch du ledger
//! échantillon (aucun test ne touche au ledger réel).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Copie du répertoire ledger dans un répertoire temporaire ; retourne le
/// chemin du `main.beancount` de la copie. Le répertoire est à nettoyer via
/// [`cleanup`].
pub fn ledger_copy() -> PathBuf {
    let real = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ledger/main.beancount");
    if !real.exists() {
        panic!("ledger échantillon introuvable pour les tests d'intégration");
    }
    let root = real.parent().expect("ledger/main.beancount a un parent");
    let scratch = temp_root();
    let ledger = scratch.join("ledger");
    copy_inner(root, &ledger).expect("copie du ledger");
    ledger.join("main.beancount")
}

/// Nettoyage du scratch de test.
pub fn cleanup(root: &Path) {
    let _ = fs::remove_dir_all(root);
}

/// Racine du scratch (`…/Temp/household-tests-…/ledger/`).
fn copy_inner(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_inner(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn temp_root() -> PathBuf {
    // Compteur atomique : les tests tournent en parallèle dans le même
    // processus et l'horloge murale (résolution ~µs sur macOS) ne suffit pas
    // à distinguer deux appels simultanés.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "household-tests-{}-{nanos}-{seq}",
        std::process::id()
    ))
}

/// Le contenu d'un fichier mensuel (helper d'assertions).
pub fn monthly(main: &Path, year: i16, month: u8) -> String {
    fs::read_to_string(
        main.parent()
            .expect("parent")
            .join("transactions")
            .join(year.to_string())
            .join(format!("{month:02}.beancount")),
    )
    .expect("fichier mensuel de la copie")
}
