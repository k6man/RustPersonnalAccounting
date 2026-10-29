//! Cas de test partagés du crate : copie scratch du ledger échantillon.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Copie du répertoire ledger dans un scratch ; retourne le chemin du
/// `main.beancount` de la copie (aucun test ne touche au ledger réel).
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
pub fn cleanup(scratch: &Path) {
    let _ = fs::remove_dir_all(scratch);
}

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

/// Compteur atomique : les tests tournent en parallèle et l'horloge murale
/// (résolution ~µs sur macOS) ne distingue pas deux appels simultanés.
fn temp_root() -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "household-gui-tests-{}-{nanos}-{seq}",
        std::process::id()
    ))
}
