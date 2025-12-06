pub mod types;
pub mod instr;
pub mod builder;

pub use builder::IrBuilder;
pub use instr::{IrFunction, IrInstr, IrModule};
pub use types::IrType;
