pub struct Memory {
    ram: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            ram: vec![0; 0x1000],
        }
    }

    pub fn read(&self, address: u16, value: u8) -> u8 {
        let address = address & 0x7fff;
        if address >= 0x4000 && address < 0x5000 {
            self.ram[(address & 0xfff) as usize]
        } else {
            value
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let address = address & 0x7fff;
        if address >= 0x4000 && address < 0x5000 {
            self.ram[(address & 0xfff) as usize] = value;
        }
    }

    pub fn nt_ram(&self) -> &[u8] {
        &self.ram[0..0x400]
    }

    pub fn attr_ram(&self) -> &[u8] {
        &self.ram[0x400..0x800]
    }

    pub fn spr_ram(&self) -> &[u8] {
        &self.ram[0xff0..0x1000]
    }
}
