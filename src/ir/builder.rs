use super::{IrFunction, IrInstr, IrModule, IrType};

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
        let func = IrFunction::new(name, Vec::new(), IrType::Void).with_body(vec![IrInstr::Ret {
            value: None,
        }]);
        self.module.funcs.push(func);
        self
    }

    pub fn push_function(&mut self, function: IrFunction) {
        self.module.funcs.push(function);
    }

    pub fn finish(self) -> IrModule {
        self.module
    }
}
