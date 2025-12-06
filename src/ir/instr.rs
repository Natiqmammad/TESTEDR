use super::types::IrType;

#[derive(Debug, Clone)]
pub enum IrInstr {
    LoadConstI32 { dst: u32, value: i32 },
    AddI32 { dst: u32, a: u32, b: u32 },
    Ret { value: Option<u32> },
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<IrType>,
    pub ret: IrType,
    pub body: Vec<IrInstr>,
}

impl IrFunction {
    pub fn new(name: impl Into<String>, params: Vec<IrType>, ret: IrType) -> Self {
        Self {
            name: name.into(),
            params,
            ret,
            body: Vec::new(),
        }
    }

    pub fn with_body(mut self, body: Vec<IrInstr>) -> Self {
        self.body = body;
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct IrModule {
    pub funcs: Vec<IrFunction>,
}
