//! Duplication d'année budgétaire N → N+1 (module pur).
//!
//! Recopie les directives `custom "budget"` de l'année source vers l'année
//! cible (la supersession étant datée, la recopie est une opération d'édition
//! — cf. TODO.md Q4), avec :
//! - ajustement optionnel en pour cent (peut être négatif) ;
//! - arrondi **au multiple supérieur** d'un pas « rond » paramétrable
//!   (`0.01` = centimes exacts, `1.00` = euros entiers, `5.00` = multiple
//!   de 5 €…) ;
//! - sortie = liste de directives canoniques triées par (date, compte),
//!   prête pour le writer.

use rust_decimal::Decimal;
use thiserror::Error;

use rustledger_budget::{Budgets, Interval};
use rustledger_core::{Amount, Directive, MetaValue};

/// Erreurs de duplication.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DuplicationError {
    /// Le pas d'arrondi doit être strictement positif.
    #[error("pas d'arrondi invalide : {0} (attendu > 0)")]
    InvalidStep(String),
    /// Aucune directive de budget pour l'année source.
    #[error("aucun budget trouvé pour {0}")]
    EmptySourceYear(i16),
    /// L'année cible est déjà budgétée (écrasement silencieux refusé) ou le
    /// décalage de date est impossible (bornes du calendrier atteintes).
    #[error("{0}")]
    Conflict(String),
}

/// Duplique l'année `source_year` vers `target_year`.
///
/// `percent` est un pour cent (`6` = +6 %, `-10` = −10 %, `0` = copie exacte,
/// opération idempotente). `step` est le pas d'arrondi supérieur en unités de
/// devise (voir le module). L'année cible ne doit pas déjà exister dans les
/// directives (sinon duplication en double — refusé).
pub fn duplicate_year(
    directives: &[Directive],
    source_year: i16,
    target_year: i16,
    percent: Decimal,
    step: Decimal,
) -> Result<Vec<Directive>, DuplicationError> {
    if step <= Decimal::ZERO {
        return Err(DuplicationError::InvalidStep(step.to_string()));
    }

    let budgets = Budgets::from_directives(directives);
    let source_entries: Vec<_> = budgets
        .entries()
        .filter(|entry| entry.from.year() == source_year)
        .collect();
    if source_entries.is_empty() {
        return Err(DuplicationError::EmptySourceYear(source_year));
    }

    let shift = jiff::Span::new().years(i64::from(target_year) - i64::from(source_year));
    let factor = Decimal::ONE + percent / Decimal::from(100);

    let mut out: Vec<Directive> = Vec::with_capacity(source_entries.len());
    for entry in source_entries {
        // Garde : la directive dupliquée ne doit pas exister (année cible
        // déjà budgétée → refus, pas d'écrasement silencieux).
        let key = (entry.account.as_str(), entry.currency.as_str());
        let exists = budgets.entries().any(|e| {
            e.from.year() == target_year && (e.account.as_str(), e.currency.as_str()) == key
        });
        if exists {
            return Err(DuplicationError::Conflict(format!(
                "budget {} ({}) déjà présent pour {target_year}",
                entry.account.as_str(),
                entry.currency.as_str()
            )));
        }

        let new_date = entry
            .from
            .checked_add(shift)
            .map_err(|_| DuplicationError::Conflict(entry.from.to_string()))?;

        let scaled = entry.amount * factor;
        // Arrondi AU MULTIPLE SUPÉRIEUR du pas, puis remise à l'échelle
        // monétaire canonique (2 décimales, ou celle du pas si plus fine).
        let rounded = ((scaled / step).ceil() * step).round_dp(step.scale().max(2));

        out.push(Directive::Custom(
            rustledger_core::Custom::new(new_date, "budget")
                .with_value(MetaValue::Account(entry.account.clone()))
                .with_value(MetaValue::String(
                    interval_keyword(entry.interval).to_string(),
                ))
                .with_value(MetaValue::Amount(Amount::new(
                    rounded,
                    entry.currency.clone(),
                ))),
        ));
    }

    // Tri (date, compte) — l'ordre canonique du fichier.
    out.sort_unstable_by(|a, b| match (a, b) {
        (Directive::Custom(x), Directive::Custom(y)) => {
            let account_of = |c: &rustledger_core::Custom| match c.values.first() {
                Some(MetaValue::Account(account)) => account.as_str().to_string(),
                _ => String::new(),
            };
            x.date
                .cmp(&y.date)
                .then_with(|| account_of(x).cmp(&account_of(y)))
        }
        _ => std::cmp::Ordering::Equal,
    });
    Ok(out)
}

/// Le mot-clé d'intervalle canonique (celui écrit dans les fichiers).
#[must_use]
pub fn interval_keyword(interval: Interval) -> &'static str {
    match interval {
        Interval::Day => "daily",
        Interval::Week => "weekly",
        Interval::Month => "monthly",
        Interval::Quarter => "quarterly",
        Interval::Year => "yearly",
    }
}
