## 1. Reconnaissance
- [ ] 1.1 Lire dans les sources 0.24.0 l'API d'extraction utilisable depuis une lib
- [ ] 1.2 Trancher la détection de doublons (ADR-005)

## 2. Cœur
- [ ] 2.1 Parser CSV/OFX vers `Brouillon`
- [ ] 2.2 Correspondance tiers → compte de dépense par historique de payees
- [ ] 2.3 Détection de doublons contre le Snapshot

## 3. TUI
- [ ] 3.1 Nouvel écran de rapprochement (hors Ctrl+1..7 saturé : décider de l'onglet)
- [ ] 3.2 Tests TestBackend sur fixture réelle

## 4. Qualité
- [ ] 4.1 Tests d'intégration sur scratch du ledger d'exemple
- [ ] 4.2 `just check` vert
