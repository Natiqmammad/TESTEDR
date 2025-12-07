pub mod builder;
pub mod convert;
pub mod instr;
pub mod types;

pub use builder::IrBuilder;
pub use convert::{build_ir, format_ir};
pub use instr::{
    CmpOp, GepIndex, GlobalInit, IrBlock, IrFunction, IrGlobal, IrInstr, IrIntrinsic, IrModule,
    IrTerm,
};
pub use types::IrType;
