use super::{IrFunction, IrInstr, IrModule, IrTerm, IrType};

type FunctionId = usize;

pub struct IrBuilder {
    module: IrModule,
}

impl IrBuilder {
    pub fn new() -> Self {
        Self {
            module: IrModule::default(),
        }
    }

    pub fn with_entry_function(mut self, name: impl Into<String>) -> Self {
        let func_id = self.new_function(name, Vec::new(), IrType::Void);
        let block_id = self.new_block(func_id);
        self.set_term(func_id, block_id, IrTerm::Ret { value: None });
        self
    }

    pub fn new_function(
        &mut self,
        name: impl Into<String>,
        params: Vec<IrType>,
        ret: IrType,
    ) -> FunctionId {
        let func = IrFunction::new(name, params, ret);
        self.module.funcs.push(func);
        self.module.funcs.len() - 1
    }

    pub fn new_block(&mut self, func_id: FunctionId) -> u32 {
        self.module
            .funcs
            .get_mut(func_id)
            .expect("invalid function id")
            .new_block()
    }

    pub fn emit(&mut self, func_id: FunctionId, block_id: u32, instr: IrInstr) -> Option<u32> {
        let result = instr.result();
        let func = self
            .module
            .funcs
            .get_mut(func_id)
            .expect("invalid function id");
        func.block_mut(block_id).body.push(instr);
        result
    }

    pub fn next_value(&mut self, func_id: FunctionId) -> u32 {
        self.module
            .funcs
            .get_mut(func_id)
            .expect("invalid function id")
            .allocate_value()
    }

    pub fn set_term(&mut self, func_id: FunctionId, block_id: u32, term: IrTerm) {
        self.module
            .funcs
            .get_mut(func_id)
            .expect("invalid function id")
            .block_mut(block_id)
            .term = Some(term);
    }

    pub fn block_term(&self, func_id: FunctionId, block_id: u32) -> Option<IrTerm> {
        self.module
            .funcs
            .get(func_id)
            .and_then(|f| f.blocks.iter().find(|b| b.id == block_id))
            .and_then(|b| b.term.clone())
    }

    pub fn block_has_term(&self, func_id: FunctionId, block_id: u32) -> bool {
        self.module
            .funcs
            .get(func_id)
            .and_then(|f| f.blocks.iter().find(|b| b.id == block_id))
            .and_then(|b| b.term.as_ref())
            .is_some()
    }

    pub fn block_instrs_mut(&mut self, func_id: FunctionId, block_id: u32) -> &mut Vec<IrInstr> {
        &mut self
            .module
            .funcs
            .get_mut(func_id)
            .expect("invalid function id")
            .block_mut(block_id)
            .body
    }

    pub fn finish(mut self) -> IrModule {
        for func in &mut self.module.funcs {
            for block in &mut func.blocks {
                if block.term.is_none() {
                    block.term = Some(IrTerm::Ret { value: None });
                }
            }
        }
        self.module
    }

    pub fn intern_string(&mut self, text: impl Into<String>) -> u32 {
        let text = text.into();
        if let Some(idx) = self.module.strings.iter().position(|s| s == &text) {
            idx as u32
        } else {
            let id = self.module.strings.len() as u32;
            self.module.strings.push(text);
            id
        }
    }
}
