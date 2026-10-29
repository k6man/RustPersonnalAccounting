//! Suite d'écriture : append / replace / remove sur une **copie scratch** du
//! ledger échantillon. Garanties vérifiées : ordre chronologique des blocs,
//! création des fichiers mensuels, soldes après mutation, refus des écritures
//! invalides (pré-vérification, fichier intouchable).

mod common;

use household_core::{
    Snapshot, Through, append_transaction, monthly_file, remove_transaction, replace_transaction,
};
use rust_decimal_macros::dec;
use rustledger_core::{Amount, NaiveDate, Posting, Transaction};

use common::ledger_copy;

/// Courses d'exemple : Restaurants +X EUR / CompteCourant -X EUR, datées
/// d'avril (après le contrôle `balance` du 2026-04-01, donc sans casser les
/// assertions du ledger — sauf dans le test dédié).
fn courses(day: &str, amount: impl Into<rust_decimal::Decimal>, narration: &str) -> Transaction {
    let amount = amount.into();
    Transaction::new(day.parse().expect("date ISO"), narration)
        .with_flag('*')
        .with_payee("Carrefour City")
        .with_synthesized_posting(Posting::new(
            "Expenses:Alimentation:Restaurants",
            Amount::new(amount, "EUR"),
        ))
        .with_synthesized_posting(Posting::new(
            "Assets:Banque:CompteCourant",
            Amount::new(-amount, "EUR"),
        ))
}

#[test]
fn append_replace_remove_boucle_complete() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let base = Snapshot::load(&main).expect("copie valide");

    // --- Append le 2026-04-10 : créé le fichier mensuel d'avril ---------
    let first = courses("2026-04-10", dec!(25.00), "Courses d'avril");
    let after_append = append_transaction(&base, &first).expect("append valide");
    assert!(
        after_append.is_valid(),
        "revalidation : {:?}",
        after_append.errors()
    );
    let cc = |s: &Snapshot| {
        s.balance_of(
            "Assets:Banque:CompteCourant",
            "2026-04-10".parse().unwrap(),
            Through::EndOfDay,
        )
        .unwrap()
    };
    assert_eq!(
        cc(&after_append).and_then(|m| m.get("EUR").copied()),
        Some(dec!(6852.39)),
        "CC : 6877.39 - 25.00"
    );
    assert!(common::monthly(&main, 2026, 4).contains("Courses d'avril"));

    // --- Remplacement sur place (même date) -----------------------------
    let correction = courses("2026-04-10", dec!(50.00), "Courses d'avril corrigées");
    let index = index_of(&after_append, first.date).expect("index de la transaction ajoutée");
    let after_replace =
        replace_transaction(&after_append, index, &correction).expect("remplacement même date");
    assert_eq!(
        cc(&after_replace).and_then(|m| m.get("EUR").copied()),
        Some(dec!(6827.39))
    );

    // --- Remove : retour au solde de contrôle ----------------------------
    let index = index_of(&after_replace, first.date).expect("index toujours présent");
    let after_remove = remove_transaction(&after_replace, index).expect("suppression");
    assert!(after_remove.is_valid());
    assert_eq!(
        cc(&after_remove),
        Some(
            [("EUR", dec!(6877.39))]
                .into_iter()
                .map(|(c, v)| (c.to_string(), v))
                .collect()
        ),
        "retour au solde de contrôle du 2026-04-01"
    );
    assert!(!common::monthly(&main, 2026, 4).contains("Carrefour"));

    // (le scratch est supprimé en fin de test)
    common::cleanup(&scratch);
}

#[test]
fn insertion_en_ordre_chronologique() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let base = Snapshot::load(&main).expect("copie valide");

    let mid = append_transaction(&base, &courses("2026-04-15", dec!(10.00), "Paiement du 15"))
        .expect("insertion valide");
    let later =
        append_transaction(&mid, &courses("2026-04-20", dec!(5.00), "Paiement du 20")).expect("ok");
    let early = append_transaction(&later, &courses("2026-04-05", dec!(7.50), "Paiement du 05"))
        .expect("ok");
    assert!(early.is_valid());

    let text = common::monthly(&main, 2026, 4);
    let pos_05 = text.find("Paiement du 05").expect("bloc du 05 présent");
    let pos_15 = text.find("Paiement du 15").expect("bloc du 15 présent");
    let pos_20 = text.find("Paiement du 20").expect("bloc du 20 présent");
    assert!(pos_05 < pos_15 && pos_15 < pos_20, "blocs triés par date");

    // Toute ligne `2026-MM-JJ` en tête de directive est non décroissante.
    let mut last = "0000-00-00".to_string();
    for line in text.lines().filter(|l| l.starts_with("2026-")) {
        assert!(
            &line[..10] >= last.as_str(),
            "{:?} arrive après {:?}",
            &line[..10],
            last
        );
        last = line[..10].to_string();
    }

    common::cleanup(&scratch);
}

#[test]
fn vue_transaction_desequilibree_refusee_et_fichier_intact() {
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let base = Snapshot::load(&main).expect("copie valide");

    let mut broken = courses("2026-04-25", dec!(100.00), "Fausse transaction");
    if let Some(posting) = broken.postings.first_mut()
        && let Some(rustledger_core::IncompleteAmount::Complete(units)) = posting.units.as_mut()
    {
        units.number = dec!(99.99);
    }

    let failure = append_transaction(&base, &broken);
    assert!(failure.is_err(), "l'écriture déséquilibrée est refusée");

    // Le fichier mensuel d'avril n'existait pas et n'a pas été créé.
    let target = monthly_file(&main, "2026-04-25".parse().unwrap());
    assert!(!target.exists(), "fichier intouché en cas d'échec");

    common::cleanup(&scratch);
}

#[test]
fn casser_une_assertion_balance_est_refuse() {
    // Une mutation datée AVANT le contrôle du 2026-04-01 casse l'assertion
    // du ledger : la pré-vérification doit la refuser, même si l'écriture
    // est parfaitement équilibrée.
    let main = ledger_copy();
    let scratch = main.parent().unwrap().parent().unwrap().to_path_buf();
    let base = Snapshot::load(&main).expect("copie valide");

    let mars = courses("2026-03-20", dec!(10.00), "Écriture historique");
    assert!(
        append_transaction(&base, &mars).is_err(),
        "casser une assertion balance est refusé"
    );

    common::cleanup(&scratch);
}

/// Index (dans l'ordre des entrées) de la première transaction datée `date`.
fn index_of(snapshot: &Snapshot, date: NaiveDate) -> Option<usize> {
    snapshot
        .entries()
        .iter()
        .enumerate()
        .find(|(_, e)| {
            matches!(
                e.value,
                rustledger_core::Directive::Transaction(ref t) if t.date == date
            )
        })
        .map(|(i, _)| i)
}
