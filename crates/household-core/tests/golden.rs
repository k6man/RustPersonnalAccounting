//! Golden files : lecture du ledger échantillon → soldes/plafonds attendus.
//!
//! Ces valeurs sont les assertions `balance` du ledger lui-même (contrôle
//! 2026-04-01, sémantique v3 à 00:00) : le test vérifie que notre couche de
//! chargement + calcul reproduit exactement ce que `rledger` affirme.

mod common;

use household_core::{Snapshot, Through};
use rust_decimal_macros::dec;

const MAIN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/ledger/main.beancount"
);

fn snapshot() -> Option<Snapshot> {
    let main = std::path::Path::new(MAIN);
    if !main.exists() {
        eprintln!("ledger absent de la sandbox → suite ignorée");
        return None;
    }
    Some(Snapshot::load(main).expect("chargement du ledger échantillon"))
}

#[test]
fn ledger_est_valide() {
    let Some(snapshot) = snapshot() else {
        return;
    };
    assert!(
        snapshot.is_valid(),
        "erreurs : {:?}",
        snapshot.errors().iter().collect::<Vec<_>>()
    );
}

#[test]
fn soldes_controles_au_1er_avril() {
    let Some(snapshot) = snapshot() else {
        return;
    };
    let day = "2026-04-01".parse().expect("date ISO");
    let balances = snapshot
        .balances_at(day, Through::StartOfDay)
        .expect("calcul des soldes après assertion");

    let expected: &[(&str, &str)] = &[
        ("Assets:Banque:CompteCourant", "6877.39"),
        ("Assets:Epargne:LivretA", "19602.49"),
        ("Assets:Epargne:LDD", "10466.99"),
    ];
    for (account, amount) in expected {
        let row = balances
            .iter()
            .find(|b| b.account == *account)
            .unwrap_or_else(|| panic!("solde absent pour {account}"));
        assert_eq!(
            row.amounts.get("EUR").copied().unwrap_or_default(),
            amount.parse::<rust_decimal::Decimal>().unwrap(),
            "solde EUR de {account} au contrôle du 2026-04-01"
        );
    }
}

#[test]
fn plafonds_lus_des_metas_open() {
    let Some(snapshot) = snapshot() else {
        return;
    };
    let ceilings = snapshot.ceilings();
    let livret = ceilings
        .get("Assets:Epargne:LivretA")
        .expect("plafond LivretA");
    assert_eq!(livret.amount, dec!(22950.00));
    assert_eq!(livret.currency, "EUR");
    let ldd = ceilings.get("Assets:Epargne:LDD").expect("plafond LDD");
    assert_eq!(ldd.amount, dec!(12000.00));
    assert_eq!(ceilings.len(), 2, "seul les deux livrets ont un plafond");
}

#[test]
fn fin_de_ledger_au_dernier_jour_transactionne() {
    let Some(snapshot) = snapshot() else {
        return;
    };
    let last = snapshot
        .last_transaction_date()
        .expect("le ledger échantillon a des transactions");
    assert_eq!(
        last.to_string(),
        "2026-03-31",
        "le dernier jour transactionné"
    );

    let balances = snapshot
        .balances_at(last, Through::EndOfDay)
        .expect("soldes fin de ledger");
    let cc = balances
        .iter()
        .find(|b| b.account == "Assets:Banque:CompteCourant")
        .expect("solde CC");
    assert_eq!(
        cc.amounts.get("EUR").copied(),
        Some("6877.39".parse().unwrap())
    );

    // Payees indexables pour l'autocomplétion.
    let payees = snapshot.payees();
    assert!(!payees.is_empty(), "des payees sont indexés");
}
