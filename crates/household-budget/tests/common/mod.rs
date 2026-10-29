//! Cas de test partagés du crate : builders de directives budget/transactions
//! (mini-ledgers en mémoire) et copie scratch du ledger échantillon pour les
//! suites d'écriture.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rust_decimal::Decimal;
use rustledger_core::{Amount, Directive, NaiveDate, Posting, Transaction};

/// Directive `custom "budget"` canonique (compte EUR).
pub fn budget(date: &str, account: &str, interval: &str, amount: Decimal) -> Directive {
    Directive::Custom(
        rustledger_core::Custom::new(date.parse::<NaiveDate>().expect("date ISO"), "budget")
            .with_value(rustledger_core::MetaValue::Account(account.into()))
            .with_value(rustledger_core::MetaValue::String(interval.to_string()))
            .with_value(rustledger_core::MetaValue::Amount(Amount::new(
                amount, "EUR",
            ))),
    )
}

/// Directive `open` (comptes simples, EUR).
pub fn open(date: &str, account: &str) -> Directive {
    Directive::Open(rustledger_core::Open::new(
        date.parse::<NaiveDate>().expect("date ISO"),
        account,
    ))
}

/// Transaction d'achat : `Expenses:…` +montant / `Assets:Banque` −montant.
pub fn spend(date: &str, account: &str, amount: Decimal) -> Directive {
    Directive::Transaction(
        Transaction::new(date.parse::<NaiveDate>().expect("date ISO"), "test")
            .with_flag('*')
            .with_synthesized_posting(Posting::new(account, Amount::new(amount, "EUR")))
            .with_synthesized_posting(Posting::new(
                "Assets:Banque:CompteCourant",
                Amount::new(-amount, "EUR"),
            )),
    )
}

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
        "household-budget-tests-{}-{nanos}-{seq}",
        std::process::id()
    ))
}

/// Contenu du fichier budgétaire d'une année de la copie.
pub fn budget_text(main: &Path, year: i16) -> String {
    fs::read_to_string(budget_file_of(main, year)).expect("fichier budget de la copie")
}

/// Chemin du fichier budgétaire d'une année de la copie.
pub fn budget_file_of(main: &Path, year: i16) -> PathBuf {
    main.parent()
        .expect("parent")
        .join("budgets")
        .join(format!("{year}.beancount"))
}
