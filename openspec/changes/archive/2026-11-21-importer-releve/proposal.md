# Importer un relevé bancaire (CSV/OFX)

## Pourquoi
Idée « hors périmètre v1 » (cf. docs/idees-hors-perimetre.md) : éviter la saisie manuelle des relevés. rustledger fournit
`rledger extract` et `importers.toml`.

## Ce qui change
- Commande worker `ImporterReleve` : lecture d'un CSV/OFX, proposition de transactions.
- Écran de rapprochement : accepter, corriger ou ignorer chaque ligne avant écriture.
- Écriture via `append_transaction` (pré-vérification scratch, atomicité) ; aucune nouvelle voie d'écriture.

## Impact
- Nouvelle capability `importer-releve`.
- Risque : dépendance à l'API instable rustledger 0.24.0 (épinglée) ; à vérifier dans les sources.
- Décision à prendre : détection des doublons sur réimport (date + montant + libellé).
