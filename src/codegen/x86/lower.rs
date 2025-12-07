use anyhow::Result;

use crate::ir::{IrInstr, IrModule, IrTerm};

#[derive(Debug, Clone)]
pub struct LoweredModule {
    pub blocks: Vec<LoweredBlock>,
    pub strings: Vec<String>,
    pub value_count: usize,
    pub uses: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct LoweredBlock {
    pub id: u32,
    pub instrs: Vec<IrInstr>,
    pub term: IrTerm,
}

pub fn lower_ir(module: &IrModule) -> Result<LoweredModule> {
    if let Some(func) = module.funcs.iter().find(|f| f.name == "apex") {
        let mut blocks = Vec::new();
        let mut max_value = 0u32;
        let mut uses: Vec<u32> = Vec::new();
        for block in &func.blocks {
            for instr in &block.body {
                if let Some(value) = instr.result() {
                    max_value = max_value.max(value + 1);
                }
                match instr {
                    IrInstr::AddI32 { a, b, .. }
                    | IrInstr::Add { a, b, .. }
                    | IrInstr::Sub { a, b, .. }
                    | IrInstr::CmpEq { a, b, .. }
                    | IrInstr::Cmp { a, b, .. }
                    | IrInstr::And { a, b, .. }
                    | IrInstr::Or { a, b, .. } => {
                        max_value = max_value.max(*a + 1);
                        max_value = max_value.max(*b + 1);
                        increment_use(&mut uses, *a);
                        increment_use(&mut uses, *b);
                    }
                    IrInstr::Phi { incomings, .. } => {
                        for (_, val) in incomings {
                            max_value = max_value.max(*val + 1);
                            increment_use(&mut uses, *val);
                        }
                    }
                    IrInstr::Load { ptr, .. } => increment_use(&mut uses, *ptr),
                    IrInstr::Store { src, ptr, .. } => {
                        increment_use(&mut uses, *src);
                        increment_use(&mut uses, *ptr);
                    }
                    _ => {}
                }
            }
            if let Some(term) = &block.term {
                if let IrTerm::CondBr { cond, .. } = term {
                    increment_use(&mut uses, *cond);
                }
            }
            let term = block.term.clone().unwrap_or(IrTerm::Ret { value: None });
            blocks.push(LoweredBlock {
                id: block.id,
                instrs: block.body.clone(),
                term,
            });
        }
        return Ok(LoweredModule {
            blocks,
            strings: module.strings.clone(),
            value_count: max_value as usize,
            uses,
        });
    }

    Ok(LoweredModule {
        blocks: Vec::new(),
        strings: module.strings.clone(),
        value_count: 0,
        uses: Vec::new(),
    })
}

fn increment_use(uses: &mut Vec<u32>, value: u32) {
    let idx = value as usize;
    if idx >= uses.len() {
        uses.resize(idx + 1, 0);
    }
    uses[idx] += 1;
}
