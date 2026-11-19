//! Vues ratatui — présentation seule, pas de logique métier.
//! Chaque écran expose une fonction `afficher` qui dessine dans le `Frame`
//! ratatui et retourne une option d'action utilisateur.

pub mod budget;
pub mod dashboard;
pub mod git;
pub mod graphiques;
pub mod import;
pub mod journal;
pub mod rapports;
pub mod saisie;
