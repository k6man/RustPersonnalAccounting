//! Widget de saisie de texte interactif (TextInput basique) pour le TUI ratatui.
//!
//! Gère un buffer de texte avec curseur, l'édition (insertion, suppression,
//! déplacement), et le rendu avec mise en évidence du champ actif.
//!
//! Usage :
//! ```text
//! let mut input = TextInput::new("valeur initiale");
//! input.on_key(key); // dans la boucle d'événements
//! f.render_widget(&input, area); // dans le rendu
//! ```

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

/// Un champ de saisie de texte avec curseur.
#[derive(Debug, Clone, Default)]
pub struct TextInput {
    /// Buffer de caractères.
    pub value: String,
    /// Position du curseur (index en octets dans `value`).
    pub cursor: usize,
    /// Le champ est-il actif (focus) ?
    pub focused: bool,
    /// Texte affiché à gauche du champ (label).
    pub label: Option<String>,
    /// Placeholder affiché quand le champ est vide et non focalisé.
    pub placeholder: Option<String>,
    /// Largeur maximale d'affichage (tronque si dépassé).
    pub max_width: Option<u16>,
}

impl TextInput {
    /// Crée un champ vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Crée un champ avec une valeur initiale.
    pub fn with_value(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            cursor: value.len(),
            value,
            ..Default::default()
        }
    }

    /// Définit le label affiché à gauche.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Définit le placeholder.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Définit la largeur maximale d'affichage.
    pub fn max_width(mut self, width: u16) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Retourne le texte affiché (truncaté si nécessaire).
    fn display_text(&self) -> String {
        if self.value.is_empty() && !self.focused {
            if let Some(p) = &self.placeholder {
                return p.clone();
            }
            return String::new();
        }
        let mut s = self.value.clone();
        let max = self.max_width.map_or(s.len(), |m| m as usize).min(s.len());
        s.truncate(max);
        s
    }

    /// Insère un caractère à la position du curseur.
    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    /// Supprime le caractère avant le curseur.
    pub fn delete_backward(&mut self) {
        if self.cursor > 0 {
            // Trouver le début du caractère UTF-8 contenant l'octet avant le curseur.
            let byte_before = self.cursor - 1;
            let mut start = byte_before;
            while start > 0 && !self.value.is_char_boundary(start) {
                start -= 1;
            }
            self.value.remove(start);
            self.cursor = start;
        }
    }

    /// Supprime le caractère après le curseur.
    pub fn delete_forward(&mut self) {
        if self.cursor < self.value.len() {
            let end = self.value[self.cursor..]
                .char_indices()
                .next()
                .map_or(self.value.len(), |(i, _)| self.cursor + i);
            self.value.remove(self.cursor);
            self.cursor = end;
        }
    }

    /// Déplace le curseur d'un caractère vers l'avant.
    pub fn move_forward(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor = self.value[self.cursor..]
                .char_indices()
                .next()
                .map_or(self.value.len(), |(i, _)| self.cursor + i);
        }
    }

    /// Déplace le curseur d'un caractère vers l'arrière.
    pub fn move_backward(&mut self) {
        if self.cursor > 0 {
            let byte_before = self.cursor - 1;
            self.cursor = self.value[..byte_before]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
        }
    }

    /// Déplace le curseur au début.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Déplace le curseur à la fin.
    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Gère un événement clavier. Retourne `true` si la touche a été consommée.
    pub fn on_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char(c) => {
                self.insert_char(c);
                true
            }
            KeyCode::Backspace => {
                self.delete_backward();
                true
            }
            KeyCode::Delete => {
                self.delete_forward();
                true
            }
            KeyCode::Left => {
                self.move_backward();
                true
            }
            KeyCode::Right => {
                self.move_forward();
                true
            }
            KeyCode::Home => {
                self.move_home();
                true
            }
            KeyCode::End => {
                self.move_end();
                true
            }
            _ => false,
        }
    }

    /// Définit le focus.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

impl Widget for &TextInput {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let style = if self.focused {
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let texte = if let Some(label) = &self.label {
            format!("{} {}", label, self.display_text())
        } else {
            self.display_text()
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(texte, style)))
            .block(Block::default().borders(Borders::NONE));
        paragraph.render(area, buf);
    }
}

impl Widget for TextInput {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        (&self).render(area, buf)
    }
}
