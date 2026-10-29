//! Logique budgétaire **pure** (indépendante de toute UI et de tout I/O), testée unitairement.
//!
//! Squelette du jalon 1 ; implémentation au jalon 3 :
//!
//! - **Grille** postes × 12 mois : budget mensuel effectif, réel, écart, % consommé —
//!   bâtie sur [`rustledger_budget`] (`Budgets::from_directives`, fenêtres mensuelles
//!   avec `compare` / `accrue` qui reproduisent la sémantique Fava : accrual par jour,
//!   ancrage calendaire, supersession datée) ;
//! - **Agrégats** hiérarchiques (parent + enfants s'additionnent) et totaux ligne/colonne ;
//! - **Rollover** (optionnel, non fourni par rustledger) : report du non-dépensé/écart
//!   cumulé mois → mois suivant ;
//! - **Duplication** d'année N→N+1 avec ajustement optionnel en pour cent ;
//! - **Écriture chirurgicale** de `budgets/AAAA.beancount` : une directive = une ligne,
//!   éditée à son emplacement (préservation des commentaires), écritures atomiques.
//!
//! Le ledger (`budgets/AAAA.beancount`) reste la seule source de vérité.
