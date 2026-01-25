use std::future::poll_fn;
use std::task::Poll;

use save_states::SaveState;
use serde::{Deserialize, Serialize};

mod instructions;
mod registers;

use super::{CpuTickState, TickRequest};
use instructions::{
    Alu, Inst, InstCB, InstED, Instructions, InterruptMode, LoadLoc8, LoadLoc16, PreInst, Repeat,
    StoreLoc8, StoreLoc16,
};
use registers::{IndexMode, Reg8, Reg16, Registers};

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub struct CpuPinInputs {
    pub data: u8,
    pub nmi: bool,
    pub int_req: bool,
    pub reset: bool,
}

#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
pub enum CpuPinOutputs {
    Read(u16),
    Write(u16, u8),
    IoRead(u16),
    IoWrite(u16, u8),
    InterruptAck,
    Break,
    #[default]
    Idle,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum InstructionPrefix {
    None,
    CB,
    ED,
}

#[derive(SaveState)]
pub struct Cpu {
    #[save(skip)]
    insts: Instructions,
    regs: Registers,
    inputs: CpuPinInputs,
    prefix: InstructionPrefix,
    interrupt_mode: InterruptMode,
    pending_reset: bool,
    pending_nmi: bool,
    pending_int: bool,
    inhibit_interrupts: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            insts: Instructions::new(),
            regs: Registers::new(),
            inputs: Default::default(),
            prefix: InstructionPrefix::None,
            interrupt_mode: InterruptMode::Zero,
            pending_reset: true,
            pending_nmi: false,
            pending_int: false,
            inhibit_interrupts: false,
        }
    }

    pub async fn run(mut self) {
        loop {
            self.yield_requests().await;
            let opcode = self.fetch().await;

            self.exec(opcode).await;

            if !self.inhibit_interrupts {
                self.handle_interrupts().await;
            }
        }
    }

    async fn exec(&mut self, opcode: u8) {
        let p_inst = self.insts.lookup(opcode, self.prefix);

        match p_inst {
            PreInst::None(inst) => self.exec_none(inst).await,
            PreInst::ED(inst) => self.exec_ed(inst).await,
            PreInst::CB(inst) => self.exec_cb(inst).await,
        }
    }

    async fn exec_none(&mut self, inst: Inst) {
        if self.regs.is_indexing() && inst.uses_index_offset() {
            let offset = self.read_pc().await;
            self.regs.index_offset(offset);
            self.tick_n(5).await;
        }

        self.inhibit_interrupts = false;
        let mut inhibit_mode_reset = false;
        match inst {
            Inst::Nop => (),
            Inst::Add16(src) => {
                let left = self.regs.get(Reg16::HL);
                let right = self.load16(src).await;
                self.tick_n(7).await;
                let (result, carry) = left.overflowing_add(right);
                self.regs.set(Reg16::HL, result);

                self.regs
                    .flag_h_mut()
                    .value((left ^ right ^ result) & 0x1000 != 0);
                self.regs.flag_n_mut().reset();
                self.regs.flag_c_mut().value(carry);
            }
            Inst::Alu(alu, src) => {
                let left = self.regs.get(Reg8::A);
                let right = self.load8(src).await;
                let result = alu.op(left, right, &mut self.regs);
                self.regs.set(Reg8::A, result);
            }
            Inst::BitAlu(alu, dst, src) => {
                let value = self.load8(src).await;
                let result = alu.op(value, &mut self.regs);
                self.store8(dst, result).await
            }
            Inst::Call => {
                let addr = self.load16(LoadLoc16::Operand).await;
                self.tick().await;
                self.push16(self.regs.get(Reg16::PC)).await;
                self.regs.set(Reg16::PC, addr);
            }
            Inst::CallCond(cond) => {
                let addr = self.load16(LoadLoc16::Operand).await;
                if cond.check(&self.regs) {
                    self.tick().await;
                    self.push16(self.regs.get(Reg16::PC)).await;
                    self.regs.set(Reg16::PC, addr);
                }
            }
            Inst::Ccf => {
                let c = self.regs.flag_c();
                self.regs.flag_c_mut().toggle();
                self.regs.flag_h_mut().value(c);
                self.regs.flag_n_mut().reset();
            }
            Inst::Cpl => {
                let value = self.regs.get(Reg8::A);
                self.regs.set(Reg8::A, value ^ 0xff);
                self.regs.flag_h_mut().set();
                self.regs.flag_n_mut().set();
            }
            Inst::Daa => {
                let mut adj = if self.regs.flag_c() { 0x60 } else { 0x00 };
                if self.regs.flag_h() {
                    adj |= 0x06;
                }
                let a = self.regs.get(Reg8::A);
                let a = if !self.regs.flag_n() {
                    if a & 0x0f > 0x09 {
                        adj |= 0x06;
                    }
                    if a > 0x99 {
                        adj |= 0x60
                    }

                    a.wrapping_add(adj)
                } else {
                    a.wrapping_sub(adj)
                };

                self.regs.set(Reg8::A, a);
                self.regs.set_flags_szv(a);
                self.regs.flag_h_mut().reset();
                self.regs.flag_c_mut().value(adj & 0x60 != 0);
            }
            Inst::Dec16(dst, src) => {
                let old = self.load16(src).await;
                let new = old.wrapping_sub(1);
                self.tick_n(2).await;
                self.store16(dst, new).await;
            }
            Inst::Dec8(dst, src) => {
                let old = self.load8(src).await;
                let new = old.wrapping_sub(1);
                self.store8(dst, new).await;
                self.regs.flag_s_mut().value(new & 0x80 != 0);
                self.regs.flag_z_mut().value(new == 0);
                self.regs.flag_h_mut().value(old & 0x0f == 0x0);
                self.regs.flag_v_mut().value(old == 0x80);
                self.regs.flag_n_mut().set();
            }
            Inst::Di => {
                self.regs.flag_iff1_mut().reset();
                self.regs.flag_iff2_mut().reset();
                self.pending_int = false;
            }
            Inst::Djnz => {
                self.tick().await;
                let offset = self.load8(LoadLoc8::Operand).await;
                let addr = u16_offset(self.regs.get(Reg16::PC), offset);
                self.regs.dec(Reg8::B);
                if self.regs.get(Reg8::B) != 0 {
                    self.tick_n(5).await;
                    self.regs.set(Reg16::PC, addr);
                }
            }
            Inst::Ei => {
                self.regs.flag_iff1_mut().set();
                self.regs.flag_iff2_mut().set();
            }
            Inst::Ex(r_a, r_b) => {
                let a = self.regs.get(r_a);
                let b = self.regs.get(r_b);
                self.regs.set(r_a, b);
                self.regs.set(r_b, a);
            }
            Inst::ExSP => {
                let addr = self.regs.get(Reg16::SP);
                self.tick().await;
                let a = self.read16(addr).await;
                let b = self.regs.get(Reg16::HL);
                self.regs.set(Reg16::HL, a);
                self.write16(addr, b).await;
                self.tick_n(2).await;
            }
            Inst::Exx => {
                let swaps = [
                    (Reg16::BC, Reg16::BC_),
                    (Reg16::DE, Reg16::DE_),
                    (Reg16::HL, Reg16::HL_),
                ];

                for (r_a, r_b) in swaps {
                    let a = self.regs.get(r_a);
                    let b = self.regs.get(r_b);
                    self.regs.set(r_a, b);
                    self.regs.set(r_b, a);
                }
            }
            Inst::Halt => self.regs.dec(Reg16::PC),
            Inst::In(src) => {
                let lo = self.load8(src).await;
                let port = u16(lo, self.regs.get(Reg8::A));
                let value = self.io_read(port).await;
                self.regs.set(Reg8::A, value);
            }
            Inst::Inc16(dst, src) => {
                let old = self.load16(src).await;
                let new = old.wrapping_add(1);
                self.tick_n(2).await;
                self.store16(dst, new).await;
            }
            Inst::Inc8(dst, src) => {
                let old = self.load8(src).await;
                let new = old.wrapping_add(1);
                self.store8(dst, new).await;
                self.regs.flag_s_mut().value(new & 0x80 != 0);
                self.regs.flag_z_mut().value(new == 0);
                self.regs.flag_h_mut().value(old & 0x0f == 0x0f);
                self.regs.flag_v_mut().value(old == 0x7f);
                self.regs.flag_n_mut().reset();
            }
            Inst::Jp(src) => {
                let addr = self.load16(src).await;
                self.regs.set(Reg16::PC, addr);
            }
            Inst::JpCond(cond) => {
                let addr = self.load16(LoadLoc16::Operand).await;
                if cond.check(&self.regs) {
                    self.regs.set(Reg16::PC, addr);
                }
            }
            Inst::Jr => {
                let offset = self.load8(LoadLoc8::Operand).await;
                self.tick_n(5).await;
                let addr = u16_offset(self.regs.get(Reg16::PC), offset);
                self.regs.set(Reg16::PC, addr);
            }
            Inst::JrCond(cond) => {
                let offset = self.load8(LoadLoc8::Operand).await;
                if cond.check(&self.regs) {
                    self.tick_n(5).await;
                    let addr = u16_offset(self.regs.get(Reg16::PC), offset);
                    self.regs.set(Reg16::PC, addr);
                }
            }
            Inst::Ld8(dst, src) => {
                let value = self.load8(src).await;
                self.store8(dst, value).await;
            }
            Inst::Ld16(dst, src) => {
                let value = self.load16(src).await;
                self.store16(dst, value).await;
            }
            Inst::Out(src) => {
                let lo = self.load8(src).await;
                let a = self.regs.get(Reg8::A);
                let port = u16(lo, a);
                self.io_write(port, a).await;
            }
            Inst::Pop(dst) => {
                let value = self.pop16().await;
                self.store16(dst, value).await;
            }
            Inst::Push(src) => {
                self.tick().await;
                let value = self.load16(src).await;
                self.push16(value).await;
            }
            Inst::Ret => {
                let value = self.pop16().await;
                self.regs.set(Reg16::PC, value);
            }
            Inst::RetCond(cond) => {
                self.tick().await;
                if cond.check(&self.regs) {
                    let value = self.pop16().await;
                    self.regs.set(Reg16::PC, value);
                }
            }
            Inst::Rst(lo) => {
                self.tick().await;
                self.push16(self.regs.get(Reg16::PC)).await;
                let addr = u16(lo, 0);
                self.regs.set(Reg16::PC, addr);
            }
            Inst::Scf => {
                self.regs.flag_c_mut().set();
                self.regs.flag_h_mut().reset();
                self.regs.flag_n_mut().reset();
            }
            Inst::PrefixED => {
                self.prefix = InstructionPrefix::ED;
                self.inhibit_interrupts = true;
            }
            Inst::PrefixCB => {
                self.prefix = InstructionPrefix::CB;
                inhibit_mode_reset = true;
                self.inhibit_interrupts = true;

                if self.regs.is_indexing() {
                    let offset = self.read_pc().await;
                    self.regs.index_offset(offset);
                    self.tick_n(5).await;
                }
            }
            Inst::PrefixIX => {
                self.regs.index_mode(IndexMode::IX);
                inhibit_mode_reset = true;
                self.inhibit_interrupts = true;
            }
            Inst::PrefixIY => {
                self.regs.index_mode(IndexMode::IY);
                inhibit_mode_reset = true;
                self.inhibit_interrupts = true;
            }
            Inst::Unknown => todo!("unknown inst: PC:{:04x}", self.regs.get(Reg16::PC)),
        }

        if !inhibit_mode_reset {
            self.regs.index_mode(IndexMode::HL);
        }
    }

    async fn exec_ed(&mut self, inst: InstED) {
        self.inhibit_interrupts = false;
        match inst {
            InstED::Adc16(src) => {
                let left = self.regs.get(Reg16::HL);
                let right = self.load16(src).await;
                self.tick_n(7).await;

                let (left_lo, left_hi) = u8(left);
                let (right_lo, right_hi) = u8(right);

                let result_lo = Alu::Adc.op(left_lo, right_lo, &mut self.regs);
                let result_hi = Alu::Adc.op(left_hi, right_hi, &mut self.regs);

                let result = u16(result_lo, result_hi);
                self.regs.set(Reg16::HL, result);
                self.regs.flag_z_mut().value(result == 0);
            }
            InstED::CpBlock(repeat, direction) => {
                let c = self.regs.flag_c();
                let a = self.regs.get(Reg8::A);
                let value = self.load8(LoadLoc8::RegIndirect(Reg16::HL)).await;
                self.tick_n(5).await;

                let _ = Alu::Cp.op(a, value, &mut self.regs);

                match direction {
                    instructions::Direction::Inc => self.regs.inc(Reg16::HL),
                    instructions::Direction::Dec => self.regs.dec(Reg16::HL),
                }
                self.regs.dec(Reg16::BC);
                let bc = self.regs.get(Reg16::BC);
                self.regs.flag_v_mut().value(bc != 0);
                self.regs.flag_c_mut().value(c);

                if bc != 0 && a != value && repeat == Repeat::Repeat {
                    self.tick_n(5).await;
                    let pc = self.regs.get(Reg16::PC);
                    self.regs.set(Reg16::PC, pc.wrapping_sub(2));
                }
            }
            InstED::Im(mode) => {
                self.interrupt_mode = mode;
            }
            InstED::In(dst) => {
                let value = self.io_read(self.regs.get(Reg16::BC)).await;
                self.tick().await;
                self.regs.set_flags_szv(value);
                self.regs.flag_h_mut().reset();
                self.regs.flag_n_mut().reset();
                self.store8(dst, value).await;
            }
            InstED::InBlock(repeat, direction) => {
                self.tick().await;
                let value = self.io_read(self.regs.get(Reg16::BC)).await;
                self.store8(StoreLoc8::RegIndirect(Reg16::HL), value).await;
                self.tick().await;
                match direction {
                    instructions::Direction::Inc => {
                        self.regs.inc(Reg16::HL);
                    }
                    instructions::Direction::Dec => {
                        self.regs.dec(Reg16::HL);
                    }
                }
                self.regs.dec(Reg8::B);
                self.regs.flag_n_mut().set();
                let b = self.regs.get(Reg8::B);
                self.regs.flag_z_mut().value(b != 0);

                if b != 0 && repeat == Repeat::Repeat {
                    self.tick_n(5).await;
                    let pc = self.regs.get(Reg16::PC);
                    self.regs.set(Reg16::PC, pc.wrapping_sub(2));
                }
            }
            InstED::Ld16(dst, src) => {
                let value = self.load16(src).await;
                self.store16(dst, value).await;
            }
            InstED::Ld8(dst, src) => {
                let value = self.load8(src).await;
                self.store8(dst, value).await;
                self.tick_n(2).await;
            }
            InstED::Ld8Flags(dst, src) => {
                let value = self.load8(src).await;
                self.store8(dst, value).await;
                self.tick_n(2).await;
                self.regs.flag_s_mut().value(value & 0x80 != 0);
                self.regs.flag_z_mut().value(value == 0);
                self.regs.flag_h_mut().reset();
                let iff2 = self.regs.flag_iff2();
                self.regs.flag_v_mut().value(iff2);
                self.regs.flag_n_mut().reset();
            }
            InstED::LdBlock(repeat, direction) => {
                let value = self.load8(LoadLoc8::RegIndirect(Reg16::HL)).await;
                self.store8(StoreLoc8::RegIndirect(Reg16::DE), value).await;
                self.tick_n(2).await;
                match direction {
                    instructions::Direction::Inc => {
                        self.regs.inc(Reg16::DE);
                        self.regs.inc(Reg16::HL);
                    }
                    instructions::Direction::Dec => {
                        self.regs.dec(Reg16::DE);
                        self.regs.dec(Reg16::HL);
                    }
                }
                self.regs.dec(Reg16::BC);
                self.regs.flag_h_mut().reset();
                self.regs.flag_n_mut().reset();
                let bc = self.regs.get(Reg16::BC);
                self.regs.flag_v_mut().value(bc != 0);

                if bc != 0 && repeat == Repeat::Repeat {
                    self.tick_n(5).await;
                    let pc = self.regs.get(Reg16::PC);
                    self.regs.set(Reg16::PC, pc.wrapping_sub(2));
                }
            }
            InstED::Neg => {
                let a = self.regs.get(Reg8::A);
                let result = Alu::Sub.op(0, a, &mut self.regs);
                self.regs.set(Reg8::A, result);
            }
            InstED::Out(src) => {
                let value = self.load8(src).await;
                self.io_write(self.regs.get(Reg16::BC), value).await;
                self.tick().await;
            }
            InstED::OutBlock(repeat, direction) => {
                self.tick().await;
                let value = self.load8(LoadLoc8::RegIndirect(Reg16::HL)).await;
                self.io_write(self.regs.get(Reg16::BC), value).await;
                self.tick().await;
                match direction {
                    instructions::Direction::Inc => {
                        self.regs.inc(Reg16::HL);
                    }
                    instructions::Direction::Dec => {
                        self.regs.dec(Reg16::HL);
                    }
                }
                self.regs.dec(Reg8::B);
                self.regs.flag_n_mut().set();
                let b = self.regs.get(Reg8::B);
                self.regs.flag_z_mut().value(b != 0);

                if b != 0 && repeat == Repeat::Repeat {
                    self.tick_n(5).await;
                    let pc = self.regs.get(Reg16::PC);
                    self.regs.set(Reg16::PC, pc.wrapping_sub(2));
                }
            }
            InstED::Reti => {
                let value = self.pop16().await;
                self.regs.set(Reg16::PC, value);
            }
            InstED::Retn => {
                let value = self.pop16().await;
                self.regs.set(Reg16::PC, value);
                let iff2 = self.regs.flag_iff2();
                self.regs.flag_iff1_mut().value(iff2);
            }
            InstED::Rld => {
                let a = self.regs.get(Reg8::A);
                let mem = self.load8(LoadLoc8::RegIndirect(Reg16::HL)).await;
                self.tick_n(4).await;

                let a_lo = a & 0xf;
                let a_hi = a >> 4;
                let mem_lo = mem & 0xf;
                let mem_hi = mem >> 4;

                let a = (a_hi << 4) | mem_hi;
                let mem = (mem_lo << 4) | a_lo;

                self.store8(StoreLoc8::RegIndirect(Reg16::HL), mem).await;
                self.regs.set(Reg8::A, a);
                self.regs.set_flags_szv(a);
                self.regs.flag_h_mut().reset();
                self.regs.flag_n_mut().reset();
            }
            InstED::Rrd => {
                let a = self.regs.get(Reg8::A);
                let mem = self.load8(LoadLoc8::RegIndirect(Reg16::HL)).await;
                self.tick_n(4).await;

                let a_lo = a & 0xf;
                let a_hi = a >> 4;
                let mem_lo = mem & 0xf;
                let mem_hi = mem >> 4;

                let a = (a_hi << 4) | mem_lo;
                let mem = (a_lo << 4) | mem_hi;

                self.store8(StoreLoc8::RegIndirect(Reg16::HL), mem).await;
                self.regs.set(Reg8::A, a);
                self.regs.set_flags_szv(a);
                self.regs.flag_h_mut().reset();
                self.regs.flag_n_mut().reset();
            }
            InstED::Sbc16(src) => {
                let left = self.regs.get(Reg16::HL);
                let right = self.load16(src).await;
                self.tick_n(7).await;

                let (left_lo, left_hi) = u8(left);
                let (right_lo, right_hi) = u8(right);

                let result_lo = Alu::Sbc.op(left_lo, right_lo, &mut self.regs);
                let result_hi = Alu::Sbc.op(left_hi, right_hi, &mut self.regs);

                let result = u16(result_lo, result_hi);
                self.regs.set(Reg16::HL, result);
                self.regs.flag_z_mut().value(result == 0);
            }
            InstED::Unknown => todo!("unknown ED inst: PC:{:04x}", self.regs.get(Reg16::PC)),
        }

        self.prefix = InstructionPrefix::None;
        self.regs.index_mode(IndexMode::HL);
    }

    async fn exec_cb(&mut self, inst: InstCB) {
        self.inhibit_interrupts = false;
        match inst {
            InstCB::Alu(alu, dst, src) => {
                let value = self.load8(src).await;
                let result = alu.op(value, &mut self.regs);
                self.store8(dst, result).await
            }
            InstCB::Bit(b, src) => {
                let mask = 1 << b;
                let value = self.load8(src).await;
                self.regs.flag_z_mut().value(value & mask == 0);
                self.regs.flag_h_mut().set();
                self.regs.flag_n_mut().reset();
            }
            InstCB::Set(b, dst, src) => {
                let mask = 1 << b;
                let value = self.load8(src).await;
                self.store8(dst, value | mask).await;
            }
            InstCB::Reset(b, dst, src) => {
                let mask = 1 << b;
                let value = self.load8(src).await;
                self.store8(dst, value & !mask).await;
            }
            InstCB::Unknown => todo!("unknown CB inst: PC:{:04x}", self.regs.get(Reg16::PC)),
        }

        self.prefix = InstructionPrefix::None;
        self.regs.index_mode(IndexMode::HL);
    }

    async fn handle_interrupts(&mut self) {
        if self.pending_reset {
            self.pending_reset = false;
            self.pending_nmi = false;
            self.pending_int = false;
            self.interrupt_mode = InterruptMode::Zero;
            self.regs = Registers::new();
        } else if self.pending_nmi {
            let pc = self.regs.get(Reg16::PC);
            self.tick().await;
            self.read(pc).await;
            self.tick().await;

            self.pending_nmi = false;
            self.regs.flag_iff1_mut().reset();
            self.push16(pc).await;
            let addr = u16(0x66, 0);
            self.regs.set(Reg16::PC, addr);
        } else if self.regs.flag_iff1() && self.pending_int {
            self.regs.flag_iff1_mut().reset();
            self.regs.flag_iff2_mut().reset();
            let data = self.interrupt_ack().await;
            match self.interrupt_mode {
                InterruptMode::Zero => {
                    self.exec(data).await;
                }
                InterruptMode::One => {
                    self.tick().await;
                    self.push16(self.regs.get(Reg16::PC)).await;
                    let addr = u16(0x38, 0);
                    self.regs.set(Reg16::PC, addr);
                }
                InterruptMode::Two => {
                    self.tick().await;
                    self.push16(self.regs.get(Reg16::PC)).await;
                    let addr = u16(data & 0xfe, self.regs.get(Reg8::I));
                    let addr = self.read16(addr).await;
                    self.regs.set(Reg16::PC, addr);
                }
            }
        }
    }

    async fn load8(&mut self, loc: LoadLoc8) -> u8 {
        match loc {
            LoadLoc8::Reg(reg) => self.regs.get(reg),
            LoadLoc8::RegIndirect(reg) => self.read(self.regs.get_idx_off(reg)).await,
            LoadLoc8::RegNoIdx(reg) => self.regs.no_idx().get(reg),
            LoadLoc8::Operand => self.read_pc().await,
            LoadLoc8::OperandIndirect => {
                let lo = self.read_pc().await;
                let hi = self.read_pc().await;
                self.read(u16(lo, hi)).await
            }
        }
    }

    async fn store8(&mut self, loc: StoreLoc8, value: u8) {
        match loc {
            StoreLoc8::Reg(reg) => self.regs.set(reg, value),
            StoreLoc8::RegIndirect(reg) => self.write(self.regs.get_idx_off(reg), value).await,
            StoreLoc8::RegNoIdx(reg) => self.regs.no_idx().set(reg, value),
            StoreLoc8::OperandIndirect => {
                let lo = self.read_pc().await;
                let hi = self.read_pc().await;
                let addr = u16(lo, hi);
                self.write(addr, value).await;
            }
        }
    }

    async fn load16(&mut self, loc: LoadLoc16) -> u16 {
        match loc {
            LoadLoc16::Reg(reg) => self.regs.get(reg),
            LoadLoc16::Operand => {
                let lo = self.read_pc().await;
                let hi = self.read_pc().await;
                u16(lo, hi)
            }
            LoadLoc16::OperandIndirect => {
                let lo = self.read_pc().await;
                let hi = self.read_pc().await;
                let addr = u16(lo, hi);
                self.read16(addr).await
            }
        }
    }

    async fn store16(&mut self, loc: StoreLoc16, value: u16) {
        match loc {
            StoreLoc16::Reg(reg) => {
                self.regs.set(reg, value);
            }
            StoreLoc16::OperandIndirect => {
                let lo = self.read_pc().await;
                let hi = self.read_pc().await;
                let addr = u16(lo, hi);
                self.write16(addr, value).await;
            }
        }
    }

    async fn fetch(&mut self) -> u8 {
        let pc = self.regs.get(Reg16::PC);
        self.regs.inc(Reg16::PC);
        self.regs.inc(Reg8::R);
        self.tick().await;
        self.read(pc).await
    }

    async fn read_pc(&mut self) -> u8 {
        let pc = self.regs.get(Reg16::PC);
        self.regs.inc(Reg16::PC);
        self.read(pc).await
    }

    async fn read(&mut self, address: u16) -> u8 {
        self.tick().await;
        self.tick().await;
        self.do_yield(CpuPinOutputs::Read(address)).await;
        self.data_bus()
    }

    async fn read16(&mut self, address: u16) -> u16 {
        let lo = self.read(address).await;
        let hi = self.read(address.wrapping_add(1)).await;
        u16(lo, hi)
    }

    async fn write(&mut self, address: u16, value: u8) {
        self.tick().await;
        self.tick().await;
        self.do_yield(CpuPinOutputs::Write(address, value)).await;
    }

    async fn write16(&mut self, address: u16, value: u16) {
        let (lo, hi) = u8(value);
        self.write(address, lo).await;
        self.write(address.wrapping_add(1), hi).await;
    }

    async fn pop(&mut self) -> u8 {
        let value = self.read(self.regs.get(Reg16::SP)).await;
        self.regs.inc(Reg16::SP);
        value
    }

    async fn pop16(&mut self) -> u16 {
        let lo = self.pop().await;
        let hi = self.pop().await;
        u16(lo, hi)
    }

    async fn push(&mut self, value: u8) {
        self.regs.dec(Reg16::SP);
        self.write(self.regs.get(Reg16::SP), value).await;
    }

    async fn push16(&mut self, value: u16) {
        let (lo, hi) = u8(value);
        self.push(hi).await;
        self.push(lo).await;
    }

    async fn io_read(&mut self, address: u16) -> u8 {
        self.tick().await;
        self.tick().await;
        self.tick().await;
        self.do_yield(CpuPinOutputs::IoRead(address)).await;
        self.data_bus()
    }

    async fn io_write(&mut self, address: u16, value: u8) {
        self.tick().await;
        self.tick().await;
        self.tick().await;
        self.do_yield(CpuPinOutputs::IoWrite(address, value)).await;
    }

    async fn interrupt_ack(&mut self) -> u8 {
        self.tick().await;
        self.tick().await;
        self.tick().await;
        self.tick().await;
        self.tick().await;
        self.do_yield(CpuPinOutputs::InterruptAck).await;
        self.data_bus()
    }

    async fn tick(&mut self) {
        self.do_yield(CpuPinOutputs::Idle).await;
    }

    #[inline(always)]
    async fn tick_n(&mut self, cycles: u32) {
        for _ in 0..cycles {
            self.tick().await;
        }
    }

    fn update_input(&mut self, new_inputs: CpuPinInputs) {
        self.inputs = new_inputs;
        if self.inputs.reset {
            self.pending_reset = true;
        }
        if self.inputs.nmi {
            self.pending_nmi = true;
        }
        if self.inputs.int_req && self.regs.flag_iff1() {
            self.pending_int = true;
        }
    }

    fn data_bus(&self) -> u8 {
        self.inputs.data
    }

    async fn do_yield(&mut self, output: CpuPinOutputs) {
        let mut yielded = false;
        let inputs = poll_fn(|cx| {
            use super::TickState;
            let waker = CpuTickState::from_context(cx);

            if !yielded {
                waker.set_output(output);
                yielded = true;
                Poll::Pending
            } else {
                let input = waker.input();
                Poll::Ready(input)
            }
        })
        .await;
        self.update_input(inputs);
    }

    async fn yield_requests(&mut self) {
        poll_fn(|cx| {
            use super::TickState;
            let waker = CpuTickState::from_context(cx);
            match waker.take_request() {
                Some(TickRequest::Break) => {
                    waker.set_output(CpuPinOutputs::Break);
                    Poll::Pending
                }
                Some(TickRequest::SaveState) => {
                    let save_data = self.save_state();
                    waker.set_save_data(save_data);
                    waker.set_output(CpuPinOutputs::Break);
                    Poll::Pending
                }
                Some(TickRequest::RestoreState(restore_data)) => {
                    self.restore_state(&restore_data);
                    Poll::Ready(())
                }

                None => Poll::Ready(()),
            }
        })
        .await
    }
}

#[inline(always)]
fn u16(lo: u8, hi: u8) -> u16 {
    lo as u16 | ((hi as u16) << 8)
}

#[inline(always)]
fn u8(val: u16) -> (u8, u8) {
    ((val & 0xff) as u8, (val >> 8) as u8)
}

#[inline(always)]
fn u16_offset(addr: u16, offset: u8) -> u16 {
    let offset = offset as i8 as i16;
    addr.wrapping_add_signed(offset)
}
