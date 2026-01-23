use save_states::SaveState;

use super::CPU_CLOCK;
use super::mem::Memory;
use super::rom::BaseRom;

#[derive(Debug, Copy, Clone, Default)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

const VAL_0220_OHM: u8 = 0x97;
const VAL_0470_OHM: u8 = 0x47;
const VAL_1000_OHM: u8 = 0x21;

impl From<u8> for Color {
    fn from(value: u8) -> Self {
        let mut r = 0;
        let mut g = 0;
        let mut b = 0;

        fn update_color<const N: u8>(value: u8, channel: &mut u8, ohms: u8) {
            if (value & (1 << N)) != 0 {
                *channel += ohms;
            }
        }

        update_color::<0>(value, &mut r, VAL_1000_OHM);
        update_color::<1>(value, &mut r, VAL_0470_OHM);
        update_color::<2>(value, &mut r, VAL_0220_OHM);
        update_color::<3>(value, &mut g, VAL_1000_OHM);
        update_color::<4>(value, &mut g, VAL_0470_OHM);
        update_color::<5>(value, &mut g, VAL_0220_OHM);
        update_color::<6>(value, &mut b, VAL_0470_OHM);
        update_color::<7>(value, &mut b, VAL_0220_OHM);

        Color { r, g, b }
    }
}

const FRAME_RATE: u64 = 60;
const FRAME_TICKS: u64 = CPU_CLOCK / FRAME_RATE;

#[derive(SaveState)]
pub struct Video {
    ticks: u64,
    #[save(skip)]
    rgb_colors: [Color; 0x20],
    #[save(skip)]
    tile_map: [u16; 0x480],
    spr_data: [u8; 0x10],
    #[save(skip)]
    screen: RawImage,
    interrupt_enable: bool,
    interrupt_data: u8,
    interrupt_pending: bool,
    frame: u32,
}

impl Video {
    pub fn new(rom: &BaseRom) -> Self {
        let mut cursor = 0;
        let mut tile_map = [0x400; _];
        let mut set = |x: usize, y: usize| {
            let idx = y * 36 + x;
            tile_map[idx] = cursor;
            cursor += 1;
        };

        for x in [1, 0] {
            for y in (0..32).rev() {
                set(x, y);
            }
        }
        for y in (2..30).rev() {
            for x in (2..34).rev() {
                set(x, y)
            }
        }
        for x in [35, 34] {
            for y in (0..32).rev() {
                set(x, y);
            }
        }

        let rgb_colors = std::array::from_fn(|idx| rom.palette(idx as u8).into());
        let spr_data = [0; 0x10];
        let screen = RawImage::new(36 * 8, 28 * 8, 16);

        Self {
            ticks: 0,
            rgb_colors,
            tile_map,
            spr_data,
            screen,
            interrupt_enable: false,
            interrupt_data: 0,
            interrupt_pending: false,
            frame: 0,
        }
    }

    pub fn screen(&self) -> &[u8] {
        &self.screen.pixels
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }

    fn bg_pixel(&self, rom: &BaseRom, pattern: u8, color: u8, x: u8, y: u8) -> Color {
        let tile = pattern as u16;
        let x = (x as u16) & 7;
        let y = (7 - y as u16) & 7;

        let idx = tile << 4 | (x & 4) << 1 | y;
        let entry = rom.chr(idx);

        let bit_0 = x & 3;
        let bit_1 = bit_0 + 4;

        let mut color = color << 2;
        color |= (entry >> bit_0) & 1;
        color |= (entry >> (bit_1 - 1)) & 2;

        let pal = rom.color(color) & 0x1f;
        self.rgb_colors[pal as usize]
    }

    fn spr_pixel(&self, rom: &BaseRom, pattern: u8, color: u8, x: u8, y: u8) -> Option<Color> {
        let tile = (pattern as u16) & 0xfc;
        let x = (((15 - x as u16) & 0xf) + 4) & 0xf;
        let y = (15 - y as u16) & 0xf;

        let idx = 0x1000 | tile << 4 | (x & 0xc) << 1 | (y & 0x8) << 2 | (y & 0x7);
        let entry = rom.chr(idx);

        let bit_0 = (x ^ 3) & 3;
        let bit_1 = bit_0 + 4;

        let mut color = color << 2;
        color |= (entry >> bit_0) & 1;
        color |= (entry >> (bit_1 - 1)) & 2;

        let pal = rom.color(color) & 0x1f;
        if pal == 0 {
            None
        } else {
            Some(self.rgb_colors[pal as usize])
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address & 0x7fff {
            0x5000 => self.interrupt_enable = value != 0,
            0x5060..0x5070 => {
                let idx = address & 0x0f;
                self.spr_data[idx as usize] = value;
            }
            _ => (),
        }
    }

    pub fn io_write(&mut self, address: u16, value: u8) {
        if address & 0xff == 0 {
            self.interrupt_data = value;
        }
    }

    pub fn interrupt_ack(&self) -> u8 {
        self.interrupt_data
    }

    pub fn interrupt_req(&mut self) -> bool {
        self.interrupt_pending
    }

    pub fn tick(&mut self, rom: &BaseRom, mem: &Memory) {
        self.interrupt_pending = false;
        self.ticks += 1;
        if self.ticks >= FRAME_TICKS {
            self.ticks -= FRAME_TICKS;
            if self.interrupt_enable {
                self.interrupt_pending = true;
            }
            self.render_frame(rom, mem);
            self.frame += 1;
        }
    }

    pub fn render_frame(&mut self, rom: &BaseRom, mem: &Memory) {
        for tile_x in 0..36 {
            for tile_y in 2..30 {
                let tile = (tile_y * 36 + tile_x) as u16;
                let idx = self.tile_map[tile as usize];
                if idx == 0x400 {
                    continue;
                }
                let pattern = mem.nt_ram()[idx as usize];
                let color = mem.attr_ram()[idx as usize];

                for x in 0..8 {
                    for y in 0..8 {
                        let color = self.bg_pixel(rom, pattern, color, x as u8, y as u8);
                        let pixel_x = tile_x * 8 + x;
                        let pixel_y = tile_y * 8 + y;
                        self.screen.set_pixel(pixel_x, pixel_y, color);
                    }
                }
            }
        }

        let (attrs, _) = mem.spr_ram().as_chunks::<2>();
        let (coords, _) = self.spr_data.as_chunks::<2>();

        let sprites = attrs.iter().zip(coords.iter()).enumerate().rev();

        for (id, (&[shape, color], &[spr_y, spr_x])) in sprites {
            let mirroring: Mirroring = shape.into();
            let (x_flip, y_flip) = mirroring.mirror();

            let spr_x = spr_x;
            let spr_y = 255 - spr_y;
            let spr_y = if id <= 2 {
                spr_y.wrapping_sub(1)
            } else {
                spr_y
            };

            for x in 0..16 {
                for y in 0..16 {
                    let pixel_x = x + (spr_x as usize);
                    let pixel_y = y + (spr_y as usize);
                    if pixel_y < 16 || pixel_y >= 240 {
                        continue;
                    }

                    let color = {
                        let x = if x_flip { 0xf - x } else { x };
                        let y = if y_flip { 0xf - y } else { y };
                        self.spr_pixel(rom, shape, color, x as u8, y as u8)
                    };

                    let Some(color) = color else {
                        continue;
                    };

                    self.screen.set_pixel(pixel_x, pixel_y, color);
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Mirroring {
    None,
    Horz,
    Vert,
    Both,
}

impl Mirroring {
    fn mirror(self) -> (bool, bool) {
        match self {
            Mirroring::None => (false, false),
            Mirroring::Horz => (true, false),
            Mirroring::Vert => (false, true),
            Mirroring::Both => (true, true),
        }
    }
}

impl From<u8> for Mirroring {
    fn from(value: u8) -> Self {
        match value & 3 {
            0 => Mirroring::None,
            1 => Mirroring::Horz,
            2 => Mirroring::Vert,
            3 => Mirroring::Both,
            _ => unreachable!(),
        }
    }
}

struct RawImage {
    width: usize,
    height: usize,
    y_offset: usize,
    pixels: Vec<u8>,
}

impl RawImage {
    fn new(width: usize, height: usize, y_offset: usize) -> Self {
        Self {
            width,
            height,
            y_offset,
            pixels: vec![0; width * height * 3],
        }
    }

    fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if y < self.y_offset || x >= self.width {
            return;
        }

        let y = y - self.y_offset;

        if y >= self.height {
            return;
        }

        let idx = (y * self.width + x) * 3;
        self.pixels[idx + 0] = color.r;
        self.pixels[idx + 1] = color.g;
        self.pixels[idx + 2] = color.b;
    }
}
