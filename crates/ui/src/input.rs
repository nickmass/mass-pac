use std::collections::HashMap;

use pacman::Player;
use winit::keyboard::KeyCode;

pub struct InputMap {
    map: HashMap<InputType, bool>,
}

impl InputMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn is_pressed(&self, key: impl Into<InputType>) -> bool {
        self.map.get(&key.into()).cloned().unwrap_or(false)
    }

    pub fn press(&mut self, key: impl Into<InputType>) {
        self.map
            .entry(key.into())
            .and_modify(|e| *e = true)
            .or_insert(true);
    }

    pub fn release(&mut self, key: impl Into<InputType>) {
        self.map
            .entry(key.into())
            .and_modify(|e| *e = false)
            .or_insert(false);
    }

    pub fn controller(&self) -> Player {
        Player {
            up_1: self.is_pressed(KeyCode::ArrowUp),
            down_1: self.is_pressed(KeyCode::ArrowDown),
            left_1: self.is_pressed(KeyCode::ArrowLeft),
            right_1: self.is_pressed(KeyCode::ArrowRight),
            up_2: self.is_pressed(KeyCode::KeyW),
            down_2: self.is_pressed(KeyCode::KeyS),
            left_2: self.is_pressed(KeyCode::KeyA),
            right_2: self.is_pressed(KeyCode::KeyD),
            coin_1: self.is_pressed(KeyCode::ShiftRight)
                || self.is_pressed(KeyCode::Backslash)
                || self.is_pressed(KeyCode::F1),
            coin_2: self.is_pressed(KeyCode::F2),
            start_1: self.is_pressed(KeyCode::Enter) || self.is_pressed(KeyCode::F3),
            start_2: self.is_pressed(KeyCode::F4),
            test: self.is_pressed(KeyCode::KeyZ),
            rack_advance: self.is_pressed(KeyCode::KeyX),
            cocktail: false,
            credit: self.is_pressed(KeyCode::F5),
        }
    }

    pub fn reset(&self) -> bool {
        self.is_pressed(KeyCode::Backspace)
    }

    pub fn pause(&self) -> bool {
        self.is_pressed(KeyCode::Space)
    }

    pub fn rewind(&self) -> bool {
        self.is_pressed(KeyCode::Tab)
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum InputType {
    Key(KeyCode),
}

impl From<KeyCode> for InputType {
    fn from(value: KeyCode) -> Self {
        InputType::Key(value)
    }
}
