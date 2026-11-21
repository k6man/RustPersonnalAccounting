# Brancher filtres et pagination dans le Journal TUI

## Pourquoi
`filters.rs` (JournalFilter, Pagination à 100 lignes/page, parsing tolérant des dates et montants) est
implémenté et testé mais n'est utilisé par aucune vue : le Journal TUI n'affiche que les 50 premières
transactions (`MAX_LIGNES`). Un journal réel de plusieurs années est donc inaccessible au-delà.
Le plan TODO « Phase 4 — Journal » prévoyait ces fonctions.

## Ce qui change
- Champs de filtre dans l'écran Journal (texte, compte, tag, dates, montants).
- Pagination réelle (100 lignes/page) remplaçant `MAX_LIGNES`.
- Sélection exprimée en position d'affichage filtrée, résolue vers l'index ledger à l'action.

## Impact
- Spec `parcourir-journal` : requirements ajoutés.
- Code : `views/journal.rs`, `app.rs` (routage clavier : champs texte consomment toutes les touches).
