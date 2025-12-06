#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrType {
    I32,
    I64,
    Bool,
    Void,
    Ptr(Box<IrType>),
    // Future extensions: Struct, Enum, Generic, etc.
}
