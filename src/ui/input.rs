use winit::keyboard::KeyCode;

use std::collections::HashMap;

use crate::pacman::Player;

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
            service: self.is_pressed(KeyCode::KeyZ),
            rack_test: self.is_pressed(KeyCode::KeyX),
            coin: self.is_pressed(KeyCode::ShiftRight) || self.is_pressed(KeyCode::Backslash),
            start: self.is_pressed(KeyCode::Enter),
            up: self.is_pressed(KeyCode::ArrowUp),
            down: self.is_pressed(KeyCode::ArrowDown),
            left: self.is_pressed(KeyCode::ArrowLeft),
            right: self.is_pressed(KeyCode::ArrowRight),
        }
    }

    pub fn reset(&self) -> bool {
        self.is_pressed(KeyCode::Backspace)
    }

    pub fn pause(&self) -> bool {
        self.is_pressed(KeyCode::Space)
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
