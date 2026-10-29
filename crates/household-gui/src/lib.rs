//! Bibliothèque de l'application « Comptabilité du foyer ».
//!
//! Découpage (le binaire `compta-foyer` n'est qu'un point d'entrée) :
//! - [`viewmodel`] : assemblage **pur** des données affichées (thread worker) ;
//! - [`filters`] : filtres du journal et pagination (pur) ;
//! - [`fmt`] : formatage des nombres (pur) ;
//! - [`worker`] : thread worker de rechargement + watcher `notify` ;
//! - [`views`] : écrans egui — présentation seule, pas de logique métier ;
//! - [`app`] : état applicatif, onglets, bannière d'erreurs, canaux.
//!
//! Trait `eframe::App` 0.37 : `logic` (hors rendu) puis `ui` (Ui monté).

pub mod app;
pub mod filters;
pub mod fmt;
pub mod saisie;
pub mod viewmodel;
pub mod views;
pub mod worker;
