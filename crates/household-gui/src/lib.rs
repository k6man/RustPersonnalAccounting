//! Bibliothèque de l'application « Comptabilité du foyer » (TUI ratatui).
//!
//! Découpage (le binaire `compta-foyer` n'est qu'un point d'entrée) :
//! - [`viewmodel`] : assemblage **pur** des données affichées (thread worker) ;
//! - [`filters`] : filtres du journal et pagination (pur) ;
//! - [`fmt`] : formatage des nombres (pur) ;
//! - [`worker`] : thread worker de rechargement + watcher `notify` ;
//! - [`views`] : écrans ratatui — présentation seule, pas de logique métier ;
//! - [`app`] : état applicatif, écrans, bannière d'erreurs, canaux.
//!
//! Boucle TUI : `app.run(&mut terminal)` dessine puis attend un événement
//! clavier, traduit en `Command` vers le worker.

pub mod app;
pub mod budget;
pub mod config;
pub mod filters;
pub mod fmt;
pub mod saisie;
pub mod viewmodel;
pub mod views;
pub mod worker;
