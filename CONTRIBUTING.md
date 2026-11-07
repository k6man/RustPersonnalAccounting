# Contributions

Merci d'avoir voulu contribuer. Ce projet suit une **workflow très rigoureux** :
tout sujet de conception est traité comme une décision d'architecture (voir
`docs/` — ADR). Les contributions doivent donc respecter les **conventions**
fixées en
`AGENTS.md` et le **ledger** géré par `examples/ledger/` (données fictives,
recette, jamais éditées par des outils).

## Avant de coder

- **Comprendre l'état du projet** : lisez d'abord `TODO.md` (journal, état
  courant, écarts). Les décisions d'architecture sont dans `docs/` (ADR).
- **Ne pas inventer** : si une API, un flag ou une dépendance n'est pas
  documentée, lisez le code / le Cargo.lock / `TODO.md` — ne la fictions pas.

## Commande de validation (obligatoires avant chaque commit)

```sh
just check          # fmt-check + clippy -D warnings + cargo test --workspace + rledger check
just run            # GUI (cargo run -p household-gui)
cargo test -p household-gui --test budget_gui grille_2026   # un seul test
cargo run -q -p household-core --bin household-cli -- check examples/ledger/main.beancount   # repli si rledger absent
```

- **Clippy et tests verts avec `-D warnings`** ; `unsafe_code = "forbid"` (lints
  workspace).
- **`just setup`** après un clone (active les hooks `githooks/`).
- Le **commit** revalide le ledger via le hook `pre-commit` (ou l'écran Git).
  Le message type est `ledger: AAAA-MM — saisie/édition (N fichiers)`.

## Éditeur de ledger

Toutes les écritures passent par les **writers** de `household-core`
(copie scratch + pré-validation + écriture atomique tmp+rename + blocs
canoniques déterministes). Ne nouvelles pas votre propre fichier ledger.

## Reporting

- **Ledger** : signalez chaque modification (`just check-ledger`).
- **Vulnérabilités / dépendances** : un ticket, avec l'identifiant de l'étude
  (advisory, licence, bans). Vérifiez le `CI` (jobs `cargo-audit`,
  `cargo-deny`).
- **Sensibilité** : les montants, IBAN, noms tiers, SIREN/SIRET, sorties bancaire
  export sont **données de sécurité** — ne les partagez pas dans les tickets.

## Licence

**GPL-3.0-only** (dérivé de rustledger, GPL‑3.0, ADR-001). Ne remplacez pas
la licence. Les dépendances de développement sont listées dans
`THIRD_PARTY_LICENSES.md`.