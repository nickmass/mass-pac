use super::{InstructionPrefix, Reg8, Reg16, Registers};

#[derive(Debug, Copy, Clone)]
pub enum PreInst {
    None(Inst),
    ED(InstED),
    CB(InstCB),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Inst {
    Add16(LoadLoc16),
    Alu(Alu, LoadLoc8),
    BitAlu(BitAlu, StoreLoc8, LoadLoc8),
    Call,
    CallCond(Cond),
    Ccf,
    Cpl,
    Daa,
    Dec16(StoreLoc16, LoadLoc16),
    Dec8(StoreLoc8, LoadLoc8),
    Di,
    Djnz,
    Ei,
    Ex(Reg16, Reg16),
    ExSP,
    Exx,
    Halt,
    In(LoadLoc8),
    Inc16(StoreLoc16, LoadLoc16),
    Inc8(StoreLoc8, LoadLoc8),
    Jp(LoadLoc16),
    JpCond(Cond),
    Jr,
    JrCond(Cond),
    Ld16(StoreLoc16, LoadLoc16),
    Ld8(StoreLoc8, LoadLoc8),
    Nop,
    Out(LoadLoc8),
    Pop(StoreLoc16),
    PrefixCB,
    PrefixED,
    PrefixIX,
    PrefixIY,
    Push(LoadLoc16),
    Ret,
    RetCond(Cond),
    Rst(u8),
    Scf,
    Unknown,
}

impl Default for Inst {
    fn default() -> Self {
        Inst::Unknown
    }
}

impl Inst {
    pub fn uses_index_offset(&self) -> bool {
        match self {
            Inst::Ld8(StoreLoc8::RegIndirect(Reg16::HL), _) => true,
            Inst::Ld8(_, LoadLoc8::RegIndirect(Reg16::HL)) => true,
            Inst::Inc8(StoreLoc8::RegIndirect(Reg16::HL), _) => true,
            Inst::Inc8(_, LoadLoc8::RegIndirect(Reg16::HL)) => true,
            Inst::Dec8(StoreLoc8::RegIndirect(Reg16::HL), _) => true,
            Inst::Dec8(_, LoadLoc8::RegIndirect(Reg16::HL)) => true,
            Inst::Alu(_, LoadLoc8::RegIndirect(Reg16::HL)) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InstED {
    Adc16(LoadLoc16),
    CpBlock(Repeat, Direction),
    Im(InterruptMode),
    In(StoreLoc8),
    InBlock(Repeat, Direction),
    Ld16(StoreLoc16, LoadLoc16),
    Ld8(StoreLoc8, LoadLoc8),
    Ld8Flags(StoreLoc8, LoadLoc8),
    LdBlock(Repeat, Direction),
    Neg,
    Out(LoadLoc8),
    OutBlock(Repeat, Direction),
    Reti,
    Retn,
    Rld,
    Rrd,
    Sbc16(LoadLoc16),
    Unknown,
}

impl Default for InstED {
    fn default() -> Self {
        InstED::Unknown
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InstCB {
    Alu(BitAlu, StoreLoc8, LoadLoc8),
    Bit(u8, LoadLoc8),
    Set(u8, StoreLoc8, LoadLoc8),
    Reset(u8, StoreLoc8, LoadLoc8),
    Unknown,
}

impl Default for InstCB {
    fn default() -> Self {
        InstCB::Unknown
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StoreLoc8 {
    Reg(Reg8),
    RegIndirect(Reg16),
    RegNoIdx(Reg8),
    OperandIndirect,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoadLoc8 {
    Reg(Reg8),
    RegIndirect(Reg16),
    RegNoIdx(Reg8),
    Operand,
    OperandIndirect,
}

impl StoreLoc8 {
    fn reg_indirect<const B: u8>(loc: OpLoc16<B>) -> Self {
        StoreLoc8::RegIndirect(loc.into())
    }
}

impl<const B: u8> From<OpLoc8<B>> for StoreLoc8 {
    fn from(value: OpLoc8<B>) -> Self {
        match value {
            OpLoc8::RegB => StoreLoc8::Reg(Reg8::B),
            OpLoc8::RegC => StoreLoc8::Reg(Reg8::C),
            OpLoc8::RegD => StoreLoc8::Reg(Reg8::D),
            OpLoc8::RegE => StoreLoc8::Reg(Reg8::E),
            OpLoc8::RegH => StoreLoc8::Reg(Reg8::H),
            OpLoc8::RegL => StoreLoc8::Reg(Reg8::L),
            OpLoc8::Indirect => StoreLoc8::RegIndirect(Reg16::HL),
            OpLoc8::RegA => StoreLoc8::Reg(Reg8::A),
        }
    }
}

impl LoadLoc8 {
    fn reg_indirect<const B: u8>(loc: OpLoc16<B>) -> Self {
        LoadLoc8::RegIndirect(loc.into())
    }
}

impl<const B: u8> From<OpLoc8<B>> for LoadLoc8 {
    fn from(value: OpLoc8<B>) -> Self {
        match value {
            OpLoc8::RegB => LoadLoc8::Reg(Reg8::B),
            OpLoc8::RegC => LoadLoc8::Reg(Reg8::C),
            OpLoc8::RegD => LoadLoc8::Reg(Reg8::D),
            OpLoc8::RegE => LoadLoc8::Reg(Reg8::E),
            OpLoc8::RegH => LoadLoc8::Reg(Reg8::H),
            OpLoc8::RegL => LoadLoc8::Reg(Reg8::L),
            OpLoc8::Indirect => LoadLoc8::RegIndirect(Reg16::HL),
            OpLoc8::RegA => LoadLoc8::Reg(Reg8::A),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StoreLoc16 {
    Reg(Reg16),
    OperandIndirect,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LoadLoc16 {
    Reg(Reg16),
    Operand,
    OperandIndirect,
}

impl StoreLoc16 {
    fn reg<const B: u8>(loc: OpLoc16<B>) -> Self {
        StoreLoc16::Reg(loc.into())
    }
}

impl LoadLoc16 {
    fn reg<const B: u8>(loc: OpLoc16<B>) -> Self {
        LoadLoc16::Reg(loc.into())
    }
}

impl<const B: u8> From<OpLoc16<B>> for Reg16 {
    fn from(value: OpLoc16<B>) -> Self {
        match value {
            OpLoc16::RegBC => Reg16::BC,
            OpLoc16::RegDE => Reg16::DE,
            OpLoc16::RegHL => Reg16::HL,
            OpLoc16::RegSP => Reg16::SP,
            OpLoc16::RegAF => Reg16::AF,
        }
    }
}

pub struct Instructions {
    insts: [Inst; 256],
    insts_ed: [InstED; 256],
    insts_cb: [InstCB; 256],
}

impl Instructions {
    pub fn new() -> Self {
        let insts = load_insts();
        let insts_ed = load_insts_ed();
        let insts_cb = load_insts_cb();
        Self {
            insts,
            insts_ed,
            insts_cb,
        }
    }

    #[inline(always)]
    pub fn lookup(&self, opcode: u8, prefix: InstructionPrefix) -> PreInst {
        match prefix {
            InstructionPrefix::None => PreInst::None(self.insts[opcode as usize]),
            InstructionPrefix::ED => PreInst::ED(self.insts_ed[opcode as usize]),
            InstructionPrefix::CB => PreInst::CB(self.insts_cb[opcode as usize]),
        }
    }
}

fn load_insts() -> [Inst; 256] {
    let mut insts = InstructionBuilder::new();

    insts.push(0x00, Inst::Nop);
    insts.extend((0x01, OpLoc16::<4>::all_sp()), |(_, d)| {
        Inst::Ld16(StoreLoc16::reg(d), LoadLoc16::Operand)
    });
    insts.extend((0x02, [OpLoc16::<4>::RegBC, OpLoc16::RegDE]), |(_, d)| {
        Inst::Ld8(StoreLoc8::reg_indirect(d), LoadLoc8::Reg(Reg8::A))
    });
    insts.extend((0x03, OpLoc16::<4>::all_sp()), |(_, r)| {
        Inst::Inc16(StoreLoc16::reg(r), LoadLoc16::reg(r))
    });
    insts.extend((0x04, OpLoc8::<3>::all()), |(_, d)| {
        Inst::Inc8(d.into(), d.into())
    });
    insts.extend((0x05, OpLoc8::<3>::all()), |(_, d)| {
        Inst::Dec8(d.into(), d.into())
    });
    insts.extend((0x06, OpLoc8::<3>::all()), |(_, d)| {
        Inst::Ld8(d.into(), LoadLoc8::Operand)
    });
    insts.extend(
        (0x07, [BitAlu::Rlc, BitAlu::Rrc, BitAlu::Rl, BitAlu::Rr]),
        |(_, alu)| Inst::BitAlu(alu, StoreLoc8::Reg(Reg8::A), LoadLoc8::Reg(Reg8::A)),
    );
    insts.extend((0x09, OpLoc16::<4>::all_sp()), |(_, s)| {
        Inst::Add16(LoadLoc16::reg(s))
    });
    insts.extend((0x0a, [OpLoc16::<4>::RegBC, OpLoc16::RegDE]), |(_, s)| {
        Inst::Ld8(StoreLoc8::Reg(Reg8::A), LoadLoc8::reg_indirect(s))
    });
    insts.extend((0x0b, OpLoc16::<4>::all_sp()), |(_, r)| {
        Inst::Dec16(StoreLoc16::reg(r), LoadLoc16::reg(r))
    });
    insts.push(0x08, Inst::Ex(Reg16::AF, Reg16::AF_));
    insts.push(0x10, Inst::Djnz);
    insts.push(0x18, Inst::Jr);
    insts.extend(
        (0x20, [Cond::NoZero, Cond::Zero, Cond::NoCarry, Cond::Carry]),
        |(_, cond)| Inst::JrCond(cond),
    );
    insts.push(
        0x22,
        Inst::Ld16(StoreLoc16::OperandIndirect, LoadLoc16::Reg(Reg16::HL)),
    );
    insts.push(0x27, Inst::Daa);
    insts.push(
        0x2a,
        Inst::Ld16(StoreLoc16::Reg(Reg16::HL), LoadLoc16::OperandIndirect),
    );
    insts.push(0x2f, Inst::Cpl);
    insts.push(
        0x32,
        Inst::Ld8(StoreLoc8::OperandIndirect, LoadLoc8::Reg(Reg8::A)),
    );
    insts.push(0x37, Inst::Scf);
    insts.push(
        0x3a,
        Inst::Ld8(StoreLoc8::Reg(Reg8::A), LoadLoc8::OperandIndirect),
    );
    insts.push(0x3f, Inst::Ccf);
    insts.extend(
        (0x40, OpLoc8::<3>::all(), OpLoc8::<0>::all()),
        |(_, d, s)| Inst::Ld8(d.into(), s.into()),
    );
    insts.extend((0x80, Alu::all(), OpLoc8::<0>::all()), |(_, op, s)| {
        Inst::Alu(op, s.into())
    });
    insts.extend((0xc0, Cond::all()), |(_, cond)| Inst::RetCond(cond));
    insts.extend((0xc1, OpLoc16::<4>::all_af()), |(_, r)| {
        Inst::Pop(StoreLoc16::reg(r))
    });
    insts.extend((0xc2, Cond::all()), |(_, cond)| Inst::JpCond(cond));
    insts.push(0xc3, Inst::Jp(LoadLoc16::Operand));
    insts.extend((0xc4, Cond::all()), |(_, cond)| Inst::CallCond(cond));
    insts.extend((0xc5, OpLoc16::<4>::all_af()), |(_, r)| {
        Inst::Push(LoadLoc16::reg(r))
    });
    insts.extend((0xc6, Alu::all()), |(_, op)| {
        Inst::Alu(op, LoadLoc8::Operand)
    });
    insts.extend((0xc7, bits::<3>()), |(_, addr)| Inst::Rst(addr));
    insts.push(0xc9, Inst::Ret);
    insts.push(0xcb, Inst::PrefixCB);
    insts.push(0xcd, Inst::Call);
    insts.push(0xd3, Inst::Out(LoadLoc8::Operand));
    insts.push(0xd9, Inst::Exx);
    insts.push(0xdb, Inst::In(LoadLoc8::Operand));
    insts.push(0xdd, Inst::PrefixIX);
    insts.push(0xe3, Inst::ExSP);
    insts.push(0xe9, Inst::Jp(LoadLoc16::Reg(Reg16::HL)));
    insts.push(0xeb, Inst::Ex(Reg16::DE, Reg16::HL));
    insts.push(0xed, Inst::PrefixED);
    insts.push(0xf3, Inst::Di);
    insts.push(
        0xf9,
        Inst::Ld16(StoreLoc16::Reg(Reg16::SP), LoadLoc16::Reg(Reg16::HL)),
    );
    insts.push(0xfb, Inst::Ei);
    insts.push(0xfd, Inst::PrefixIY);

    insts.overwrite(0x76, Inst::Halt);
    insts.overwrite(
        0x66,
        Inst::Ld8(
            StoreLoc8::RegNoIdx(Reg8::H),
            LoadLoc8::RegIndirect(Reg16::HL),
        ),
    );
    insts.overwrite(
        0x6e,
        Inst::Ld8(
            StoreLoc8::RegNoIdx(Reg8::L),
            LoadLoc8::RegIndirect(Reg16::HL),
        ),
    );
    insts.overwrite(
        0x74,
        Inst::Ld8(
            StoreLoc8::RegIndirect(Reg16::HL),
            LoadLoc8::RegNoIdx(Reg8::H),
        ),
    );
    insts.overwrite(
        0x75,
        Inst::Ld8(
            StoreLoc8::RegIndirect(Reg16::HL),
            LoadLoc8::RegNoIdx(Reg8::L),
        ),
    );

    insts.build()
}

fn load_insts_ed() -> [InstED; 256] {
    let mut insts = InstructionBuilder::new();

    insts.extend((0x40, OpLoc8::<3>::no_indirect()), |(_, d)| {
        InstED::In(d.into())
    });
    insts.extend((0x41, OpLoc8::<3>::no_indirect()), |(_, s)| {
        InstED::Out(s.into())
    });
    insts.extend((0x42, OpLoc16::<4>::all_sp()), |(_, s)| {
        InstED::Sbc16(LoadLoc16::reg(s))
    });
    insts.extend((0x43, OpLoc16::<4>::all_sp()), |(_, s)| {
        InstED::Ld16(StoreLoc16::OperandIndirect, LoadLoc16::reg(s))
    });
    insts.push(0x44, InstED::Neg);
    insts.push(0x45, InstED::Retn);
    insts.push(0x46, InstED::Im(InterruptMode::Zero));
    insts.push(0x56, InstED::Im(InterruptMode::One));
    insts.push(0x5e, InstED::Im(InterruptMode::Two));
    insts.push(
        0x47,
        InstED::Ld8(StoreLoc8::Reg(Reg8::I), LoadLoc8::Reg(Reg8::A)),
    );
    insts.extend((0x4a, OpLoc16::<4>::all_sp()), |(_, s)| {
        InstED::Adc16(LoadLoc16::reg(s))
    });
    insts.extend((0x4b, OpLoc16::<4>::all_sp()), |(_, s)| {
        InstED::Ld16(StoreLoc16::reg(s), LoadLoc16::OperandIndirect)
    });
    insts.push(0x4d, InstED::Reti);
    insts.push(
        0x4f,
        InstED::Ld8(StoreLoc8::Reg(Reg8::R), LoadLoc8::Reg(Reg8::A)),
    );
    insts.push(
        0x57,
        InstED::Ld8Flags(StoreLoc8::Reg(Reg8::A), LoadLoc8::Reg(Reg8::I)),
    );
    insts.push(
        0x5f,
        InstED::Ld8Flags(StoreLoc8::Reg(Reg8::A), LoadLoc8::Reg(Reg8::R)),
    );
    insts.push(0x67, InstED::Rrd);
    insts.push(0x6f, InstED::Rld);
    insts.extend((0xa0, Repeat::all(), Direction::all()), |(_, r, d)| {
        InstED::LdBlock(r, d)
    });
    insts.extend((0xa1, Repeat::all(), Direction::all()), |(_, r, d)| {
        InstED::CpBlock(r, d)
    });
    insts.extend((0xa2, Repeat::all(), Direction::all()), |(_, r, d)| {
        InstED::InBlock(r, d)
    });
    insts.extend((0xa3, Repeat::all(), Direction::all()), |(_, r, d)| {
        InstED::OutBlock(r, d)
    });

    insts.build()
}

fn load_insts_cb() -> [InstCB; 256] {
    let mut insts = InstructionBuilder::new();

    insts.extend((0x00, BitAlu::all(), OpLoc8::<0>::all()), |(_, alu, r)| {
        InstCB::Alu(alu, r.into(), r.into())
    });
    insts.extend((0x40, bits::<3>(), OpLoc8::<0>::all()), |(_, bit, r)| {
        InstCB::Bit(bit >> 3, r.into())
    });
    insts.extend((0x80, bits::<3>(), OpLoc8::<0>::all()), |(_, bit, r)| {
        InstCB::Reset(bit >> 3, r.into(), r.into())
    });
    insts.extend((0xc0, bits::<3>(), OpLoc8::<0>::all()), |(_, bit, r)| {
        InstCB::Set(bit >> 3, r.into(), r.into())
    });

    insts.build()
}

struct InstructionBuilder<T> {
    insts: [T; 256],
}

impl<T: Default + Copy + std::fmt::Debug + PartialEq> InstructionBuilder<T> {
    fn new() -> Self {
        Self {
            insts: [T::default(); 256],
        }
    }

    fn push(&mut self, opcode: u8, inst: T) {
        assert_eq!(
            self.insts[opcode as usize],
            T::default(),
            "overlap for opcode: {:02x}",
            opcode
        );
        self.insts[opcode as usize] = inst;
    }

    fn overwrite(&mut self, opcode: u8, inst: T) {
        self.insts[opcode as usize] = inst;
    }

    fn extend<I: OpcodeIterator, F: Fn(I::Item) -> T>(&mut self, iter: I, map: F) {
        for (opcode, vars) in iter.iter() {
            let inst = map(vars);
            self.push(opcode, inst);
        }
    }

    fn build(self) -> [T; 256] {
        self.insts
    }
}

fn bits<const SHIFT: u8>() -> [u8; 8] {
    [0, 1, 2, 3, 4, 5, 6, 7].map(|b| b << SHIFT)
}

trait OpcodePart {
    fn bits(&self) -> u8;
}

impl OpcodePart for u8 {
    fn bits(&self) -> u8 {
        *self
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InterruptMode {
    Zero,
    One,
    Two,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Repeat {
    Repeat,
    NoRepeat,
}

impl Repeat {
    fn all() -> &'static [Self] {
        &[Repeat::Repeat, Repeat::NoRepeat]
    }
}

impl OpcodePart for Repeat {
    fn bits(&self) -> u8 {
        match self {
            Repeat::Repeat => 1 << 4,
            Repeat::NoRepeat => 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Direction {
    Inc,
    Dec,
}

impl Direction {
    fn all() -> &'static [Self] {
        &[Direction::Inc, Direction::Dec]
    }
}

impl OpcodePart for Direction {
    fn bits(&self) -> u8 {
        match self {
            Direction::Dec => 1 << 3,
            Direction::Inc => 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum OpLoc8<const BIT: u8> {
    RegB,
    RegC,
    RegD,
    RegE,
    RegH,
    RegL,
    Indirect,
    RegA,
}

impl<const BIT: u8> OpLoc8<BIT> {
    fn all() -> &'static [Self] {
        use OpLoc8::*;

        &[RegB, RegC, RegD, RegE, RegH, RegL, Indirect, RegA]
    }

    fn no_indirect() -> &'static [Self] {
        use OpLoc8::*;

        &[RegB, RegC, RegD, RegE, RegH, RegL, RegA]
    }
}

impl<const BIT: u8> OpcodePart for OpLoc8<BIT> {
    fn bits(&self) -> u8 {
        let v = match self {
            OpLoc8::RegB => 0,
            OpLoc8::RegC => 1,
            OpLoc8::RegD => 2,
            OpLoc8::RegE => 3,
            OpLoc8::RegH => 4,
            OpLoc8::RegL => 5,
            OpLoc8::Indirect => 6,
            OpLoc8::RegA => 7,
        };

        v << BIT
    }
}

#[derive(Debug, Copy, Clone)]
enum OpLoc16<const BIT: u8> {
    RegBC,
    RegDE,
    RegHL,
    RegSP,
    RegAF,
}

impl<const BIT: u8> OpLoc16<BIT> {
    fn all_af() -> &'static [Self] {
        use OpLoc16::*;

        &[RegBC, RegDE, RegHL, RegAF]
    }

    fn all_sp() -> &'static [Self] {
        use OpLoc16::*;

        &[RegBC, RegDE, RegHL, RegSP]
    }
}

impl<const BIT: u8> OpcodePart for OpLoc16<BIT> {
    fn bits(&self) -> u8 {
        let v = match self {
            OpLoc16::RegBC => 0,
            OpLoc16::RegDE => 1,
            OpLoc16::RegHL => 2,
            OpLoc16::RegSP => 3,
            OpLoc16::RegAF => 3,
        };

        v << BIT
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Cond {
    NoZero,
    Zero,
    NoCarry,
    Carry,
    ParityOdd,
    ParityEven,
    Positive,
    Negative,
}

impl Cond {
    fn all() -> &'static [Self] {
        use Cond::*;

        &[
            NoZero, Zero, NoCarry, Carry, ParityOdd, ParityEven, Positive, Negative,
        ]
    }

    pub fn check(&self, regs: &Registers) -> bool {
        match self {
            Cond::NoZero => !regs.flag_z(),
            Cond::Zero => regs.flag_z(),
            Cond::NoCarry => !regs.flag_c(),
            Cond::Carry => regs.flag_c(),
            Cond::ParityOdd => !regs.flag_v(),
            Cond::ParityEven => regs.flag_v(),
            Cond::Positive => !regs.flag_s(),
            Cond::Negative => regs.flag_s(),
        }
    }
}

impl OpcodePart for Cond {
    fn bits(&self) -> u8 {
        let value = match self {
            Cond::NoZero => 0,
            Cond::Zero => 1,
            Cond::NoCarry => 2,
            Cond::Carry => 3,
            Cond::ParityOdd => 4,
            Cond::ParityEven => 5,
            Cond::Positive => 6,
            Cond::Negative => 7,
        };

        value << 3
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Alu {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
}

impl Alu {
    pub fn op(&self, left: u8, right: u8, regs: &mut Registers) -> u8 {
        let mut result;
        let carry = if regs.flag_c() { 1 } else { 0 };
        match self {
            Alu::Add => {
                let (r, overflow) = left.overflowing_add(right);
                result = r;
                regs.flag_s_mut().value(result & 0x80 != 0);
                regs.flag_z_mut().value(result == 0);
                regs.flag_h_mut().value((left ^ right ^ result) & 0x10 != 0);
                regs.flag_v_mut()
                    .value((!(left ^ right) & (result ^ right) & 0x80) != 0);
                regs.flag_n_mut().reset();
                regs.flag_c_mut().value(overflow);
            }
            Alu::Adc => {
                let (r, overflow_0) = left.overflowing_add(right);
                let (r, overflow_1) = r.overflowing_add(carry);
                result = r;
                regs.flag_s_mut().value(result & 0x80 != 0);
                regs.flag_z_mut().value(result == 0);
                regs.flag_h_mut().value((left ^ right ^ result) & 0x10 != 0);
                regs.flag_v_mut()
                    .value((!(left ^ right) & (result ^ right) & 0x80) != 0);
                regs.flag_n_mut().reset();
                regs.flag_c_mut().value(overflow_0 || overflow_1);
            }
            Alu::Sub => {
                let (r, overflow) = left.overflowing_sub(right);
                result = r;
                regs.flag_s_mut().value(result & 0x80 != 0);
                regs.flag_z_mut().value(result == 0);
                regs.flag_h_mut().value((left ^ right ^ result) & 0x10 != 0);
                regs.flag_v_mut()
                    .value(((left ^ right) & (result ^ left) & 0x80) != 0);
                regs.flag_n_mut().set();
                regs.flag_c_mut().value(overflow);
            }
            Alu::Sbc => {
                let (r, overflow_0) = left.overflowing_sub(right);
                let (r, overflow_1) = r.overflowing_sub(carry);
                result = r;
                regs.flag_s_mut().value(result & 0x80 != 0);
                regs.flag_z_mut().value(result == 0);
                regs.flag_h_mut().value((left ^ right ^ result) & 0x10 != 0);
                regs.flag_v_mut()
                    .value(((left ^ right) & (result ^ left) & 0x80) != 0);
                regs.flag_n_mut().set();
                regs.flag_c_mut().value(overflow_0 || overflow_1);
            }
            Alu::And => {
                result = left & right;
                regs.set_flags_szv(result);
                regs.flag_h_mut().reset();
                regs.flag_n_mut().reset();
                regs.flag_c_mut().reset();
            }
            Alu::Xor => {
                result = left ^ right;
                regs.set_flags_szv(result);
                regs.flag_h_mut().reset();
                regs.flag_n_mut().reset();
                regs.flag_c_mut().reset();
            }
            Alu::Or => {
                result = left | right;
                regs.set_flags_szv(result);
                regs.flag_h_mut().reset();
                regs.flag_n_mut().reset();
                regs.flag_c_mut().reset();
            }
            Alu::Cp => {
                let (r, overflow) = left.overflowing_sub(right);
                result = r;
                regs.flag_s_mut().value(result & 0x80 != 0);
                regs.flag_z_mut().value(result == 0);
                regs.flag_h_mut().value((left ^ right ^ result) & 0x10 != 0);
                regs.flag_v_mut()
                    .value(((left ^ right) & (result ^ left) & 0x80) != 0);
                regs.flag_n_mut().set();
                regs.flag_c_mut().value(overflow);

                // revert result as CP does not store value
                result = left;
            }
        }

        result
    }

    fn all() -> &'static [Self] {
        use Alu::*;

        &[Add, Adc, Sub, Sbc, And, Xor, Or, Cp]
    }
}

impl OpcodePart for Alu {
    fn bits(&self) -> u8 {
        let value = match self {
            Alu::Add => 0,
            Alu::Adc => 1,
            Alu::Sub => 2,
            Alu::Sbc => 3,
            Alu::And => 4,
            Alu::Xor => 5,
            Alu::Or => 6,
            Alu::Cp => 7,
        };

        value << 3
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BitAlu {
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Sll,
    Srl,
}

impl BitAlu {
    pub fn op(&self, mut value: u8, registers: &mut Registers) -> u8 {
        let hi_bit = value & 0x80 != 0;
        let lo_bit = value & 0x01 != 0;
        let carry = registers.flag_c();
        match self {
            BitAlu::Rlc => {
                value <<= 1;
                registers.flag_c_mut().value(hi_bit);
                if hi_bit {
                    value |= 1;
                }
            }
            BitAlu::Rrc => {
                value >>= 1;
                registers.flag_c_mut().value(lo_bit);
                if lo_bit {
                    value |= 0x80;
                }
            }
            BitAlu::Rl => {
                value <<= 1;
                registers.flag_c_mut().value(hi_bit);
                if carry {
                    value |= 1;
                }
            }
            BitAlu::Rr => {
                value >>= 1;
                registers.flag_c_mut().value(lo_bit);
                if carry {
                    value |= 0x80;
                }
            }
            BitAlu::Sla => {
                value <<= 1;
                registers.flag_c_mut().value(hi_bit);
            }
            BitAlu::Sra => {
                value >>= 1;
                registers.flag_c_mut().value(lo_bit);
                if hi_bit {
                    value |= 0x80;
                }
            }
            BitAlu::Sll => {
                value <<= 1;
                registers.flag_c_mut().value(hi_bit);
                value |= 1;
            }
            BitAlu::Srl => {
                value >>= 1;
                registers.flag_c_mut().value(lo_bit);
            }
        }

        registers.set_flags_szv(value);
        registers.flag_h_mut().reset();
        registers.flag_n_mut().reset();

        value
    }

    fn all() -> &'static [Self] {
        use BitAlu::*;
        &[Rlc, Rrc, Rl, Rr, Sla, Sll, Sra, Srl]
    }
}

impl OpcodePart for BitAlu {
    fn bits(&self) -> u8 {
        let v = match self {
            BitAlu::Rlc => 0,
            BitAlu::Rrc => 1,
            BitAlu::Rl => 2,
            BitAlu::Rr => 3,
            BitAlu::Sla => 4,
            BitAlu::Sra => 5,
            BitAlu::Sll => 6,
            BitAlu::Srl => 7,
        };

        v << 3
    }
}

trait OpcodeVariation: Clone {
    type Item: Copy;
    fn variations(&self) -> impl Iterator<Item = (u8, Self::Item)>;
}

impl OpcodeVariation for u8 {
    type Item = Self;

    fn variations(&self) -> impl Iterator<Item = (u8, Self::Item)> {
        std::iter::once((*self, *self))
    }
}

trait OpcodeIterator {
    type Item;
    fn iter(&self) -> impl Iterator<Item = (u8, Self::Item)>;
}

impl<T0: OpcodeVariation, T1: OpcodeVariation> OpcodeIterator for (T0, T1) {
    type Item = (T0::Item, T1::Item);

    fn iter(&self) -> impl Iterator<Item = (u8, Self::Item)> {
        let v0 = self.0.variations();

        let v1 =
            v0.flat_map(|(p0, v0)| self.1.variations().map(move |(p1, v1)| (p0 | p1, (v0, v1))));

        v1
    }
}

impl<T0: OpcodeVariation, T1: OpcodeVariation, T2: OpcodeVariation> OpcodeIterator
    for (T0, T1, T2)
{
    type Item = (T0::Item, T1::Item, T2::Item);

    fn iter(&self) -> impl Iterator<Item = (u8, Self::Item)> {
        let v0 = self.0.variations();

        let v1 =
            v0.flat_map(|(p0, v0)| self.1.variations().map(move |(p1, v1)| (p0 | p1, (v0, v1))));

        let v2 = v1.flat_map(|(p0, (v0, v1))| {
            self.2
                .variations()
                .map(move |(p1, v2)| (p0 | p1, (v0, v1, v2)))
        });

        v2
    }
}

macro_rules! impl_op_arr {
    ($base:ty$( => $($len:literal) +)?) => {
        impl<'a, const BIT: u8> OpcodeVariation for &'a [$base] {
            type Item = $base;

            fn variations(&self) -> impl Iterator<Item = (u8, Self::Item)> {
                self.iter().copied().map(|v| (v.bits(), v))
            }
        }

        $($(
            impl<const BIT: u8> OpcodeVariation for [$base; $len] {
                type Item = $base;

                fn variations(&self) -> impl Iterator<Item = (u8, Self::Item)> {
                    self.iter().copied().map(|v| (v.bits(), v))
                }
            }
        )+)?
    }
}

impl_op_arr!(OpLoc8<BIT>);
impl_op_arr!(OpLoc16<BIT> => 2);

macro_rules! impl_bit_arr {
    ($base:ty$( => $($len:literal) +)?) => {
        impl<'a> OpcodeVariation for &'a [$base] {
            type Item = $base;

            fn variations(&self) -> impl Iterator<Item = (u8, Self::Item)> {
                self.iter().copied().map(|v| (v.bits(), v))
            }
        }

        $($(
            impl OpcodeVariation for [$base; $len] {
                type Item = $base;

                fn variations(&self) -> impl Iterator<Item = (u8, Self::Item)> {
                    self.iter().copied().map(|v| (v.bits(), v))
                }
            }
        )+)?
    }
}

impl_bit_arr!(Cond => 4);
impl_bit_arr!(Alu);
impl_bit_arr!(BitAlu => 4);
impl_bit_arr!(u8 => 8);
impl_bit_arr!(Repeat);
impl_bit_arr!(Direction);
