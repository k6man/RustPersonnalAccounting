//! CLI `household-cli` : validation et soldes du ledger.
//!
//! Rôle : repli canonique du hook `pre-commit` (quand `rledger` n'est pas
//! installé) et artefact CI. Usage :
//!
//! ```text
//! household-cli check <main.beancount>     # exit 0 valide, 1 invalide, 2 usage
//! household-cli balances <main.beancount> [--date=YYYA-MM-JJ]
//! ```

use std::path::{Path, PathBuf};

use household_core::{Snapshot, Through};
use rustledger_core::NaiveDate;

const USAGE: &str = "usage: household-cli check <main.beancount>\n       household-cli balances <main.beancount> [--date=AAAA-MM-JJ]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(run(&args));
}

fn run(args: &[String]) -> i32 {
    let mut iter = args.iter();
    let Some(command) = iter.next().map(String::as_str) else {
        eprintln!("{USAGE}");
        return 2;
    };
    let Some(path) = iter.next().map(|s| PathBuf::from(s.as_str())) else {
        eprintln!("{USAGE}");
        return 2;
    };

    match command {
        "check" => check(&path),
        "balances" => balances(&path, iter.next().map(|s| s.as_str())),
        _ => {
            eprintln!("{USAGE}");
            2
        }
    }
}

/// Validation complète : exit 0 si zéro erreur, 1 sinon (une erreur de
/// *chargement fatal* — include manquant — compte comme invalide).
fn check(main: &Path) -> i32 {
    match Snapshot::load(main) {
        Ok(snapshot) => {
            let errors = snapshot.errors();
            if errors.is_empty() {
                println!("✓ ledger valide : {}", main.display());
                0
            } else {
                for error in errors {
                    let where_clause = error
                        .location
                        .as_ref()
                        .map(|l| format!(" ({}:{})", l.file.display(), l.line))
                        .unwrap_or_default();
                    eprintln!(
                        "[{} {}] {}{}",
                        error.code, error.phase, error.message, where_clause
                    );
                }
                eprintln!("✗ {} erreur(s) de validation", errors.len());
                1
            }
        }
        Err(load) => {
            eprintln!("✗ chargement impossible : {load}");
            1
        }
    }
}

/// Soldes finaux (par compte, par devise), au jour donné (compris) ou à
/// la toute fin du ledger. Format prêt pour la console et les scripts.
fn balances(main: &Path, date: Option<&str>) -> i32 {
    let Ok(snapshot) = Snapshot::load(main) else {
        eprintln!("✗ chargement impossible");
        return 1;
    };
    // Accepte `--date=AAAA-MM-JJ` ou une date ISO nue.
    let raw = date.map(|d| d.strip_prefix("--date=").unwrap_or(d));
    let balances = match raw {
        Some(raw) => match parse_date(raw) {
            Some(day) => snapshot.balances_at(day, Through::EndOfDay),
            None => {
                eprintln!("✗ date invalide : {raw} (attendu AAAA-MM-JJ)");
                return 2;
            }
        },
        None => snapshot.balances_at_end(),
    };
    match balances {
        Ok(rows) => {
            for row in &rows {
                for (currency, amount) in &row.amounts {
                    println!("{account:44} {currency:5} {amount}", account = row.account);
                }
            }
            0
        }
        Err(balance_failure) => {
            eprintln!("✗ calcul des soldes impossible : {balance_failure}");
            1
        }
    }
}

fn parse_date(raw: &str) -> Option<NaiveDate> {
    raw.parse().ok()
}
