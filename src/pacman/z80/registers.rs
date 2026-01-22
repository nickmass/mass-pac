use std::ops::{Deref, DerefMut};

macro_rules! define_flag {
    ($name:ident, $name_mut:ident, $bit:literal$( , self.$reg:ident)?) => {
        #[inline(always)]
        pub fn $name_mut(&mut self) -> Flag<$bit, &mut u8> {
            #[allow(unused)]
            let reg = &mut self.regs[Reg8::F as usize];
            $(let reg = &mut self.$reg;)?
            Flag(reg)
        }

        #[inline(always)]
        pub fn $name(&self) -> bool {
            #[allow(unused)]
            let reg = &self.regs[Reg8:: F as usize];
            $(let reg = &self.$reg;)?
            Flag::<$bit, _>(reg).get()
        }
    };
}

#[derive(Default, Clone)]
#[repr(C, align(2))]
struct RegArray {
    regs: [u8; 26],
}

impl std::ops::Deref for RegArray {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.regs
    }
}

impl std::ops::DerefMut for RegArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.regs
    }
}

#[derive(Default, Clone)]
pub struct Registers {
    regs: RegArray,
    iff: u8,
    index_mode: IndexMode,
    index_offset: u8,
}

impl Registers {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn get<R: RegisterSize>(&self, reg: R) -> R::Output {
        let reg = reg.swap_idx(self.index_mode);
        reg.get(&self.regs)
    }

    #[inline(always)]
    pub fn get_idx_off<R: RegisterSize>(&self, reg: R) -> R::Output {
        reg.get_idx_off(&self.regs, self.index_mode, self.index_offset)
    }

    #[inline(always)]
    pub fn set<R: RegisterSize>(&mut self, reg: R, value: R::Output) {
        let reg = reg.swap_idx(self.index_mode);
        reg.set(&mut self.regs, value);
    }

    #[inline(always)]
    pub fn inc<R: RegisterSize>(&mut self, reg: R) {
        let reg = reg.swap_idx(self.index_mode);
        reg.inc(&mut self.regs);
    }

    #[inline(always)]
    pub fn dec<R: RegisterSize>(&mut self, reg: R) {
        let reg = reg.swap_idx(self.index_mode);
        reg.dec(&mut self.regs);
    }

    #[inline(always)]
    pub fn pc(&self) -> u16 {
        self.get(Reg16::PC)
    }

    define_flag!(flag_c, flag_c_mut, 0);
    define_flag!(flag_n, flag_n_mut, 1);
    define_flag!(flag_v, flag_v_mut, 2);
    define_flag!(flag_h, flag_h_mut, 4);
    define_flag!(flag_z, flag_z_mut, 6);
    define_flag!(flag_s, flag_s_mut, 7);
    define_flag!(flag_iff1, flag_iff1_mut, 0, self.iff);
    define_flag!(flag_iff2, flag_iff2_mut, 1, self.iff);

    pub fn set_flags_szv(&mut self, value: u8) {
        self.flag_s_mut().value(value & 0x80 != 0);
        self.flag_z_mut().value(value == 0);
        self.flag_v_mut().value(value.count_ones() & 1 == 0);
    }

    pub fn index_mode(&mut self, mode: IndexMode) {
        self.index_mode = mode;
    }

    pub fn is_indexing(&self) -> bool {
        match self.index_mode {
            IndexMode::HL => false,
            _ => true,
        }
    }

    pub fn index_offset(&mut self, offset: u8) {
        self.index_offset = offset;
    }

    pub fn no_idx(&mut self) -> NoIdx<'_> {
        NoIdx::new(self)
    }
}

pub struct NoIdx<'a> {
    old_index_mode: IndexMode,
    registers: &'a mut Registers,
}

impl<'a> NoIdx<'a> {
    fn new(registers: &'a mut Registers) -> Self {
        let old_index_mode = registers.index_mode;
        registers.index_mode = IndexMode::HL;

        Self {
            old_index_mode,
            registers,
        }
    }
}

impl<'a> std::ops::Drop for NoIdx<'a> {
    fn drop(&mut self) {
        self.registers.index_mode = self.old_index_mode;
    }
}

impl<'a> std::ops::Deref for NoIdx<'a> {
    type Target = Registers;

    fn deref(&self) -> &Self::Target {
        &self.registers
    }
}

impl<'a> std::ops::DerefMut for NoIdx<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.registers
    }
}

pub trait RegisterSize {
    type Output: Copy;
    fn get(self, regs: &[u8]) -> Self::Output;
    fn get_idx_off(self, regs: &[u8], index_mode: IndexMode, index_offset: u8) -> Self::Output;
    fn set(self, regs: &mut [u8], value: Self::Output);
    fn inc(self, regs: &mut [u8]);
    fn dec(self, regs: &mut [u8]);
    fn swap_idx(self, index_mode: IndexMode) -> Self;
}

#[repr(u8)]
#[allow(non_camel_case_types, unused)]
#[cfg(target_endian = "little")]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Reg8 {
    F,
    A,
    C,
    B,
    E,
    D,
    L,
    H,
    F_,
    A_,
    C_,
    B_,
    E_,
    D_,
    L_,
    H_,
    IX_L,
    IX_H,
    IY_L,
    IY_H,
    SP_L,
    SP_H,
    R,
    I,
    PC_L,
    PC_H,
}

#[repr(u8)]
#[allow(non_camel_case_types, unused)]
#[cfg(target_endian = "big")]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Reg8 {
    A,
    F,
    B,
    C,
    D,
    E,
    H,
    L,
    A_,
    F_,
    B_,
    C_,
    D_,
    E_,
    H_,
    L_,
    IX_H,
    IX_L,
    IY_H,
    IY_L,
    SP_H,
    SP_L,
    I,
    R,
    PC_H,
    PC_L,
}

impl RegisterSize for Reg8 {
    type Output = u8;

    #[inline(always)]
    fn get(self, regs: &[u8]) -> Self::Output {
        regs[self as usize]
    }

    #[inline(always)]
    fn get_idx_off(self, regs: &[u8], index_mode: IndexMode, index_offset: u8) -> Self::Output {
        let (reg, offset) = match index_mode {
            IndexMode::HL => (self, 0),
            IndexMode::IX => match self {
                Reg8::H => (Reg8::IX_H, index_offset),
                Reg8::L => (Reg8::IX_L, index_offset),
                _ => (self, 0),
            },
            IndexMode::IY => match self {
                Reg8::H => (Reg8::IY_H, index_offset),
                Reg8::L => (Reg8::IY_L, index_offset),
                _ => (self, 0),
            },
        };

        reg.get(regs).wrapping_add_signed(offset as i8)
    }

    #[inline(always)]
    fn set(self, regs: &mut [u8], value: Self::Output) {
        regs[self as usize] = value;
    }

    #[inline(always)]
    fn inc(self, regs: &mut [u8]) {
        let value = regs[self as usize].wrapping_add(1);
        regs[self as usize] = value;
    }

    #[inline(always)]
    fn dec(self, regs: &mut [u8]) {
        let value = regs[self as usize].wrapping_sub(1);
        regs[self as usize] = value;
    }

    #[inline(always)]
    fn swap_idx(self, index_mode: IndexMode) -> Self {
        match index_mode {
            IndexMode::HL => self,
            IndexMode::IX => match self {
                Reg8::H => Reg8::IX_H,
                Reg8::L => Reg8::IX_L,
                _ => self,
            },
            IndexMode::IY => match self {
                Reg8::H => Reg8::IY_H,
                Reg8::L => Reg8::IY_L,
                _ => self,
            },
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(unused)]
pub enum Reg16 {
    AF,
    BC,
    DE,
    HL,
    AF_,
    BC_,
    DE_,
    HL_,
    IX,
    IY,
    SP,
    RI,
    PC,
}

impl RegisterSize for Reg16 {
    type Output = u16;

    #[inline(always)]
    fn get(self, regs: &[u8]) -> Self::Output {
        let regs = unsafe {
            std::slice::from_raw_parts(
                regs.as_ptr() as *const Self::Output,
                regs.len() / std::mem::size_of::<Self::Output>(),
            )
        };
        regs[self as usize]
    }

    #[inline(always)]
    fn get_idx_off(self, regs: &[u8], index_mode: IndexMode, index_offset: u8) -> Self::Output {
        let (reg, offset) = match index_mode {
            IndexMode::HL => (self, 0),
            IndexMode::IX => match self {
                Reg16::HL => (Reg16::IX, index_offset),
                _ => (self, 0),
            },
            IndexMode::IY => match self {
                Reg16::HL => (Reg16::IY, index_offset),
                _ => (self, 0),
            },
        };

        reg.get(regs).wrapping_add_signed(offset as i8 as i16)
    }

    #[inline(always)]
    fn set(self, regs: &mut [u8], value: Self::Output) {
        let regs = unsafe {
            std::slice::from_raw_parts_mut(
                regs.as_ptr() as *mut Self::Output,
                regs.len() / std::mem::size_of::<Self::Output>(),
            )
        };
        regs[self as usize] = value;
    }

    #[inline(always)]
    fn inc(self, regs: &mut [u8]) {
        let regs = unsafe {
            std::slice::from_raw_parts_mut(
                regs.as_ptr() as *mut Self::Output,
                regs.len() / std::mem::size_of::<Self::Output>(),
            )
        };
        let value = regs[self as usize].wrapping_add(1);
        regs[self as usize] = value;
    }

    #[inline(always)]
    fn dec(self, regs: &mut [u8]) {
        let regs = unsafe {
            std::slice::from_raw_parts_mut(
                regs.as_ptr() as *mut Self::Output,
                regs.len() / std::mem::size_of::<Self::Output>(),
            )
        };
        let value = regs[self as usize].wrapping_sub(1);
        regs[self as usize] = value;
    }

    #[inline(always)]
    fn swap_idx(self, index_mode: IndexMode) -> Self {
        match index_mode {
            IndexMode::HL => self,
            IndexMode::IX => match self {
                Reg16::HL => Reg16::IX,
                _ => self,
            },
            IndexMode::IY => match self {
                Reg16::HL => Reg16::IY,
                _ => self,
            },
        }
    }
}

pub struct Flag<const N: u8, T>(T);

impl<const N: u8, T: Deref<Target = u8>> Flag<N, T> {
    const MASK: u8 = 1 << N;

    #[inline(always)]
    pub fn get(&self) -> bool {
        *self.0 & Self::MASK != 0
    }
}

impl<const N: u8, T: DerefMut<Target = u8>> Flag<N, T> {
    #[inline(always)]
    pub fn set(&mut self) {
        *self.0 |= Self::MASK;
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        *self.0 &= !Self::MASK;
    }

    #[inline(always)]
    pub fn toggle(&mut self) {
        let others = *self.0 & !Self::MASK;
        let flag = (*self.0 ^ 0xff) & Self::MASK;
        *self.0 = others | flag;
    }

    #[inline(always)]
    pub fn value(&mut self, value: bool) {
        if value {
            self.set();
        } else {
            self.reset();
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum IndexMode {
    HL,
    IX,
    IY,
}

impl Default for IndexMode {
    fn default() -> Self {
        Self::HL
    }
}
