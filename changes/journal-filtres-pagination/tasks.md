## 1. Filtres
- [ ] 1.1 Ajouter l'état de filtre à `UiState` du journal
- [ ] 1.2 Rendre les champs de filtre (curseur visible, `sync_focus`)
- [ ] 1.3 Appliquer `JournalFilter::correspond` avant affichage

## 2. Pagination
- [ ] 2.1 Utiliser `Pagination { par_page: 100 }` à la place de `MAX_LIGNES`
- [ ] 2.2 Touches page précédente/suivante et affichage « page X/Y »
- [ ] 2.3 Appeler `borner` quand le total change

## 3. Actions
- [ ] 3.1 Résoudre la position filtrée/paginée vers l'index ledger à l'édition et à la suppression
- [ ] 3.2 Tests TestBackend avec fixture de plus de 100 lignes
