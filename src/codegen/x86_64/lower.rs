use anyhow::Result;

use crate::ir::IrModule;

#[derive(Debug, Clone)]
pub struct LoweredModule {
    pub entry: String,
}

pub fn lower_ir(module: &IrModule) -> Result<LoweredModule> {
    let entry = module
        .funcs
        .first()
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "apex".to_string());
    Ok(LoweredModule { entry })
}
