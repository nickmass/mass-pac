use save_states::SaveState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub enum UserInput {
    Player(Player),
    Reset,
}

impl Default for UserInput {
    fn default() -> Self {
        Self::Player(Player::default())
    }
}

#[derive(Debug, Copy, Clone, Default, Deserialize, Serialize)]
pub struct Player {
    pub up_1: bool,
    pub down_1: bool,
    pub left_1: bool,
    pub right_1: bool,
    pub up_2: bool,
    pub down_2: bool,
    pub left_2: bool,
    pub right_2: bool,
    pub coin_1: bool,
    pub coin_2: bool,
    pub start_1: bool,
    pub start_2: bool,
    pub test: bool,
    pub rack_advance: bool,
    pub cocktail: bool,
    pub credit: bool,
}

impl Player {
    pub fn port_0(&self) -> u8 {
        let mut v = 0;
        if self.up_1 {
            v |= 0x01;
        }
        if self.left_1 {
            v |= 0x02;
        }
        if self.right_1 {
            v |= 0x04;
        }
        if self.down_1 {
            v |= 0x08;
        }
        if self.rack_advance {
            v |= 0x10;
        }
        if self.coin_1 {
            v |= 0x20;
        }
        if self.coin_2 {
            v |= 0x40;
        }
        if self.credit {
            v |= 0x80;
        }

        !v
    }

    pub fn port_1(&self) -> u8 {
        let mut v = 0;
        if self.up_2 {
            v |= 0x01;
        }
        if self.left_2 {
            v |= 0x02;
        }
        if self.right_2 {
            v |= 0x04;
        }
        if self.down_2 {
            v |= 0x08;
        }
        if self.test {
            v |= 0x10;
        }
        if self.start_1 {
            v |= 0x20;
        }
        if self.start_2 {
            v |= 0x40;
        }
        if self.cocktail {
            v |= 0x80;
        }

        !v
    }
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct DipSettings {
    pub cost: dip::Cost,
    pub lives: dip::Lives,
    pub bonus: dip::BonusLife,
    pub difficulty: dip::Difficulty,
    pub names: dip::GhostNames,
}

impl DipSettings {
    pub fn value(&self) -> u8 {
        self.cost as u8
            | self.lives as u8
            | self.bonus as u8
            | self.difficulty as u8
            | self.names as u8
    }
}

pub mod dip {
    #![allow(unused)]
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
    #[repr(u8)]
    pub enum Cost {
        FreePlay = 0x00,
        OneCreditPerCoin = 0x01,
        TwoCreditsPerCoin = 0x02,
        #[default]
        TwoCoinsPerCredit = 0x03,
    }

    #[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
    #[repr(u8)]
    pub enum Lives {
        One = 0x00,
        Two = 0x04,
        Three = 0x08,
        #[default]
        Five = 0x0c,
    }

    #[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
    #[repr(u8)]
    pub enum BonusLife {
        Score10000 = 0x00,
        Score15000 = 0x10,
        Score20000 = 0x20,
        #[default]
        None = 0x30,
    }

    #[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
    #[repr(u8)]
    pub enum Difficulty {
        Hard = 0x00,
        #[default]
        Normal = 0x40,
    }

    #[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
    #[repr(u8)]
    pub enum GhostNames {
        Alternate = 0x00,
        #[default]
        Normal = 0x80,
    }
}

#[derive(Default, SaveState)]
pub struct Input {
    player: Player,
    dip: DipSettings,
    pending_reset: bool,
}

impl Input {
    pub fn new(dip: DipSettings) -> Self {
        Self {
            dip,
            ..Default::default()
        }
    }

    pub fn read(&self, address: u16, value: u8) -> u8 {
        match address & 0x7fc0 {
            0x5000 => self.player.port_0(),
            0x5040 => self.player.port_1(),
            0x5080 => self.dip.value(),
            _ => value,
        }
    }

    pub fn reset(&mut self) -> bool {
        let reset = self.pending_reset;
        self.pending_reset = false;
        reset
    }

    pub fn handle_input(&mut self, input: UserInput) {
        match input {
            UserInput::Player(player) => self.player = player,
            UserInput::Reset => self.pending_reset = true,
        }
    }
}
