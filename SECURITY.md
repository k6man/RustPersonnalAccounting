# Sécurité

Ce projet gère une **comptabilité personnelle** : il contient et peut encaisser
des **données de sécurité** (montants, IBAN, noms tiers, SIREN/SIRET,
sorties bancaire export, identifiants).

## Quand signaler une vulnérabilité

- Contactez le responsable via l'adresse de contact documentée.
- **Ne publiez rien** de votre dépôt ; passez par le canal de signalement.
- **Ne partagez pas** les données ou identifiants liés à la demande.
- La réponse est donnée sous l'responsabilité des contributeurs.

## Processus de réponse

1. Validation immédiate (CRT de police).
2. Analyse technique (honeurs/security review).
3. Tests (CVE / exale) si la vulnérabilité est exploitée.
4. Exploitation de correctifs (version, drop-in ou patch).

## Fixtures et contenu réel

- `examples/ledger/` contient **des données fictives** (accueil annonyme,
  montants invents) : recette et golden de tests. Il ne doit pas être édité avec
  des données vraies, ni partagé avec un public si possible.
- Le **vrai ledger** du foyer vit **hors du dépôt** (ADR-004), dans sa propre
  arborescence ; son contenu n'est pas versionné ici.

## Licence

**GPL-3.0-only** (ADR-001). Les dépendances y compris ; voir
`THIRD_PARTY_LICENSES.md`.