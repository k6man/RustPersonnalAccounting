//! Écran Import bancaire — preview et confirmation des transactions extraites.
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::widgets::text_input::TextInput;
use crate::worker::TransactionImportee;
use household_core::importer::{
    ImporterProfile, ResolutionImporters, SourceImporters, charger_importers_pour_ledger,
};

/// Action utilisateur retournée par l'écran Import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Aucune action.
    Aucun,
    /// Choisir un fichier et lancer l'import.
    ChoisirFichierEtImporter,
    /// Confirmer l'écriture des transactions en preview.
    ConfirmerEcriture,
    /// Recharger les profils (importers.toml modifié).
    RechargerProfils,
    /// Annuler / reset.
    Annuler,
}

/// État de l'écran Import.
#[derive(Debug, Default)]
pub struct UiState {
    /// Profils disponibles (chargés au premier rendu).
    pub profils: Vec<ImporterProfile>,
    /// Index du profil sélectionné.
    pub profil_selectionne: usize,
    /// Résolution du fichier importers.toml (pour affichage info).
    pub resolution: Option<ResolutionImporters>,
    /// Fichier source sélectionné (via rfd).
    pub fichier_source: Option<PathBuf>,
    /// Transactions en preview (remplies après LoadOutcome::Importe).
    pub preview: Vec<TransactionImportee>,
    /// Message de l'import.
    pub message_import: Option<String>,
    /// Avertissements de l'import.
    pub avertissements_import: Vec<String>,
    /// État de la table de preview (sélection, scroll).
    pub table_state: TableState,
    /// Hauteur visible de la table (mesurée au rendu).
    pub table_visibles: usize,
    /// Scroll vertical de la table.
    pub defilement: usize,
    /// En cours de chargement (entre Command::Importer et LoadOutcome).
    pub en_chargement: bool,
    /// Dernière erreur (si LoadOutcome::Refuse/Echec).
    pub derniere_erreur: Option<String>,
    /// Champ de saisie pour le chemin de fichier (si on veut le taper manuellement).
    pub champ_fichier: TextInput,
    /// Activer la détection de doublons (--existing).
    pub detecter_doublons: bool,
    /// Activer la suggestion de catégories ML (--suggest-categories).
    pub suggérer_catégories: bool,
    /// Profil d'import par défaut configuré dans les préférences (nom du profil).
    /// Utilisé au chargement pour pré-sélectionner le bon profil.
    pub profil_defaut: Option<String>,
}

impl UiState {
    /// Charge les profils depuis le ledger actif.
    pub fn charger_profils(&mut self, main: &std::path::Path) {
        let (config, resolution) = charger_importers_pour_ledger(main);
        self.profils = config.importers;
        self.resolution = Some(resolution);

        // Si un profil par défaut est configuré, essayer de le sélectionner
        if let Some(ref defaut) = self.profil_defaut {
            if let Some(pos) = self.profils.iter().position(|p| p.name == *defaut) {
                self.profil_selectionne = pos;
            } else {
                // Profil par défaut introuvable → fallback sur le premier
                self.profil_selectionne = 0;
            }
        } else if self.profil_selectionne >= self.profils.len() {
            // Aucun profil par défaut → premier profil
            self.profil_selectionne = 0;
        }
    }

    /// Réinitialise l'état après une écriture confirmée.
    pub fn reset_apres_ecriture(&mut self) {
        self.preview.clear();
        self.message_import = None;
        self.avertissements_import.clear();
        self.fichier_source = None;
        self.derniere_erreur = None;
        self.table_state = TableState::default();
        self.defilement = 0;
        self.champ_fichier = TextInput::new();
    }

    /// Profil actuellement sélectionné.
    pub fn profil_actuel(&self) -> Option<&ImporterProfile> {
        self.profils.get(self.profil_selectionne)
    }

    /// Gère les touches clavier. Retourne une Action à exécuter par l'appelant (App).
    pub fn on_key(&mut self, key: KeyEvent) -> Action {
        // Si un chargement est en cours, ne consommer que Esc (annule).
        if self.en_chargement {
            if key.code == KeyCode::Esc {
                self.en_chargement = false;
                self.derniere_erreur = Some("import annulé".to_string());
                return Action::Annuler;
            }
            return Action::Aucun;
        }

        // Si le champ fichier a le focus, laisser TextInput le gérer
        if self.champ_fichier.focused {
            self.champ_fichier.on_key(key);
            if key.code == KeyCode::Enter {
                let chemin = self.champ_fichier.value.trim();
                if !chemin.is_empty() {
                    self.fichier_source = Some(PathBuf::from(chemin));
                    self.champ_fichier = TextInput::new();
                    return Action::ChoisirFichierEtImporter;
                }
            }
            return Action::Aucun;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') if key.modifiers == KeyModifiers::NONE => {
                self.selection_precedente();
                Action::Aucun
            }
            KeyCode::Down | KeyCode::Char('j') if key.modifiers == KeyModifiers::NONE => {
                self.selection_suivante();
                Action::Aucun
            }
            KeyCode::PageUp => {
                self.defilement_page_haut();
                Action::Aucun
            }
            KeyCode::PageDown => {
                self.defilement_page_bas();
                Action::Aucun
            }
            KeyCode::Home => {
                self.table_state.select(Some(0));
                self.defilement = 0;
                Action::Aucun
            }
            KeyCode::End => {
                if !self.preview.is_empty() {
                    self.table_state.select(Some(self.preview.len() - 1));
                    self.ajuster_defilement_pour_selection();
                }
                Action::Aucun
            }
            KeyCode::Left | KeyCode::Char('h') if key.modifiers == KeyModifiers::NONE => {
                // Profil précédent
                if self.profil_selectionne > 0 {
                    self.profil_selectionne -= 1;
                }
                Action::Aucun
            }
            KeyCode::Right | KeyCode::Char('l') if key.modifiers == KeyModifiers::NONE => {
                // Profil suivant
                if self.profil_selectionne + 1 < self.profils.len() {
                    self.profil_selectionne += 1;
                }
                Action::Aucun
            }
            KeyCode::Enter => {
                if self.fichier_source.is_none() {
                    // Pas encore de fichier → ouvrir champ pour taper chemin ou utiliser rfd
                    self.champ_fichier = TextInput::new();
                    self.champ_fichier.set_focused(true);
                    Action::Aucun
                } else if !self.preview.is_empty() {
                    // Preview présente → confirmer écriture
                    Action::ConfirmerEcriture
                } else {
                    Action::Aucun
                }
            }
            KeyCode::Char('r') if key.modifiers == KeyModifiers::NONE => {
                // r = recharger les profils (si importers.toml modifié)
                Action::RechargerProfils
            }
            KeyCode::Char('d') if key.modifiers == KeyModifiers::NONE => {
                // d = basculer détection doublons (--existing)
                self.detecter_doublons = !self.detecter_doublons;
                // Si on désactive la détection doublons, désactiver aussi la suggestion catégories
                if !self.detecter_doublons {
                    self.suggérer_catégories = false;
                }
                Action::Aucun
            }
            KeyCode::Char('m') if key.modifiers == KeyModifiers::NONE => {
                // m = basculer suggestion catégories ML (--suggest-categories)
                // Nécessite que la détection doublons soit active
                if self.detecter_doublons {
                    self.suggérer_catégories = !self.suggérer_catégories;
                }
                Action::Aucun
            }
            KeyCode::Esc => {
                // Annule preview / reset
                if self.fichier_source.is_some() || !self.preview.is_empty() {
                    Action::Annuler
                } else {
                    Action::Aucun
                }
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::NONE => {
                // f = ouvrir sélecteur de fichier (rfd) directement
                if self.fichier_source.is_none() {
                    Action::ChoisirFichierEtImporter
                } else {
                    Action::Aucun
                }
            }
            _ => Action::Aucun,
        }
    }

    fn selection_precedente(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) if i > 0 => i - 1,
            _ => 0,
        };
        self.table_state.select(Some(i));
        self.ajuster_defilement_pour_selection();
    }

    fn selection_suivante(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) if i + 1 < self.preview.len() => i + 1,
            Some(i) => i,
            None => 0,
        };
        self.table_state.select(Some(i));
        self.ajuster_defilement_pour_selection();
    }

    fn defilement_page_haut(&mut self) {
        let page = self.table_visibles.saturating_sub(1).max(1);
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(page),
            None => 0,
        };
        self.table_state.select(Some(i));
        if self.defilement > i {
            self.defilement = i;
        }
    }

    fn defilement_page_bas(&mut self) {
        let page = self.table_visibles.saturating_sub(1).max(1);
        let i = match self.table_state.selected() {
            Some(i) => (i + page).min(self.preview.len().saturating_sub(1)),
            None => 0,
        };
        self.table_state.select(Some(i));
        self.ajuster_defilement_pour_selection();
    }

    fn ajuster_defilement_pour_selection(&mut self) {
        if let Some(sel) = self.table_state.selected() {
            if sel < self.defilement {
                self.defilement = sel;
            } else if sel >= self.defilement + self.table_visibles {
                self.defilement = sel + 1 - self.table_visibles;
            }
        }
    }

    /// Traite le LoadOutcome reçu du worker.
    pub fn traiter_outcome(&mut self, outcome: &crate::worker::LoadOutcome) {
        use crate::worker::LoadOutcome;
        self.en_chargement = false;
        match outcome {
            LoadOutcome::Importe {
                transactions,
                message,
                avertissements,
            } => {
                self.preview = transactions.clone();
                self.message_import = Some(message.clone());
                self.avertissements_import = avertissements.clone();
                if !self.preview.is_empty() {
                    self.table_state.select(Some(0));
                }
                self.defilement = 0;
            }
            LoadOutcome::Refuse(msg) => {
                self.derniere_erreur = Some(msg.clone());
            }
            LoadOutcome::Echec(msg) => {
                self.derniere_erreur = Some(msg.clone());
            }
            LoadOutcome::Ecrit { .. } => {
                // Écriture confirmée : reset pour nouvelle import
                self.reset_apres_ecriture();
            }
            _ => {} // Autres outcomes ignorés
        }
    }
}

/// Rend l'écran Import.
pub fn render(f: &mut Frame, etat: &mut UiState, main: &std::path::Path, area: Rect) {
    // Charger profils si pas encore fait
    if etat.profils.is_empty() {
        etat.charger_profils(main);
    }

    // Layout principal : 4 zones
    // 1. En-tête (profil + fichier + résolution)
    // 2. Tableau preview
    // 3. Bande avertissements / message
    // 4. Barre d'aide / boutons
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // En-tête (profil + options)
            Constraint::Min(10),   // Tableau preview
            Constraint::Length(3), // Avertissements / message
            Constraint::Length(3), // Aide / boutons
        ])
        .split(area);

    // === 1. En-tête ===
    render_entete(f, etat, chunks[0]);

    // === 2. Tableau preview ===
    render_tableau(f, etat, chunks[1]);

    // === 3. Avertissements / message ===
    render_avertissements(f, etat, chunks[2]);

    // === 4. Barre d'aide / boutons ===
    render_aide(f, etat, chunks[3]);
}

fn render_entete(f: &mut Frame, etat: &mut UiState, area: Rect) {
    let profil = etat.profil_actuel();
    let resolution = &etat.resolution;

    let mut lignes = Vec::new();

    // Ligne 1 : Profil
    let profil_txt = if let Some(p) = profil {
        format!("Profil : {}  (type: {})", p.name, p.r#type)
    } else {
        "Profil : aucun disponible".to_string()
    };
    lignes.push(Line::from(profil_txt).style(Style::default().fg(Color::Cyan)));

    // Ligne 2 : Compte cible / devise du profil
    if let Some(p) = profil {
        let compte = &p.account;
        let devise = p.currency.as_deref().unwrap_or("EUR");
        lignes.push(
            Line::from(format!("Compte cible : {}  |  Devise : {}", compte, devise))
                .style(Style::default().fg(Color::Gray)),
        );
    }

    // Ligne 3 : Fichier source sélectionné
    let fichier_txt = if etat.champ_fichier.focused {
        format!("Fichier : {}_", etat.champ_fichier.value)
    } else if let Some(ref f) = etat.fichier_source {
        format!("Fichier : {}", f.display())
    } else {
        "Fichier : aucun (Entrée pour taper, 'f' pour sélecteur)".to_string()
    };
    let style_fichier = if etat.fichier_source.is_some() || etat.champ_fichier.focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };
    lignes.push(Line::from(fichier_txt).style(style_fichier));

    // Ligne 4 : Résolution importers.toml
    if let Some(res) = resolution {
        let source_label = match res.source {
            SourceImporters::RacineLedger => "racine du ledger",
            SourceImporters::ConfigLocale => "~/.config/rust-personal-accounting/",
            SourceImporters::Aucun => "aucun",
        };
        let chemin = res
            .chemin
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "introuvable".to_string());
        let txt = format!("importers.toml : {} ({})", source_label, chemin);
        let style = if res.fichier_trouve() {
            Color::Gray
        } else {
            Color::Red
        };
        lignes.push(Line::from(txt).style(Style::default().fg(style)));
    }

    // Ligne 5 : Options d'import
    let doublons_coche = if etat.detecter_doublons { "[x]" } else { "[ ]" };
    let categories_coche = if etat.suggérer_catégories {
        "[x]"
    } else {
        "[ ]"
    };
    let options_txt = format!(
        "Options : {} Détection doublons (--existing)  |  {} Catégorisation ML (--suggest-categories)",
        doublons_coche, categories_coche
    );
    lignes.push(Line::from(options_txt).style(Style::default().fg(Color::Gray)));

    let block = Block::default()
        .title(" Import bancaire ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let para = Paragraph::new(Text::from(lignes))
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(para, area);
}

fn render_tableau(f: &mut Frame, etat: &mut UiState, area: Rect) {
    if etat.preview.is_empty() {
        // Pas de preview : message d'invite
        let txt = if etat.en_chargement {
            "⏳ Import en cours…"
        } else if etat.fichier_source.is_some() {
            "Aucune transaction extraite (fichier vide ou format non reconnu)"
        } else {
            "Sélectionnez un profil (←/→), choisissez un fichier (Entrée ou 'f'), lancez l'import"
        };
        let style = if etat.en_chargement {
            Style::default().fg(Color::Yellow)
        } else if etat.derniere_erreur.is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Gray)
        };
        let para = Paragraph::new(txt)
            .style(style)
            .alignment(Alignment::Center);
        f.render_widget(para, area);
        return;
    }

    // Colonnes : Date | Flag | Tiers | Libellé | Montant | Devise | Compte cible | ⚠
    let header = Row::new(vec![
        Cell::from("Date").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("F").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Tiers").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Libellé").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Montant").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Devise").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Compte cible").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("⚠").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .bottom_margin(0);

    // Lignes visibles (fenêtre glissante)
    let debut = etat.defilement;
    let fin = (debut + etat.table_visibles).min(etat.preview.len());
    let lignes_visibles = &etat.preview[debut..fin];

    let rows: Vec<Row> = lignes_visibles
        .iter()
        .enumerate()
        .map(|(idx, t)| {
            let absolu = debut + idx;
            let est_selection = etat.table_state.selected() == Some(absolu);

            let montant_str = t
                .postings
                .iter()
                .find(|(c, _, _)| c == &t.compte_cible)
                .map(|(_, m, d)| format!("{m} {d}"))
                .unwrap_or_else(|| {
                    // Premier posting non nul
                    t.postings
                        .iter()
                        .find(|(_, m, _)| !m.is_zero())
                        .map(|(_, m, d)| format!("{m} {d}"))
                        .unwrap_or_else(|| "0".to_string())
                });

            let montant_style = if t
                .postings
                .iter()
                .any(|(c, m, _)| c == &t.compte_cible && m.is_sign_negative())
            {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };

            let mut cells = vec![
                Cell::from(t.date.to_string()),
                Cell::from(t.flag.to_string()),
                Cell::from(t.tiers.clone().unwrap_or_default()),
                Cell::from(t.libelle.clone()),
                Cell::from(montant_str).style(montant_style),
                Cell::from(t.devise.clone()),
                Cell::from(t.compte_cible.clone()),
                Cell::from(if t.avertissements.is_empty() {
                    ""
                } else {
                    "⚠"
                })
                .style(Style::default().fg(Color::Yellow)),
            ];

            if est_selection {
                for cell in &mut cells {
                    *cell = cell.clone().style(
                        Style::default()
                            .bg(Color::Blue)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    );
                }
            }

            Row::new(cells).height(1)
        })
        .collect();

    // Calculer hauteur visible pour scroll
    let inner = Block::default().borders(Borders::ALL).inner(area);
    etat.table_visibles = inner.height.saturating_sub(1) as usize; // -1 pour header

    // Titre avec indicateur de scroll
    let titre = if etat.preview.len() > etat.table_visibles {
        format!(
            " Preview — {} transactions (lignes {}-{} / {}) ",
            etat.preview.len(),
            debut + 1,
            fin,
            etat.preview.len()
        )
    } else {
        format!(" Preview — {} transactions ", etat.preview.len())
    };

    let widths = [
        Constraint::Length(12), // Date
        Constraint::Length(2),  // Flag
        Constraint::Min(12),    // Tiers
        Constraint::Min(20),    // Libellé
        Constraint::Length(14), // Montant
        Constraint::Length(8),  // Devise
        Constraint::Min(20),    // Compte cible
        Constraint::Length(3),  // Avertissement
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(titre)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .column_spacing(1);

    f.render_stateful_widget(table, area, &mut etat.table_state);
}

fn render_avertissements(f: &mut Frame, etat: &UiState, area: Rect) {
    let mut lignes = Vec::new();

    if let Some(ref err) = etat.derniere_erreur {
        lignes.push(Line::from(vec![
            Span::styled(
                "✗ ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(err.clone(), Style::default().fg(Color::Red)),
        ]));
    } else if let Some(ref msg) = etat.message_import {
        lignes.push(Line::from(vec![
            Span::styled(
                "✓ ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(msg.clone(), Style::default().fg(Color::Green)),
        ]));
    }

    for av in &etat.avertissements_import {
        lignes.push(Line::from(vec![
            Span::styled(
                "⚠ ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(av.clone(), Style::default().fg(Color::Yellow)),
        ]));
    }

    if lignes.is_empty() {
        lignes.push(
            Line::from("Prêt. Choisissez un fichier et lancez l'import (Entrée ou 'f').")
                .style(Style::default().fg(Color::Gray)),
        );
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    let para = Paragraph::new(Text::from(lignes))
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(para, area);
}

fn render_aide(f: &mut Frame, etat: &UiState, area: Rect) {
    let mut spans = Vec::new();

    // Bouton fichier
    if etat.fichier_source.is_none() && !etat.champ_fichier.focused {
        spans.push(Span::styled(
            " [Entrée] ",
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            "Taper chemin",
            Style::default().fg(Color::White),
        ));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            " [f] ",
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("Sélecteur", Style::default().fg(Color::White)));
        spans.push(Span::raw("  "));
    } else if !etat.preview.is_empty() && !etat.en_chargement {
        spans.push(Span::styled(
            " [Entrée] ",
            Style::default()
                .bg(Color::Green)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            "Confirmer écriture",
            Style::default().fg(Color::White),
        ));
        spans.push(Span::raw("  "));
    }

    // Navigation profils
    if etat.profils.len() > 1 {
        spans.push(Span::styled(
            " ←/→ ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ));
        spans.push(Span::styled("Profil", Style::default().fg(Color::White)));
        spans.push(Span::raw("  "));
    }

    // Options d'import
    spans.push(Span::styled(
        " d ",
        Style::default().bg(Color::DarkGray).fg(Color::White),
    ));
    spans.push(Span::styled(
        if etat.detecter_doublons {
            "Doublons ✓"
        } else {
            "Doublons ✗"
        },
        Style::default().fg(Color::White),
    ));
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        " m ",
        Style::default().bg(Color::DarkGray).fg(Color::White),
    ));
    spans.push(Span::styled(
        if etat.suggérer_catégories {
            "Catégories ML ✓"
        } else {
            "Catégories ML ✗"
        },
        Style::default().fg(Color::White),
    ));
    spans.push(Span::raw("  "));

    // Navigation tableau
    if !etat.preview.is_empty() {
        spans.push(Span::styled(
            " ↑/↓ ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ));
        spans.push(Span::styled("Sélection", Style::default().fg(Color::White)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            " PgUp/PgDn ",
            Style::default().bg(Color::DarkGray).fg(Color::White),
        ));
        spans.push(Span::styled("Page", Style::default().fg(Color::White)));
        spans.push(Span::raw("  "));
    }

    // Raccourcis globaux
    spans.push(Span::styled(
        " Esc ",
        Style::default().bg(Color::DarkGray).fg(Color::White),
    ));
    spans.push(Span::styled(
        "Annuler/Reset",
        Style::default().fg(Color::White),
    ));
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        " r ",
        Style::default().bg(Color::DarkGray).fg(Color::White),
    ));
    spans.push(Span::styled(
        "Recharger profils",
        Style::default().fg(Color::White),
    ));

    let aide = Paragraph::new(Line::from(spans))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(aide, area)
}
