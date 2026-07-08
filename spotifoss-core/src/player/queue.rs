use rand::prelude::SliceRandom;

use super::PlaybackItem;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

pub struct Queue {
    items: Vec<PlaybackItem>,
    user_items: Vec<PlaybackItem>,
    position: usize,
    user_items_position: usize,
    positions: Vec<usize>,
    shuffle: bool,
    repeat: RepeatMode,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            user_items: Vec::new(),
            position: 0,
            user_items_position: 0,
            positions: Vec::new(),
            shuffle: false,
            repeat: RepeatMode::default(),
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.user_items.clear();
        self.user_items_position = 0;
        self.positions.clear();
        self.position = 0;
    }

    pub fn fill(&mut self, items: Vec<PlaybackItem>, position: usize) {
        self.user_items.clear();
        self.user_items_position = 0;
        self.positions.clear();
        self.items = items;
        self.position = position;
        self.compute_positions();
    }

    pub fn replace(&mut self, items: Vec<PlaybackItem>) {
        let current = self.get_current().copied();
        self.items = items;
        self.user_items.clear();
        self.user_items_position = 0;
        self.positions.clear();
        self.position = current
            .and_then(|item| self.items.iter().position(|candidate| *candidate == item))
            .unwrap_or(0);
        self.compute_positions();
    }

    pub fn add(&mut self, item: PlaybackItem) {
        self.user_items.push(item);
    }

    pub fn add_next(&mut self, item: PlaybackItem) {
        let insert_index = self.items.len();
        self.items.push(item);
        if self.positions.is_empty() {
            self.positions.push(insert_index);
            self.position = 0;
            return;
        }
        let insert_pos = (self.position + 1).min(self.positions.len());
        self.positions.insert(insert_pos, insert_index);
    }

    fn handle_added_queue(&mut self) {
        if self.user_items.len() > self.user_items_position {
            // Insert the next user item right after the current position
            // so it plays immediately on the next skip (matching UI behavior).
            let item_index = self.items.len();
            self.items.push(self.user_items[self.user_items_position]);
            let insert_pos = (self.position + 1).min(self.positions.len());
            self.positions.insert(insert_pos, item_index);
            self.user_items_position += 1;
        }
    }

    pub fn set_settings(&mut self, shuffle: bool, repeat: RepeatMode) {
        let reshuffle = self.shuffle != shuffle;
        self.shuffle = shuffle;
        self.repeat = repeat;
        if reshuffle {
            self.compute_positions();
        }
    }

    fn compute_positions(&mut self) {
        // In the case of switching away from shuffle, the position should be set back to
        // where it appears in the actual playlist order.
        let playlist_position = if self.positions.len() > 1 {
            self.positions[self.position]
        } else {
            self.position
        };
        // Start with an ordered 1:1 mapping.
        self.positions = (0..self.items.len()).collect();

        if self.shuffle {
            // Swap the current position with the first item, so we will start from the
            // beginning, with the full queue ahead of us.  Then shuffle the rest of the
            // items and set the position to 0.
            if self.positions.len() > 1 {
                self.positions.swap(0, self.position);
                self.positions[1..].shuffle(&mut rand::rng());
            }
            self.position = 0;
        } else {
            self.position = playlist_position;
        }
    }

    pub fn skip_to_previous(&mut self) {
        self.position = self.previous_position();
    }

    pub fn skip_to_next(&mut self) {
        self.handle_added_queue();
        self.position = self.next_position();
    }

    pub fn skip_to_following(&mut self) {
        self.handle_added_queue();
        self.position = self.following_position();
    }

    pub fn get_current(&self) -> Option<&PlaybackItem> {
        let position = self.positions.get(self.position).copied()?;
        self.items.get(position)
    }

    pub fn get_following(&self) -> Option<&PlaybackItem> {
        if self.items.is_empty() {
            return self.user_items.first();
        }

        let next_position = self.following_position();
        if let Some(position) = self.positions.get(next_position).copied() {
            self.items.get(position)
        } else {
            self.user_items.first()
        }
    }

    fn previous_position(&self) -> usize {
        self.position.saturating_sub(1)
    }

    fn next_position(&self) -> usize {
        match self.repeat {
            RepeatMode::One => self.position,
            RepeatMode::All if !self.items.is_empty() => {
                (self.position + 1) % self.items.len()
            }
            RepeatMode::Off | RepeatMode::All => self.position + 1,
        }
    }

    fn following_position(&self) -> usize {
        match self.repeat {
            RepeatMode::One => self.position,
            RepeatMode::All if !self.items.is_empty() => {
                (self.position + 1) % self.items.len()
            }
            RepeatMode::Off | RepeatMode::All => self.position + 1,
        }
    }
}
