pub struct BaseRom {
    color: [u8; 0x100],
    palette: [u8; 0x20],
    prg: Prg,
    chr: [[u8; 0x1000]; 2],
    snd: [[u8; 0x100]; 2],
}

impl BaseRom {
    pub fn read(&mut self, address: u16, value: u8) -> u8 {
        let address = address & 0x7fff;
        if address < 0x4000 {
            self.prg[address]
        } else {
            value
        }
    }

    pub fn chr(&self, address: u16) -> u8 {
        let chip = (address >> 12) & 1;
        let address = address & 0xfff;

        self.chr[chip as usize][address as usize]
    }

    pub fn snd(&self, address: u16) -> u8 {
        let chip = (address >> 8) & 1;
        let address = address & 0xff;

        self.snd[chip as usize][address as usize]
    }

    pub fn color(&self, address: u8) -> u8 {
        self.color[address as usize]
    }

    pub fn palette(&self, address: u8) -> u8 {
        let address = address & 0x1f;
        self.palette[address as usize]
    }
}

#[derive(Debug, Clone)]
struct Prg([[u8; 0x1000]; 4]);

impl std::ops::Index<u16> for Prg {
    type Output = u8;

    fn index(&self, address: u16) -> &Self::Output {
        let address = address as usize;
        let chip = (address >> 12) & 3;
        let address = address & 0xfff;

        &self.0[chip][address]
    }
}

impl std::ops::IndexMut<u16> for Prg {
    fn index_mut(&mut self, address: u16) -> &mut Self::Output {
        let address = address as usize;
        let chip = (address >> 12) & 3;
        let address = address & 0xfff;

        &mut self.0[chip][address]
    }
}

struct DecryptPrg {
    base: Prg,
    prg_u5: [u8; 0x800],
    prg_u6: [u8; 0x1000],
    prg_u7: [u8; 0x1000],
}

impl DecryptPrg {
    fn read(&self, address: u16, value: u8) -> u8 {
        match address {
            0x0000..0x3000 => self.base[address],
            0x3000..0x4000 => {
                let addr = bit_swap_u16(address & 0xfff, [11, 3, 7, 9, 10, 8, 6, 5, 4, 2, 1, 0]);
                let v = self.prg_u7[addr as usize];
                bit_swap_u8(v, [0, 4, 5, 7, 6, 3, 2, 1])
            }
            0x8000..0x8800 => {
                let addr = bit_swap_u16(address & 0x7ff, [8, 7, 5, 9, 10, 6, 3, 4, 2, 1, 0]);
                let v = self.prg_u5[addr as usize];
                bit_swap_u8(v, [0, 4, 5, 7, 6, 3, 2, 1])
            }
            0x8800..0x9000 => {
                let addr = bit_swap_u16(address & 0x7ff, [3, 7, 9, 10, 8, 6, 5, 4, 2, 1, 0]);
                let v = self.prg_u6[0x800 | addr as usize];
                bit_swap_u8(v, [0, 4, 5, 7, 6, 3, 2, 1])
            }
            0x9000..0x9800 => {
                let addr = bit_swap_u16(address & 0x7ff, [3, 7, 9, 10, 8, 6, 5, 4, 2, 1, 0]);
                let v = self.prg_u6[addr as usize];
                bit_swap_u8(v, [0, 4, 5, 7, 6, 3, 2, 1])
            }
            0x9800..0xa000 => self.base[0x1800 | (address & 0x7ff)],
            0xa000..0xc000 => self.base[address & 0x3fff],
            _ => value,
        }
    }

    fn patch(&mut self, address: u16, value: u8) {
        self.base[address] = value;
    }
}

fn bit_swap_u8<const N: usize>(value: u8, bits: [u8; N]) -> u8 {
    let mut result = 0;
    for bit in bits {
        let b = (value >> bit) & 1;
        result <<= 1;
        result |= b;
    }
    result
}

fn bit_swap_u16<const N: usize>(value: u16, bits: [u8; N]) -> u16 {
    let mut result = 0;
    for bit in bits {
        let b = (value >> bit) & 1;
        result <<= 1;
        result |= b;
    }
    result
}

pub struct ExpansionRom {
    base: BaseRom,
    enable_decode: bool,
    patched_prg: DecryptPrg,
}

impl ExpansionRom {
    fn new(base: BaseRom, prg_u5: [u8; 0x800], prg_u6: [u8; 0x1000], prg_u7: [u8; 0x1000]) -> Self {
        let mut patched_prg = DecryptPrg {
            base: base.prg.clone(),
            prg_u5,
            prg_u6,
            prg_u7,
        };

        for i in 0..8 {
            patched_prg.patch(0x0410 + i, patched_prg.read(0x8008 + i, 0xff));
            patched_prg.patch(0x08e0 + i, patched_prg.read(0x81d8 + i, 0xff));
            patched_prg.patch(0x0a30 + i, patched_prg.read(0x8118 + i, 0xff));
            patched_prg.patch(0x0bd0 + i, patched_prg.read(0x80d8 + i, 0xff));
            patched_prg.patch(0x0c20 + i, patched_prg.read(0x8120 + i, 0xff));
            patched_prg.patch(0x0e58 + i, patched_prg.read(0x8168 + i, 0xff));
            patched_prg.patch(0x0ea8 + i, patched_prg.read(0x8198 + i, 0xff));

            patched_prg.patch(0x1000 + i, patched_prg.read(0x8020 + i, 0xff));
            patched_prg.patch(0x1008 + i, patched_prg.read(0x8010 + i, 0xff));
            patched_prg.patch(0x1288 + i, patched_prg.read(0x8098 + i, 0xff));
            patched_prg.patch(0x1348 + i, patched_prg.read(0x8048 + i, 0xff));
            patched_prg.patch(0x1688 + i, patched_prg.read(0x8088 + i, 0xff));
            patched_prg.patch(0x16b0 + i, patched_prg.read(0x8188 + i, 0xff));
            patched_prg.patch(0x16d8 + i, patched_prg.read(0x80c8 + i, 0xff));
            patched_prg.patch(0x16f8 + i, patched_prg.read(0x81c8 + i, 0xff));
            patched_prg.patch(0x19a8 + i, patched_prg.read(0x80a8 + i, 0xff));
            patched_prg.patch(0x19b8 + i, patched_prg.read(0x81a8 + i, 0xff));

            patched_prg.patch(0x2060 + i, patched_prg.read(0x8148 + i, 0xff));
            patched_prg.patch(0x2108 + i, patched_prg.read(0x8018 + i, 0xff));
            patched_prg.patch(0x21a0 + i, patched_prg.read(0x81a0 + i, 0xff));
            patched_prg.patch(0x2298 + i, patched_prg.read(0x80a0 + i, 0xff));
            patched_prg.patch(0x23e0 + i, patched_prg.read(0x80e8 + i, 0xff));
            patched_prg.patch(0x2418 + i, patched_prg.read(0x8000 + i, 0xff));
            patched_prg.patch(0x2448 + i, patched_prg.read(0x8058 + i, 0xff));
            patched_prg.patch(0x2470 + i, patched_prg.read(0x8140 + i, 0xff));
            patched_prg.patch(0x2488 + i, patched_prg.read(0x8080 + i, 0xff));
            patched_prg.patch(0x24b0 + i, patched_prg.read(0x8180 + i, 0xff));
            patched_prg.patch(0x24d8 + i, patched_prg.read(0x80c0 + i, 0xff));
            patched_prg.patch(0x24f8 + i, patched_prg.read(0x81c0 + i, 0xff));
            patched_prg.patch(0x2748 + i, patched_prg.read(0x8050 + i, 0xff));
            patched_prg.patch(0x2780 + i, patched_prg.read(0x8090 + i, 0xff));
            patched_prg.patch(0x27b8 + i, patched_prg.read(0x8190 + i, 0xff));
            patched_prg.patch(0x2800 + i, patched_prg.read(0x8028 + i, 0xff));
            patched_prg.patch(0x2b20 + i, patched_prg.read(0x8100 + i, 0xff));
            patched_prg.patch(0x2b30 + i, patched_prg.read(0x8110 + i, 0xff));
            patched_prg.patch(0x2bf0 + i, patched_prg.read(0x81d0 + i, 0xff));
            patched_prg.patch(0x2cc0 + i, patched_prg.read(0x80d0 + i, 0xff));
            patched_prg.patch(0x2cd8 + i, patched_prg.read(0x80e0 + i, 0xff));
            patched_prg.patch(0x2cf0 + i, patched_prg.read(0x81e0 + i, 0xff));
            patched_prg.patch(0x2d60 + i, patched_prg.read(0x8160 + i, 0xff));
        }

        Self {
            base,
            enable_decode: false,
            patched_prg,
        }
    }

    pub fn read(&mut self, address: u16, value: u8) -> u8 {
        self.trigger_latch(address);
        if !self.enable_decode {
            self.base.read(address, value)
        } else {
            self.patched_prg.read(address, value)
        }
    }

    pub fn write(&mut self, address: u16, _value: u8) {
        self.trigger_latch(address);
    }

    fn trigger_latch(&mut self, address: u16) {
        match address {
            0x0038..=0x003f
            | 0x03b0..=0x03b7
            | 0x1600..=0x1607
            | 0x2120..=0x2127
            | 0x3ff0..=0x3ff7
            | 0x8000..=0x8007
            | 0x97f0..=0x97f7 => {
                self.enable_decode = false;
            }
            0x3ff8..=0x3fff => {
                self.enable_decode = true;
            }
            _ => (),
        }
    }
}

impl std::ops::Deref for ExpansionRom {
    type Target = BaseRom;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

pub enum Rom {
    Base(BaseRom),
    Expansion(ExpansionRom),
}

impl Rom {
    pub fn read(&mut self, address: u16, value: u8) -> u8 {
        match self {
            Rom::Base(base) => base.read(address, value),
            Rom::Expansion(exp) => exp.read(address, value),
        }
    }
    pub fn write(&mut self, address: u16, value: u8) {
        match self {
            Rom::Expansion(exp) => exp.write(address, value),
            _ => (),
        }
    }
}

impl std::ops::Deref for Rom {
    type Target = BaseRom;

    fn deref(&self) -> &Self::Target {
        match self {
            Rom::Base(base) => base,
            Rom::Expansion(exp) => &exp.base,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Error {
    InvalidContainer,
    MissingFile(&'static str, usize),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidContainer => write!(
                f,
                "unexpected rom file name, available options: 'pacman' or 'mspacman'"
            ),
            Error::MissingFile(n, size) => {
                write!(f, "required file missing, name '{n}' length {size} bytes")
            }
        }
    }
}

pub enum RomBuilder {
    Base {
        color: Option<[u8; 0x100]>,
        palette: Option<[u8; 0x20]>,
        prg: [Option<[u8; 0x1000]>; 4],
        chr: [Option<[u8; 0x1000]>; 2],
        snd: [Option<[u8; 0x100]>; 2],
    },
    Expansion {
        color: Option<[u8; 0x100]>,
        palette: Option<[u8; 0x20]>,
        prg: [Option<[u8; 0x1000]>; 4],
        chr: [Option<[u8; 0x1000]>; 2],
        snd: [Option<[u8; 0x100]>; 2],
        prg_u5: Option<[u8; 0x800]>,
        prg_u6: Option<[u8; 0x1000]>,
        prg_u7: Option<[u8; 0x1000]>,
    },
}

impl RomBuilder {
    pub fn new<S: AsRef<str>>(container_name: S) -> Result<Self, Error> {
        let container_name = container_name.as_ref();
        let builder = if container_name.starts_with("pacman") {
            RomBuilder::Base {
                color: None,
                palette: None,
                prg: [None; 4],
                chr: [None; 2],
                snd: [None; 2],
            }
        } else if container_name.starts_with("mspacman") {
            RomBuilder::Expansion {
                color: None,
                palette: None,
                prg: [None; 4],
                chr: [None; 2],
                snd: [None; 2],
                prg_u5: None,
                prg_u6: None,
                prg_u7: None,
            }
        } else {
            return Err(Error::InvalidContainer);
        };

        Ok(builder)
    }

    pub fn add_file<S: AsRef<str>, F: AsRef<[u8]>>(&mut self, name: S, file: F) {
        let name = name.as_ref();
        let file = file.as_ref();
        match self {
            RomBuilder::Base {
                color,
                palette,
                prg,
                chr,
                snd,
            } => match name {
                "82s126.4a" => *color = file.as_array().cloned(),
                "82s123.7f" => *palette = file.as_array().cloned(),
                "pacman.6e" => prg[0] = file.as_array().cloned(),
                "pacman.6f" => prg[1] = file.as_array().cloned(),
                "pacman.6h" => prg[2] = file.as_array().cloned(),
                "pacman.6j" => prg[3] = file.as_array().cloned(),
                "pacman.5e" => chr[0] = file.as_array().cloned(),
                "pacman.5f" => chr[1] = file.as_array().cloned(),
                "82s126.1m" => snd[0] = file.as_array().cloned(),
                "82s126.3m" => snd[1] = file.as_array().cloned(),
                _ => (),
            },
            RomBuilder::Expansion {
                color,
                palette,
                prg,
                chr,
                snd,
                prg_u5,
                prg_u6,
                prg_u7,
            } => match name {
                "82s126.4a" => *color = file.as_array().cloned(),
                "82s123.7f" => *palette = file.as_array().cloned(),
                "pacman.6e" => prg[0] = file.as_array().cloned(),
                "pacman.6f" => prg[1] = file.as_array().cloned(),
                "pacman.6h" => prg[2] = file.as_array().cloned(),
                "pacman.6j" => prg[3] = file.as_array().cloned(),
                "5e" => chr[0] = file.as_array().cloned(),
                "5f" => chr[1] = file.as_array().cloned(),
                "82s126.1m" => snd[0] = file.as_array().cloned(),
                "82s126.3m" => snd[1] = file.as_array().cloned(),
                "u5" => *prg_u5 = file.as_array().cloned(),
                "u6" => *prg_u6 = file.as_array().cloned(),
                "u7" => *prg_u7 = file.as_array().cloned(),
                _ => (),
            },
        }
    }

    pub fn build(self) -> Result<Rom, Error> {
        match self {
            RomBuilder::Base {
                color,
                palette,
                prg,
                chr,
                snd,
            } => {
                let color = unwrap_rom("82s126.4a", color)?;
                let palette = unwrap_rom("82s123.7f", palette)?;
                let prg = [
                    unwrap_rom("pacman.6e", prg[0])?,
                    unwrap_rom("pacman.6f", prg[1])?,
                    unwrap_rom("pacman.6h", prg[2])?,
                    unwrap_rom("pacman.6j", prg[3])?,
                ];
                let chr = [
                    unwrap_rom("pacman.5e", chr[0])?,
                    unwrap_rom("pacman.5f", chr[1])?,
                ];
                let snd = [
                    unwrap_rom("82s126.1m", snd[0])?,
                    unwrap_rom("82s126.3m", snd[1])?,
                ];

                let rom = BaseRom {
                    color,
                    palette,
                    prg: Prg(prg),
                    chr,
                    snd,
                };

                Ok(Rom::Base(rom))
            }
            RomBuilder::Expansion {
                color,
                palette,
                prg,
                chr,
                snd,
                prg_u5,
                prg_u6,
                prg_u7,
            } => {
                let color = unwrap_rom("82s126.4a", color)?;
                let palette = unwrap_rom("82s123.7f", palette)?;
                let prg = [
                    unwrap_rom("pacman.6e", prg[0])?,
                    unwrap_rom("pacman.6f", prg[1])?,
                    unwrap_rom("pacman.6h", prg[2])?,
                    unwrap_rom("pacman.6j", prg[3])?,
                ];
                let chr = [unwrap_rom("5e", chr[0])?, unwrap_rom("5f", chr[1])?];
                let snd = [
                    unwrap_rom("82s126.1m", snd[0])?,
                    unwrap_rom("82s126.3m", snd[1])?,
                ];
                let prg_u5 = unwrap_rom("u5", prg_u5)?;
                let prg_u6 = unwrap_rom("u6", prg_u6)?;
                let prg_u7 = unwrap_rom("u7", prg_u7)?;

                let base = BaseRom {
                    color,
                    palette,
                    prg: Prg(prg),
                    chr,
                    snd,
                };

                let rom = ExpansionRom::new(base, prg_u5, prg_u6, prg_u7);

                Ok(Rom::Expansion(rom))
            }
        }
    }
}

fn unwrap_rom<const N: usize>(name: &'static str, rom: Option<[u8; N]>) -> Result<[u8; N], Error> {
    rom.ok_or(Error::MissingFile(name, N))
}
