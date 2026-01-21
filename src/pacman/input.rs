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

#[derive(Debug, Copy, Clone, Default)]
pub struct Player {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub service: bool,
    pub rack_test: bool,
    pub coin: bool,
    pub start: bool,
}

impl Player {
    pub fn port_0(&self) -> u8 {
        let mut v = 0;
        if self.up {
            v |= 0x01;
        }
        if self.left {
            v |= 0x02;
        }
        if self.right {
            v |= 0x04;
        }
        if self.down {
            v |= 0x08;
        }
        if self.rack_test {
            v |= 0x10;
        }
        if self.coin {
            v |= 0x20;
        }

        !v
    }

    pub fn port_1(&self) -> u8 {
        let mut v = 0;
        if self.up {
            v |= 0x01;
        }
        if self.left {
            v |= 0x02;
        }
        if self.right {
            v |= 0x04;
        }
        if self.down {
            v |= 0x08;
        }
        if self.service {
            v |= 0x10;
        }
        if self.start {
            v |= 0x20;
        }

        !v
    }
}

#[derive(Debug, Copy, Clone, Default)]
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

    #[derive(Debug, Copy, Clone, Default)]
    #[repr(u8)]
    pub enum Cost {
        FreePlay = 0x00,
        OneCreditPerCoin = 0x01,
        TwoCreditsPerCoin = 0x02,
        #[default]
        TwoCoinsPerCredit = 0x03,
    }

    #[derive(Debug, Copy, Clone, Default)]
    #[repr(u8)]
    pub enum Lives {
        One = 0x00,
        Two = 0x04,
        Three = 0x08,
        #[default]
        Five = 0x0c,
    }

    #[derive(Debug, Copy, Clone, Default)]
    #[repr(u8)]
    pub enum BonusLife {
        Score10000 = 0x00,
        Score15000 = 0x10,
        Score20000 = 0x20,
        #[default]
        None = 0x30,
    }

    #[derive(Debug, Copy, Clone, Default)]
    #[repr(u8)]
    pub enum Difficulty {
        Hard = 0x00,
        #[default]
        Normal = 0x40,
    }

    #[derive(Debug, Copy, Clone, Default)]
    #[repr(u8)]
    pub enum GhostNames {
        Alternate = 0x00,
        #[default]
        Normal = 0x80,
    }
}

#[derive(Default)]
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
