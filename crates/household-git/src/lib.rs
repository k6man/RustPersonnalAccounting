//! Opérations git en-processus ([`git2`](https://docs.rs/git2), libgit2).
//!
//! Squelette du jalon 1 ; implémentation au jalon 7 :
//!
//! - statut des fichiers du ledger (par état travaillé/indexé) ;
//! - **commit programmatique**, précédé d'une **revalidation complète du ledger**
//!   par l'application (indépendante du hook `pre-commit`, car libgit2 n'exécute
//!   pas les hooks : le contrôle nous appartient, demande `household-cli check`) ;
//! - historique simple du répertoire `/ledger` (arbre des paths modifiés).
//!
//! Le binaire `git` n'est requis nulle part ; le hook `pre-commit` du dépôt, lui,
//! appelle `rledger check` (ou le repli `household-cli`) et s'applique à toute
//! modification faite hors GUI.
