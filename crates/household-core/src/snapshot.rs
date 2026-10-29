//! [`Snapshot`] : une vue immuable du ledger chargé, prête à consommer par la GUI.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use rust_decimal::Decimal;
use rustledger_booking::BookingEngine;
use rustledger_core::{Directive, Spanned};
use rustledger_loader::{Ledger, LedgerError};
use rustledger_parser::Span;

use crate::error::CoreError;

/// Localisation d'une directive dans les fichiers sources du ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
    /// Chemin du fichier contenant la directive.
    pub file: PathBuf,
    /// Ligne 1-based de son début.
    pub line: usize,
}

/// Un plafond modélisant le plafond réglementaire d'un compte d'épargne,
/// lu des métadonnées `ceiling:` de la directive `open`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ceiling {
    pub account: String,
    pub amount: Decimal,
    pub currency: String,
}

/// Un compte ouvert du ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInfo {
    pub account: String,
    pub date: rustledger_core::NaiveDate,
    /// Devises autorisées (vide = n'importe laquelle).
    pub currencies: Vec<String>,
    pub ceiling: Option<Ceiling>,
}

/// Solde d'un compte, agrégé par devise. EUR seul en pratique ; les titres
/// (positions avec `cost`) ne rentrent pas dans ces totaux simples.
#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub account: String,
    /// Somme des positions par devise, triées.
    pub amounts: BTreeMap<String, Decimal>,
}

/// Vue complète et immuable du ledger.
///
/// La frontière GUI : construit une fois (thread worker), partagé en
/// `Arc<Snapshot>` ; toute écriture re-produit un [`Snapshot`] neuf.
/// [`Ledger`] est détenu directement : `directives`, `source_map` et
/// `errors` sont déjà immuables — pas de copie inutile.
#[derive(Debug)]
pub struct Snapshot {
    /// Chemin du `main.beancount` chargé.
    pub main: PathBuf,
    ledger: Ledger,
}

impl Snapshot {
    /// Charge **tout le ledger** (includes + plugins + booking + validation)
    /// pour un `main.beancount`. Les erreurs de validation restent inspectables
    /// via [`Self::errors`] : un ledger avec une erreur de solde se charge
    /// quand même (la GUI en montre la bannière).
    pub fn load(main: &Path) -> Result<Self, CoreError> {
        let options = rustledger_loader::LoadOptions {
            // Path security empêche l'include hors du répertoire racine :
            // le ledger du foyer est sous une seule arborescence.
            path_security: false,
            ..rustledger_loader::LoadOptions::default()
        };
        let ledger = rustledger_loader::load(main, &options)?;
        Ok(Self {
            main: main.to_path_buf(),
            ledger,
        })
    }

    /// Assembly depuis un [`Ledger`] déjà chargé (réutilisé par le writer).
    #[must_use]
    pub fn from_ledger(main: PathBuf, ledger: Ledger) -> Self {
        Self { main, ledger }
    }

    /// Le main chargé (options, directives brutes, source map).
    #[must_use]
    pub const fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// Toutes les erreurs de validation collectées (codes E/W, avec source).
    #[must_use]
    pub fn errors(&self) -> &[LedgerError] {
        &self.ledger.errors
    }

    /// Ledger sans erreur fatale de validation (assertions, comptes fermés…).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.ledger.errors.is_empty()
    }

    /// Toutes les directives, ordre du ledger (triées par date, réservées par
    /// le pipeline).
    #[must_use]
    pub fn entries(&self) -> &[Spanned<Directive>] {
        &self.ledger.directives
    }

    /// Localisation d'une directive : (fichier absolu, ligne 1-based).
    /// La source map conserve les chemins tels qu'inclus (souvent relatifs
    /// au répertoire du ledger) — on les résout ici.
    pub fn source_of(&self, index: usize) -> Result<SourceRef, CoreError> {
        let spanned = self
            .entries()
            .get(index)
            .ok_or(CoreError::NoSource(index))?;
        let file = self
            .ledger
            .source_map
            .get(spanned.file_id as usize)
            .ok_or(CoreError::NoSource(index))?;
        let (line, _col) = file.line_col(spanned.span.start);
        Ok(SourceRef {
            file: self.resolve_source(&file.path),
            line,
        })
    }

    /// Résolution d'un chemin de la source map : absolu tel quel, relatif
    /// interprété par rapport au répertoire du ledger.
    #[must_use]
    pub fn resolve_source(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.main
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(path)
        }
    }

    /// Span d'octets exacte d'une directive dans son fichier — la base de
    /// l'édition chirurgicale ([`crate::writer`]).
    pub fn span_of(&self, index: usize) -> Result<(usize, Span), CoreError> {
        let spanned = self
            .entries()
            .get(index)
            .ok_or(CoreError::NoSource(index))?;
        Ok((spanned.file_id as usize, spanned.span))
    }

    /// Index des transactions, ordre du ledger.
    #[must_use]
    pub fn transactions(&self) -> Vec<usize> {
        self.entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.value, Directive::Transaction(_)))
            .map(|(i, _)| i)
            .collect()
    }

    /// La transaction à l'index d'entrée donné, s'il y en a une.
    #[must_use]
    pub fn transaction(&self, index: usize) -> Option<&rustledger_core::Transaction> {
        match self.entries().get(index).map(|e| &e.value) {
            Some(Directive::Transaction(txn)) => Some(txn),
            _ => None,
        }
    }

    /// Payees rencontrés, triés par fréquence puis alphabétique (autocomplétion).
    #[must_use]
    pub fn payees(&self) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for txn in self.entries().iter().filter_map(|e| match &e.value {
            Directive::Transaction(t) => Some(t),
            _ => None,
        }) {
            if let Some(payee) = txn.payee.as_ref() {
                let payee = payee.as_str().to_string();
                *counts.entry(payee).or_default() += 1;
            }
        }
        let mut list: Vec<(String, usize)> = counts.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    }

    /// Comptes ouverts (avec plafonds éventuels), triés par nom.
    #[must_use]
    pub fn accounts(&self) -> Vec<AccountInfo> {
        let mut out: Vec<AccountInfo> = Vec::new();
        for open in self.entries().iter().filter_map(|e| match &e.value {
            Directive::Open(o) => Some(o),
            _ => None,
        }) {
            out.push(AccountInfo {
                account: open.account.as_str().to_string(),
                date: open.date,
                currencies: open
                    .currencies
                    .iter()
                    .map(|c| c.as_str().to_string())
                    .collect(),
                ceiling: extract_ceiling(open.account.as_str(), &open.meta),
            });
        }
        out.sort_unstable_by(|a, b| a.account.cmp(&b.account));
        out
    }

    /// Plafonds déclarés (depuis les directives `open`), index par compte.
    #[must_use]
    pub fn ceilings(&self) -> BTreeMap<String, Ceiling> {
        self.accounts()
            .into_iter()
            .filter_map(|a| a.ceiling.map(|c| (a.account, c)))
            .collect()
    }

    /// Soldes par compte, à une échelle de date.
    ///
    /// Sémantique beancount v3 : une assertion `balance` à la date D est
    /// vérifiée à 00:00 de D (strictement avant les transactions de D).
    pub fn balances_at(
        &self,
        day: rustledger_core::NaiveDate,
        through: Through,
    ) -> Result<Vec<AccountBalance>, CoreError> {
        let view = self.ledger.balance_view();
        // En 0.24.0, `options.booking_method` est la chaîne d'option beancount
        // ("STRICT", "FIFO"…). Le loader a déjà bouqué une fois ; on rejoue la
        // même méthode sur la vue pure pour agréger les soldes.
        let method =
            rustledger_core::BookingMethod::from_str(self.ledger.options.booking_method.as_str())
                .unwrap_or(rustledger_core::BookingMethod::Strict);
        let mut engine = BookingEngine::with_method(method);
        engine.register_account_methods(view.iter());
        for directive in &view {
            if let Directive::Transaction(txn) = directive {
                let included = match through {
                    Through::StartOfDay => txn.date < day,
                    Through::EndOfDay => txn.date <= day,
                };
                if included {
                    engine.apply(txn)?;
                }
            }
        }
        let mut out: Vec<AccountBalance> = engine
            .into_inventories()
            .into_iter()
            .map(|(account, inventory)| AccountBalance {
                account: account.as_str().to_string(),
                amounts: sum_positions(&inventory),
            })
            .filter(|b| !b.amounts.is_empty())
            .collect();
        out.sort_unstable_by(|a, b| a.account.cmp(&b.account));
        Ok(out)
    }

    /// Solde d'un compte précis (totaux par devise) à l'échelle de date donnée.
    pub fn balance_of(
        &self,
        account: &str,
        day: rustledger_core::NaiveDate,
        through: Through,
    ) -> Result<Option<BTreeMap<String, Decimal>>, CoreError> {
        for b in self.balances_at(day, through)? {
            if b.account == account {
                return Ok(if b.amounts.values().all(Decimal::is_zero) {
                    None
                } else {
                    Some(b.amounts)
                });
            }
        }
        Ok(None)
    }

    /// Soldes à la toute fin du ledger (après le dernier jour transactionné).
    pub fn balances_at_end(&self) -> Result<Vec<AccountBalance>, CoreError> {
        match self.last_transaction_date() {
            Some(day) => self.balances_at(day, Through::EndOfDay),
            None => Ok(Vec::new()),
        }
    }

    /// Dernière date de transaction du ledger.
    #[must_use]
    pub fn last_transaction_date(&self) -> Option<rustledger_core::NaiveDate> {
        self.entries()
            .iter()
            .filter_map(|e| match &e.value {
                Directive::Transaction(txn) => Some(txn.date),
                _ => None,
            })
            .max()
    }
}

/// Fenêtre d'affichage des soldes relative à une date (sémantique v3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Through {
    /// État à 00:00 de la date : transactions strictement antérieures.
    StartOfDay,
    /// État à la fin de la date : transactions du jour incluses.
    EndOfDay,
}

/// Somme des positions simples d'un inventaire par devise.
fn sum_positions(inventory: &rustledger_core::Inventory) -> BTreeMap<String, Decimal> {
    let mut out: BTreeMap<String, Decimal> = BTreeMap::new();
    for position in inventory.positions() {
        let units = &position.units;
        *out.entry(units.currency.as_str().to_string()).or_default() += units.number;
    }
    out
}

/// Lecture de `ceiling: <montant>` des métadonnées d'un `open`.
fn extract_ceiling(account: &str, meta: &rustledger_core::Metadata) -> Option<Ceiling> {
    match meta.get("ceiling") {
        Some(rustledger_core::MetaValue::Amount(amount)) => Some(Ceiling {
            account: account.to_string(),
            amount: amount.number,
            currency: amount.currency.as_str().to_string(),
        }),
        Some(rustledger_core::MetaValue::Number(number)) => Some(Ceiling {
            account: account.to_string(),
            amount: *number,
            currency: "EUR".to_string(),
        }),
        _ => None,
    }
}

// Tests de smoke du chargement (le golden complet vit dans tests/golden.rs).
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use rust_decimal_macros::dec;

    #[test]
    fn sample_ledger_loads_and_reports_balances() {
        let main = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ledger/main.beancount");
        if !main.exists() {
            eprintln!("ledger absent de la sandbox, test ignoré");
            return;
        }
        let snapshot = Snapshot::load(&main).expect("chargement");
        assert!(snapshot.is_valid(), "erreurs : {:?}", snapshot.errors());
        // 2026-04-01 00:00 (sémantique v3) = fin mars : CC = 6877.39.
        let balances = snapshot
            .balances_at("2026-04-01".parse().unwrap(), Through::StartOfDay)
            .unwrap();
        let cc = balances
            .iter()
            .find(|b| b.account == "Assets:Banque:CompteCourant");
        assert_eq!(
            cc.map(|b| b.amounts.get("EUR").copied()),
            Some(Some(dec!(6877.39)))
        );
    }
}
