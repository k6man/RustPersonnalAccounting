//! Formatage des nombres pour l'affichage (pur, testé).
//!
//! Convention française : virgule décimale, espace fine insécable pour les
//! milliers. Le signe reste `-` (sémantique beancount : les comptes
//! crédit-normés — Income, Liabilities — sont négatifs).

use rust_decimal::Decimal;

const ESPACE_FINE: char = '\u{202f}';

/// Formate un montant décimal « à la française » (`6877.39` → `6 877,39`,
/// échelle préservée : `850.00` → `850,00`).
#[must_use]
pub fn nombre(montant: Decimal) -> String {
    let negatif = montant.is_sign_negative();
    let brut = montant.abs().to_string();
    let (entiere, decimales) = match brut.split_once('.') {
        Some((entiere, decimales)) => (entiere, Some(decimales)),
        None => (brut.as_str(), None),
    };

    let chiffres: Vec<u8> = entiere.bytes().collect();
    let mut groupee = String::with_capacity(entiere.len() + 4);
    for (index, chiffre) in chiffres.iter().enumerate() {
        if index > 0 && (chiffres.len() - index).is_multiple_of(3) {
            groupee.push(ESPACE_FINE);
        }
        groupee.push(*chiffre as char);
    }

    let signe = if negatif { "-" } else { "" };
    match decimales {
        Some(decimales) => format!("{signe}{groupee},{decimales}"),
        None => format!("{signe}{groupee}"),
    }
}

/// Montant EUR affiché (`nombre` + « € »).
#[must_use]
pub fn eur(montant: Decimal) -> String {
    format!("{} €", nombre(montant))
}

/// Pourcentage à une décimale (`0.7523` → « 75,2 % »).
#[must_use]
pub fn pourcent(part: f64) -> String {
    let arrondi = ((part * 1000.0).round() / 10.0).abs();
    let entier = arrondi as i64;
    let dixieme = ((arrondi - entier as f64) * 10.0).round() as i64;
    format!("{entier},{dixieme} %")
}
