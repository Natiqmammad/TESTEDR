use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::ir::{IrInstr, IrTerm};

use super::lower::{LoweredBlock, LoweredModule};

pub struct MachineCode {
    pub code: Vec<u8>,
    pub patches: Vec<Patch>,
    pub strings: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct Patch {
    pub offset: usize,
    pub string_id: u32,
}

#[derive(Debug, Clone)]
struct JumpPatch {
    offset: usize,
    label: String,
    kind: JumpKind,
}

#[derive(Debug, Clone, Copy)]
enum JumpKind {
    Jmp,
    Je,
}

#[derive(Debug, Clone, Copy)]
enum CmpKind {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

pub fn emit_x86_64(lowered: &LoweredModule) -> Result<MachineCode> {
    let phi_map = build_phi_map(&lowered.blocks);
    let mut ctx = CodegenCtx::new(lowered.value_count, lowered.uses.clone(), phi_map);
    if lowered.blocks.is_empty() {
        ctx.emit_exit();
    } else {
        for block in &lowered.blocks {
            ctx.emit_label(&format!("block{}", block.id));
            ctx.emit_block_instrs(block, &lowered.strings)?;
            ctx.emit_terminator(block.id, &block.term)?;
        }
    }
    ctx.patch_jumps()?;

    let strings = lowered
        .strings
        .iter()
        .map(|s| s.clone().into_bytes())
        .collect();

    Ok(MachineCode {
        code: ctx.code,
        patches: ctx.string_patches,
        strings,
    })
}

fn build_phi_map(blocks: &[LoweredBlock]) -> HashMap<(u32, u32), Vec<(u32, u32)>> {
    let mut map: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
    for block in blocks {
        for instr in &block.instrs {
            if let IrInstr::Phi { dst, incomings } = instr {
                for (pred, value) in incomings {
                    map.entry((*pred, block.id))
                        .or_default()
                        .push((*dst, *value));
                }
            }
        }
    }
    map
}

struct CodegenCtx {
    code: Vec<u8>,
    value_regs: Vec<Option<Reg>>,
    free_regs: Vec<Reg>,
    uses: Vec<u32>,
    phi_map: HashMap<(u32, u32), Vec<(u32, u32)>>,
    string_patches: Vec<Patch>,
    labels: HashMap<String, usize>,
    jumps: Vec<JumpPatch>,
}

impl CodegenCtx {
    fn new(
        value_count: usize,
        uses: Vec<u32>,
        phi_map: HashMap<(u32, u32), Vec<(u32, u32)>>,
    ) -> Self {
        Self {
            code: Vec::new(),
            value_regs: vec![None; value_count.max(1)],
            free_regs: vec![
                Reg::RAX,
                Reg::RBX,
                Reg::RCX,
                Reg::RDX,
                Reg::RSI,
                Reg::RDI,
                Reg::R8,
                Reg::R9,
                Reg::R10,
                Reg::R11,
                Reg::R12,
                Reg::R13,
                Reg::R14,
                Reg::R15,
            ],
            uses,
            phi_map,
            string_patches: Vec::new(),
            labels: HashMap::new(),
            jumps: Vec::new(),
        }
    }

    fn emit_block_instrs(&mut self, block: &LoweredBlock, strings: &[String]) -> Result<()> {
        for instr in &block.instrs {
            match instr {
                IrInstr::LoadConstInt { dst, value, .. } => {
                    let reg = self.ensure_reg(*dst)?;
                    emit_mov_reg_imm(&mut self.code, reg, *value as i64);
                }
                IrInstr::LoadConstStr { dst, sid } => {
                    let reg = self.ensure_reg(*dst)?;
                    emit_mov_reg_placeholder(&mut self.code, reg, *sid, &mut self.string_patches);
                }
                IrInstr::LoadConstBool { dst, value } => {
                    let reg = self.ensure_reg(*dst)?;
                    emit_mov_reg_imm(&mut self.code, reg, if *value { 1 } else { 0 });
                }
                IrInstr::LoadConstI32 { dst, value } => {
                    let reg = self.ensure_reg(*dst)?;
                    emit_mov_reg_imm(&mut self.code, reg, *value as i64);
                }
                IrInstr::AddI32 { dst, a, b } | IrInstr::Add { dst, a, b, .. } => {
                    let dst_reg = self.ensure_reg(*dst)?;
                    let lhs = self.ensure_reg(*a)?;
                    let rhs = self.ensure_reg(*b)?;
                    if dst_reg != lhs {
                        emit_mov_reg_reg(&mut self.code, dst_reg, lhs);
                    }
                    emit_add_reg_reg(&mut self.code, dst_reg, rhs);
                    self.consume(*a);
                    self.consume(*b);
                }
                IrInstr::Sub { dst, a, b, .. } => {
                    let dst_reg = self.ensure_reg(*dst)?;
                    let lhs = self.ensure_reg(*a)?;
                    let rhs = self.ensure_reg(*b)?;
                    if dst_reg != lhs {
                        emit_mov_reg_reg(&mut self.code, dst_reg, lhs);
                    }
                    emit_sub_reg_reg(&mut self.code, dst_reg, rhs);
                    self.consume(*a);
                    self.consume(*b);
                }
                IrInstr::CmpEq { dst, a, b } => {
                    self.emit_cmp_set(*dst, *a, *b, CmpKind::Eq)?;
                }
                IrInstr::Cmp {
                    dst, a, b, cond, ..
                } => {
                    let kind = match cond {
                        crate::ir::CmpOp::Eq => CmpKind::Eq,
                        crate::ir::CmpOp::Ne => CmpKind::Ne,
                        crate::ir::CmpOp::Lt => CmpKind::Lt,
                        crate::ir::CmpOp::Le => CmpKind::Le,
                        crate::ir::CmpOp::Gt => CmpKind::Gt,
                        crate::ir::CmpOp::Ge => CmpKind::Ge,
                        _ => {
                            return Err(anyhow!("unsupported cmp condition {:?}", cond));
                        }
                    };
                    self.emit_cmp_set(*dst, *a, *b, kind)?;
                }
                IrInstr::And { dst, a, b, .. } => {
                    let dst_reg = self.ensure_reg(*dst)?;
                    let lhs = self.ensure_reg(*a)?;
                    let rhs = self.ensure_reg(*b)?;
                    if dst_reg != lhs {
                        emit_mov_reg_reg(&mut self.code, dst_reg, lhs);
                    }
                    emit_and_reg_reg(&mut self.code, dst_reg, rhs);
                    self.consume(*a);
                    self.consume(*b);
                }
                IrInstr::Or { dst, a, b, .. } => {
                    let dst_reg = self.ensure_reg(*dst)?;
                    let lhs = self.ensure_reg(*a)?;
                    let rhs = self.ensure_reg(*b)?;
                    if dst_reg != lhs {
                        emit_mov_reg_reg(&mut self.code, dst_reg, lhs);
                    }
                    emit_or_reg_reg(&mut self.code, dst_reg, rhs);
                    self.consume(*a);
                    self.consume(*b);
                }
                IrInstr::PrintStr { sid } => {
                    let len = strings
                        .get(*sid as usize)
                        .map(|s| s.len() as u64)
                        .unwrap_or(0);
                    emit_mov_rax(&mut self.code, 1);
                    emit_mov_rdi(&mut self.code, 1);
                    emit_mov_rsi_placeholder(&mut self.code, *sid, &mut self.string_patches);
                    emit_mov_rdx(&mut self.code, len);
                    emit_syscall(&mut self.code);
                }
                IrInstr::Phi { dst, .. } => {
                    self.ensure_reg(*dst)?;
                }
                _ => {
                    return Err(anyhow!("unsupported ir instr in backend: {:?}", instr));
                }
            }
        }
        Ok(())
    }

    fn emit_terminator(&mut self, block_id: u32, term: &IrTerm) -> Result<()> {
        match term {
            IrTerm::Ret { .. } => {
                self.emit_exit();
            }
            IrTerm::Br { target } => {
                self.emit_phi_moves(block_id, *target)?;
                self.emit_jmp(&format!("block{}", target));
            }
            IrTerm::CondBr {
                cond,
                then_b,
                else_b,
            } => {
                let cond_reg = self.ensure_reg(*cond)?;
                emit_test_reg(&mut self.code, cond_reg);
                let else_label = format!("{}_else_from_{}", else_b, block_id);
                self.emit_conditional_jump(JumpKind::Je, &else_label);
                self.emit_phi_moves(block_id, *then_b)?;
                self.emit_jmp(&format!("block{}", then_b));
                self.emit_label(&else_label);
                self.emit_phi_moves(block_id, *else_b)?;
                self.emit_jmp(&format!("block{}", else_b));
                self.consume(*cond);
            }
            IrTerm::SwitchInt { .. } | IrTerm::Unreachable | IrTerm::Invoke { .. } => {
                return Err(anyhow!("unsupported ir terminator in backend: {:?}", term));
            }
        }
        Ok(())
    }

    fn emit_phi_moves(&mut self, from: u32, to: u32) -> Result<()> {
        if let Some(entries) = self.phi_map.get(&(from, to)).cloned() {
            for (dst, src) in entries {
                let dst_reg = self.ensure_reg(dst)?;
                let src_reg = self.ensure_reg(src)?;
                if dst_reg != src_reg {
                    emit_mov_reg_reg(&mut self.code, dst_reg, src_reg);
                }
                self.consume(src);
            }
        }
        Ok(())
    }

    fn emit_exit(&mut self) {
        emit_mov_rax(&mut self.code, 60);
        emit_xor_rdi(&mut self.code);
        emit_syscall(&mut self.code);
    }

    fn emit_label(&mut self, label: &str) {
        self.labels.insert(label.to_string(), self.code.len());
    }

    fn emit_jmp(&mut self, label: &str) {
        self.code.push(0xE9);
        let offset = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        self.jumps.push(JumpPatch {
            offset,
            label: label.to_string(),
            kind: JumpKind::Jmp,
        });
    }

    fn emit_conditional_jump(&mut self, kind: JumpKind, label: &str) {
        match kind {
            JumpKind::Je => {
                self.code.extend_from_slice(&[0x0F, 0x84]);
            }
            JumpKind::Jmp => unreachable!(),
        }
        let offset = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        self.jumps.push(JumpPatch {
            offset,
            label: label.to_string(),
            kind,
        });
    }

    fn patch_jumps(&mut self) -> Result<()> {
        for patch in &self.jumps {
            let target = self
                .labels
                .get(&patch.label)
                .ok_or_else(|| anyhow!("unknown label {}", patch.label))?
                .to_owned();
            let start = patch.offset;
            let rel = (target as isize - start as isize - 4) as i32;
            self.code[start..start + 4].copy_from_slice(&rel.to_le_bytes());
        }
        Ok(())
    }

    fn ensure_reg(&mut self, value: u32) -> Result<Reg> {
        if value as usize >= self.value_regs.len() {
            self.value_regs.resize(value as usize + 1, None);
        }
        if let Some(reg) = self.value_regs[value as usize] {
            return Ok(reg);
        }
        let reg = self
            .free_regs
            .pop()
            .ok_or_else(|| anyhow!("out of registers for value {}", value))?;
        self.value_regs[value as usize] = Some(reg);
        Ok(reg)
    }

    fn consume(&mut self, value: u32) {
        let idx = value as usize;
        if idx >= self.uses.len() {
            return;
        }
        if self.uses[idx] == 0 {
            return;
        }
        self.uses[idx] -= 1;
        if self.uses[idx] == 0 {
            if let Some(reg) = self.value_regs.get_mut(idx).and_then(|slot| slot.take()) {
                self.free_regs.push(reg);
            }
        }
    }

    fn emit_cmp_set(&mut self, dst: u32, a: u32, b: u32, kind: CmpKind) -> Result<()> {
        let dst_reg = self.ensure_reg(dst)?;
        let lhs = self.ensure_reg(a)?;
        let rhs = self.ensure_reg(b)?;
        emit_cmp_reg_reg(&mut self.code, lhs, rhs);
        match kind {
            CmpKind::Eq => emit_sete_reg(&mut self.code, dst_reg),
            CmpKind::Ne => emit_setne_reg(&mut self.code, dst_reg),
            CmpKind::Lt => emit_setl_reg(&mut self.code, dst_reg),
            CmpKind::Le => emit_setle_reg(&mut self.code, dst_reg),
            CmpKind::Gt => emit_setg_reg(&mut self.code, dst_reg),
            CmpKind::Ge => emit_setge_reg(&mut self.code, dst_reg),
        }
        emit_movzx_reg8(&mut self.code, dst_reg);
        self.consume(a);
        self.consume(b);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reg {
    RAX,
    RBX,
    RCX,
    RDX,
    RSI,
    RDI,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl Reg {
    fn low_bits(self) -> u8 {
        match self {
            Reg::RAX => 0,
            Reg::RBX => 3,
            Reg::RCX => 1,
            Reg::RDX => 2,
            Reg::RSI => 6,
            Reg::RDI => 7,
            Reg::R8 => 0,
            Reg::R9 => 1,
            Reg::R10 => 2,
            Reg::R11 => 3,
            Reg::R12 => 4,
            Reg::R13 => 5,
            Reg::R14 => 6,
            Reg::R15 => 7,
        }
    }

    fn rex_bit(self) -> u8 {
        match self {
            Reg::R8 | Reg::R9 | Reg::R10 | Reg::R11 | Reg::R12 | Reg::R13 | Reg::R14 | Reg::R15 => {
                1
            }
            _ => 0,
        }
    }
}

fn rex_prefix(wide: bool, reg: Reg, rm: Reg) -> u8 {
    let mut rex = 0x40;
    if wide {
        rex |= 0x08;
    }
    if reg.rex_bit() != 0 {
        rex |= 0x04;
    }
    if rm.rex_bit() != 0 {
        rex |= 0x01;
    }
    rex
}

fn modrm_byte(reg: Reg, rm: Reg) -> u8 {
    0xC0 | (reg.low_bits() << 3) | rm.low_bits()
}

fn emit_mov_reg_imm(code: &mut Vec<u8>, reg: Reg, value: i64) {
    let prefix = 0x48 | (reg.rex_bit() & 0x01);
    code.push(prefix);
    code.push(0xB8 + reg.low_bits());
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_reg_reg(code: &mut Vec<u8>, dst: Reg, src: Reg) {
    code.push(rex_prefix(true, src, dst));
    code.push(0x89);
    code.push(modrm_byte(src, dst));
}

fn emit_add_reg_reg(code: &mut Vec<u8>, dst: Reg, src: Reg) {
    code.push(rex_prefix(true, src, dst));
    code.push(0x01);
    code.push(modrm_byte(src, dst));
}

fn emit_sub_reg_reg(code: &mut Vec<u8>, dst: Reg, src: Reg) {
    code.push(rex_prefix(true, src, dst));
    code.push(0x29);
    code.push(modrm_byte(src, dst));
}

fn emit_cmp_reg_reg(code: &mut Vec<u8>, lhs: Reg, rhs: Reg) {
    code.push(rex_prefix(true, rhs, lhs));
    code.push(0x39);
    code.push(modrm_byte(rhs, lhs));
}

fn emit_test_reg(code: &mut Vec<u8>, reg: Reg) {
    code.push(rex_prefix(true, reg, reg));
    code.push(0x85);
    code.push(modrm_byte(reg, reg));
}

fn emit_sete_reg(code: &mut Vec<u8>, reg: Reg) {
    let mut rex = 0x40;
    if reg.rex_bit() != 0 {
        rex |= 0x01;
    }
    code.push(rex);
    code.extend_from_slice(&[0x0F, 0x94]);
    code.push(0xC0 | reg.low_bits());
}

fn emit_setne_reg(code: &mut Vec<u8>, reg: Reg) {
    let mut rex = 0x40;
    if reg.rex_bit() != 0 {
        rex |= 0x01;
    }
    code.push(rex);
    code.extend_from_slice(&[0x0F, 0x95]);
    code.push(0xC0 | reg.low_bits());
}

fn emit_setl_reg(code: &mut Vec<u8>, reg: Reg) {
    let mut rex = 0x40;
    if reg.rex_bit() != 0 {
        rex |= 0x01;
    }
    code.push(rex);
    code.extend_from_slice(&[0x0F, 0x9C]);
    code.push(0xC0 | reg.low_bits());
}

fn emit_setle_reg(code: &mut Vec<u8>, reg: Reg) {
    let mut rex = 0x40;
    if reg.rex_bit() != 0 {
        rex |= 0x01;
    }
    code.push(rex);
    code.extend_from_slice(&[0x0F, 0x9E]);
    code.push(0xC0 | reg.low_bits());
}

fn emit_setg_reg(code: &mut Vec<u8>, reg: Reg) {
    let mut rex = 0x40;
    if reg.rex_bit() != 0 {
        rex |= 0x01;
    }
    code.push(rex);
    code.extend_from_slice(&[0x0F, 0x9F]);
    code.push(0xC0 | reg.low_bits());
}

fn emit_setge_reg(code: &mut Vec<u8>, reg: Reg) {
    let mut rex = 0x40;
    if reg.rex_bit() != 0 {
        rex |= 0x01;
    }
    code.push(rex);
    code.extend_from_slice(&[0x0F, 0x9D]);
    code.push(0xC0 | reg.low_bits());
}

fn emit_movzx_reg8(code: &mut Vec<u8>, reg: Reg) {
    code.push(rex_prefix(true, reg, reg));
    code.extend_from_slice(&[0x0F, 0xB6]);
    code.push(modrm_byte(reg, reg));
}

fn emit_and_reg_reg(code: &mut Vec<u8>, dst: Reg, src: Reg) {
    code.push(rex_prefix(true, src, dst));
    code.push(0x21);
    code.push(modrm_byte(src, dst));
}

fn emit_or_reg_reg(code: &mut Vec<u8>, dst: Reg, src: Reg) {
    code.push(rex_prefix(true, src, dst));
    code.push(0x09);
    code.push(modrm_byte(src, dst));
}

fn emit_mov_rax(code: &mut Vec<u8>, imm: u64) {
    code.extend_from_slice(&[0x48, 0xB8]);
    code.extend_from_slice(&imm.to_le_bytes());
}

fn emit_mov_rdi(code: &mut Vec<u8>, imm: u64) {
    code.extend_from_slice(&[0x48, 0xBF]);
    code.extend_from_slice(&imm.to_le_bytes());
}

fn emit_mov_reg_placeholder(
    code: &mut Vec<u8>,
    reg: Reg,
    string_id: u32,
    patches: &mut Vec<Patch>,
) {
    code.push(rex_prefix(true, reg, reg));
    code.push(0xB8 + reg.low_bits());
    let offset = code.len();
    code.extend_from_slice(&0u64.to_le_bytes());
    patches.push(Patch { offset, string_id });
}

fn emit_mov_rsi_placeholder(code: &mut Vec<u8>, string_id: u32, patches: &mut Vec<Patch>) {
    code.extend_from_slice(&[0x48, 0xBE]);
    let offset = code.len();
    code.extend_from_slice(&0u64.to_le_bytes());
    patches.push(Patch { offset, string_id });
}

fn emit_mov_rdx(code: &mut Vec<u8>, imm: u64) {
    code.extend_from_slice(&[0x48, 0xBA]);
    code.extend_from_slice(&imm.to_le_bytes());
}

fn emit_syscall(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x0F, 0x05]);
}

fn emit_xor_rdi(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x48, 0x31, 0xFF]);
}
