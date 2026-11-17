//! Écriture chirurgicale de `budgets/AAAA.beancount` (au-dessus du writer de
//! `household-core`, qui fournit verrou / pré-vérification scratch /
//! atomicité / revalidation).
//!
//! Conventions :
//! - une directive = **une ligne** ; une mise à jour de cellule remplace
//!   exactement cette ligne (span source), les commentaires et l'alignement
//!   du reste du fichier sont préservés ;
//! - une directive nouvelle est insérée à sa place (date, compte) ;
//! - une nouvelle année : création de `budgets/AAAA.beancount` **et** ajout
//!   du `include` dans `main.beancount` en **une seule mutation** (les deux
//!   fichiers sont pré-vérifiés ensemble) ;
//! - `main.beancount` n'est par ailleurs jamais réécrit : les transactions
//!   sont en globs (Q7) mais les budgets sont inclus explicitement — la
//!   seule édition légitime de `main` est l'ajout d'un include d'année.

use std::path::Path;

use rust_decimal::Decimal;
use rustledger_budget::Interval;
use rustledger_core::{Amount, Directive, MetaValue, NaiveDate};

use household_core::CoreError;
use household_core::Snapshot;
use household_core::writer::{
    canonical_directive_block, edit_ledger_file, edit_ledger_files, insert_sorted,
};

use crate::duplicate::interval_keyword;

/// Chemin du fichier budgétaire d'une année : `…/ledger/budgets/YYYY.beancount`.
#[must_use]
pub fn budget_file(main: &Path, year: i16) -> std::path::PathBuf {
    main.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("budgets")
        .join(format!("{year}.beancount"))
}

/// Met à jour la cellule (date, compte) d'une année : la directive
/// `custom "budget"` existante est réécrite sur place (même intervalle,
/// nouveau montant) ; si elle n'existe pas, elle est insérée (`monthly`).
pub fn set_cell(
    snapshot: &Snapshot,
    year: i16,
    date: NaiveDate,
    account: &str,
    amount: Decimal,
) -> Result<Snapshot, CoreError> {
    let main = snapshot.main.clone();

    // Localise la directive existante (date + compte).
    let found: Option<usize> = snapshot
        .entries()
        .iter()
        .position(|entry| {
            matches!(&entry.value, Directive::Custom(custom)
                if custom.custom_type == "budget"
                    && custom.date == date
                    && matches!(custom.values.first(), Some(MetaValue::Account(name)) if name.as_str() == account))
        });

    if let Some(index) = found {
        // Édition ultra-chirurgicale : seul le token montant de la ligne est
        // remplacé — l'alignement et les octets du reste du fichier (et de la
        // ligne elle-même) sont préservés à l'identique.
        let (_file_id, span) = snapshot.span_of(index)?;
        let source = snapshot.source_of(index)?;
        edit_ledger_file(&main, source.file, move |old| {
            let (start, body, after) = household_core::writer::split_directive_block(old, span);
            let new_line = replace_amount_in_line(body, amount).ok_or_else(|| {
                CoreError::Writer(format!(
                    "ligne budget non reconnue, édition refusée : {body:?}"
                ))
            })?;
            Ok(Some(format!("{}{new_line}{after}", &old[..start])))
        })
    } else {
        // Insertion (nouvelle directive `monthly`) dans le fichier de l'année.
        let custom = budget_directive(date, account, "monthly", amount);
        insert_budget_directive(&main, year, custom)
    }
}

/// Remplace le montant (avant-dernier token, validé numérique) d'une ligne
/// budget, en préservant tous les autres octets (alignement inclus).
/// Retourne la ligne avec son `\n` final.
fn replace_amount_in_line(line: &str, new_amount: Decimal) -> Option<String> {
    let trimmed = line.trim_end_matches(['\n', '\r']);
    if !trimmed.contains("custom \"budget\"") {
        return None;
    }
    // Tokens : plages d'octets des runs non-blancs.
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (index, ch) in trimmed.char_indices() {
        if ch.is_whitespace() {
            if let Some(s) = start.take() {
                tokens.push((s, index));
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(s) = start.take() {
        tokens.push((s, trimmed.len()));
    }
    if tokens.len() < 2 {
        return None;
    }
    let currency_token = tokens.last()?;
    let number_token = tokens.get(tokens.len() - 2)?;
    // Validation : l'avant-dernier token est un montant, le dernier une devise.
    trimmed[number_token.0..number_token.1]
        .parse::<Decimal>()
        .ok()?;
    if !trimmed[currency_token.0..currency_token.1]
        .chars()
        .all(|c| c.is_ascii_uppercase())
    {
        return None;
    }

    let mut out = String::with_capacity(trimmed.len() + 8);
    out.push_str(&trimmed[..number_token.0]);
    out.push_str(&new_amount.to_string());
    out.push_str(&trimmed[number_token.1..]);
    out.push('\n');
    Some(out)
}

/// Écrit le commentaire d'une ligne de grille (compte) dans le fichier
/// budget de l'année : une ligne `; note` **immédiatement au-dessus** de la
/// première directive `custom "budget"` du compte (cf. [`crate::commentaires`]
/// pour la convention de lecture). Un commentaire vide (après nettoyage)
/// supprime le bloc existant ; le reste du fichier est préservé à l'octet.
/// Refuse si le compte n'a aucune directive dans le fichier de l'année.
///
/// Le snapshot doit être **à jour du fichier** : les spans qu'il porte sont
/// des offsets d'octets, toute écriture simultanée depuis un snapshot plus
/// ancien corromprait le patch — rechargez le snapshot après chaque
/// écriture (c'est ce que fait le worker GUI à chaque commande).
pub fn set_comment(
    snapshot: &Snapshot,
    year: i16,
    account: &str,
    comment: &str,
) -> Result<Snapshot, CoreError> {
    let main = snapshot.main.clone();
    let note = crate::commentaires::nettoie(comment);

    // Première directive budget du compte dans l'année : c'est à elle que la
    // ligne de grille correspond, et c'est là que le commentaire s'attache.
    let found = snapshot
        .entries()
        .iter()
        .position(|entry| {
            matches!(&entry.value, Directive::Custom(custom)
                if custom.custom_type == "budget"
                    && custom.date.year() == year
                    && matches!(custom.values.first(), Some(MetaValue::Account(name)) if name.as_str() == account))
        })
        .ok_or_else(|| {
            CoreError::Writer(format!(
                "aucune directive budget de {account} en {year} : commentaire refusé"
            ))
        })?;

    let (_file_id, span) = snapshot.span_of(found)?;
    let source = snapshot.source_of(found)?;
    // La directive visée doit vivre dans le fichier de l'année demandée
    // (comparaison canonique : la copie scratch a des chemins absolus, le
    // résolu aussi).
    let attendu = budget_file(&main, year);
    let meme_fichier = match (
        std::fs::canonicalize(&source.file),
        std::fs::canonicalize(&attendu),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => source.file == attendu,
    };
    if !meme_fichier {
        return Err(CoreError::Writer(format!(
            "directive budget de {account} en {year} hors du fichier de l'année ({})",
            source.file.display()
        )));
    }

    edit_ledger_file(&main, source.file, move |old| {
        let (start, _body, _after) = household_core::writer::split_directive_block(old, span);
        // Bloc de lignes `;` contiguës immédiatement au-dessus de la
        // directive : un blanc ou un `#` casse le rattachement (même
        // convention que la lecture).
        let mut bloc_debut = start;
        let mut cursor = start;
        while cursor > 0 {
            let nl = old[..cursor - 1].rfind('\n').map_or(0, |i| i + 1);
            let ligne = old[nl..cursor - 1].trim_end();
            if ligne.trim_start().starts_with(';') {
                bloc_debut = nl;
                cursor = nl;
            } else {
                break;
            }
        }
        let bloc_actuel = &old[bloc_debut..start];
        if note.is_empty() {
            if bloc_actuel.is_empty() {
                // Supprimer un commentaire absent : no-op (la revalidation
                // du writer reste utile, on ne court-circuite pas).
                return Ok(Some(old.to_string()));
            }
            return Ok(Some(format!("{}{}", &old[..bloc_debut], &old[start..])));
        }
        let bloc = format!("; {note}\n");
        if bloc_actuel == bloc {
            return Ok(Some(old.to_string()));
        }
        Ok(Some(format!(
            "{}{}{}",
            &old[..bloc_debut],
            bloc,
            &old[start..]
        )))
    })
}

/// Génère le fichier d'une année complète (duplication) : crée
/// `budgets/YYYY.beancount` avec les directives fournies **et** l'include
/// dans `main.beancount`, en une mutation unique. Refuse d'écraser un
/// fichier existant (les éditions manuelles seraient perdues).
pub fn write_year(
    snapshot: &Snapshot,
    year: i16,
    directives: &[Directive],
) -> Result<Snapshot, CoreError> {
    let main = snapshot.main.clone();
    let target = budget_file(&main, year);
    if target.exists() {
        return Err(CoreError::Writer(format!(
            "refus d'écraser {} : supprimez-le d'abord (ou éditez-le)",
            target.display()
        )));
    }
    if directives.is_empty() {
        return Err(CoreError::Writer(
            "aucune directive de budget à écrire".to_string(),
        ));
    }

    let mut content = String::from(concat!(
        "# Budget annuel — SEULE SOURCE DE VÉRITÉ budgétaire du foyer.\n",
        "# Généré par household (duplication d'année) ; une directive = une ligne,\n",
        "# édition chirurgicale par la GUI. `0.00 EUR` = mois sans ce poste.\n"
    ));
    // Format canonique du lot entier (alignement des montants cohérent sur
    // toutes les lignes — même moteur que `rledger format`).
    content.push_str(&rustledger_core::format::format_directives(
        directives.iter(),
        &rustledger_core::format::FormatConfig::default(),
    ));

    let include_line = format!("include \"budgets/{year}.beancount\"");
    let year_for_closure = year;
    let mut changes = vec![household_core::writer::FileChange::new(
        target.clone(),
        move |_| Ok(Some(content.clone())),
    )];
    changes.push(household_core::writer::FileChange::new(
        main.clone(),
        move |old| Ok(Some(ensure_include(old, year_for_closure, &include_line))),
    ));
    edit_ledger_files(&main, changes)
}

// ------------------------------------------------------------------ helpers ---

/// Directive `custom "budget"` canonique.
#[must_use]
pub fn budget_directive(
    date: NaiveDate,
    account: &str,
    interval: &str,
    amount: Decimal,
) -> Directive {
    Directive::Custom(
        rustledger_core::Custom::new(date, "budget")
            .with_value(MetaValue::Account(rustledger_core::Account::from(account)))
            .with_value(MetaValue::String(interval.to_string()))
            .with_value(MetaValue::Amount(Amount::new(
                amount,
                rustledger_core::Currency::from("EUR"),
            ))),
    )
}

/// Insère une directive budget (création du fichier de l'année + include si
/// nécessaire).
fn insert_budget_directive(
    main: &Path,
    year: i16,
    directive: Directive,
) -> Result<Snapshot, CoreError> {
    let target = budget_file(main, year);
    let block = canonical_directive_block(&directive);
    let iso = directive_date_iso(&directive).unwrap_or_default();

    if target.exists() {
        edit_ledger_file(main, target, move |old| {
            Ok(Some(insert_sorted(old, &block, &iso, None)))
        })
    } else {
        let header = format!(
            concat!(
                "# Budget {year} — SEULE SOURCE DE VÉRITÉ budgétaire du foyer.\n",
                "# Une directive = une ligne, édition chirurgicale par la GUI.\n"
            ),
            year = year
        );
        let content = insert_sorted("", &block, &iso, Some(&header));
        let include_line = format!("include \"budgets/{year}.beancount\"");
        let mut changes = vec![household_core::writer::FileChange::new(target, move |_| {
            Ok(Some(content.clone()))
        })];
        let year_closure = year;
        changes.push(household_core::writer::FileChange::new(
            main.to_path_buf(),
            move |old| Ok(Some(ensure_include(old, year_closure, &include_line))),
        ));
        edit_ledger_files(main, changes)
    }
}

/// Date ISO d'une directive budget (pour l'insertion triée).
fn directive_date_iso(directive: &Directive) -> Option<String> {
    match directive {
        Directive::Custom(custom) => Some(custom.date.to_string()),
        _ => None,
    }
}

/// Garantit la présence de `include "budgets/YYYY.beancount"` dans `main` :
/// inséré **trié par année croissante** (après le dernier include de budget
/// d'année inférieure, avant le premier d'année supérieure), sinon après
/// `include "accounts.beancount"`, sinon après le dernier include.
fn ensure_include(old: &str, year: i16, include_line: &str) -> String {
    let needle = format!("include \"budgets/{year}.beancount\"");
    if old.lines().any(|line| line.trim() == needle) {
        return old.to_string();
    }

    let lines: Vec<&str> = old.lines().collect();

    /// Extrait l'année d'une ligne `include "budgets/YYYY.beancount"`.
    fn budget_year(line: &str) -> Option<i16> {
        let trimmed = line.trim_start();
        trimmed
            .strip_prefix("include \"budgets/")
            .and_then(|s| s.strip_suffix(".beancount\""))
            .and_then(|s| s.trim_end_matches('"').parse().ok())
    }

    let is_budget_include = |line: &str| line.trim_start().starts_with("include \"budgets/");
    let is_accounts = |line: &str| line.trim() == "include \"accounts.beancount\"";
    let is_include = |line: &str| line.trim_start().starts_with("include ");

    // Position d'insertion pour maintenir les includes de budget triés
    // par année croissante.
    let position = lines
        .iter()
        .enumerate()
        .find(|(_, line)| is_budget_include(line) && budget_year(line).is_some_and(|y| y > year))
        .map(|(index, _)| index) // insérer juste avant le premier budget > year
        .or_else(|| {
            // Aucune année supérieure : insérer après le dernier include de budget
            lines
                .iter()
                .enumerate()
                .rfind(|(_, line)| is_budget_include(line))
                .map(|(index, _)| index + 1)
        })
        .or_else(|| {
            // Pas d'include de budget : après include "accounts.beancount"
            lines
                .iter()
                .position(|&line| is_accounts(line))
                .map(|i| i + 1)
        })
        .or_else(|| {
            // Sinon : après le dernier include
            lines
                .iter()
                .enumerate()
                .rfind(|(_, line)| is_include(line))
                .map(|(i, _)| i + 1)
        })
        .unwrap_or(lines.len());

    let mut out = String::with_capacity(old.len() + include_line.len() + 2);
    for (index, line) in lines.iter().enumerate() {
        out.push_str(line);
        out.push('\n');
        if index + 1 == position {
            out.push_str(include_line);
            out.push('\n');
        }
    }
    if position >= lines.len() {
        out.push_str(include_line);
        out.push('\n');
    }
    out
}

/// Mot-clé d'intervalle (ré-export pratique depuis [`crate::duplicate`]).
#[must_use]
pub fn interval_of(interval: Interval) -> &'static str {
    interval_keyword(interval)
}
