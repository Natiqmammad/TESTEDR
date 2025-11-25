#![allow(dead_code)]

use crate::span::Span;

#[derive(Debug, Clone)]
pub struct File {
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Trait(TraitDef),
    Impl(ImplBlock),
    ExternFunction(ExternFunction),
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<AttributeArg>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AttributeArg {
    String { value: String, span: Span },
}

#[derive(Debug, Clone)]
pub struct Function {
    pub attributes: Vec<Attribute>,
    pub signature: FunctionSignature,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct ExternFunction {
    pub attributes: Vec<Attribute>,
    pub abi: String,
    pub signature: FunctionSignature,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub is_async: bool,
    pub returns_async: bool,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub attributes: Vec<Attribute>,
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub attributes: Vec<Attribute>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Vec<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub attributes: Vec<Attribute>,
    pub name: String,
    pub methods: Vec<FunctionSignature>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub attributes: Vec<Attribute>,
    pub trait_type: Option<TypeExpr>,
    pub target: TypeExpr,
    pub methods: Vec<Function>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl(VarDecl),
    Expr(Expr),
    Return {
        value: Option<Expr>,
        span: Span,
    },
    If(IfStmt),
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    For {
        var: String,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    Switch(SwitchStmt),
    Try(TryCatch),
    Block(Block),
    Unsafe {
        body: Block,
        span: Span,
    },
    Assembly(AssemblyBlock),
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub kind: VarKind,
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Let,
    Var,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Block,
    pub else_if: Vec<(Expr, Block)>,
    pub else_branch: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SwitchStmt {
    pub expr: Expr,
    pub arms: Vec<SwitchArm>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SwitchArm {
    pub pattern: Pattern,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard {
        span: Span,
    },
    Binding {
        name: String,
        span: Span,
    },
    Path {
        segments: Vec<String>,
        span: Span,
    },
    Enum {
        path: Vec<String>,
        bindings: Vec<String>,
        span: Span,
    },
    Literal(Literal),
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard { span }
            | Pattern::Binding { span, .. }
            | Pattern::Path { span, .. }
            | Pattern::Enum { span, .. }
            | Pattern::Literal(Literal::Integer { span, .. })
            | Pattern::Literal(Literal::Float { span, .. })
            | Pattern::Literal(Literal::String { span, .. })
            | Pattern::Literal(Literal::Char { span, .. })
            | Pattern::Literal(Literal::Bool { span, .. }) => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TryCatch {
    pub try_block: Block,
    pub catch_binding: Option<String>,
    pub catch_block: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AssemblyBlock {
    pub body: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Identifier {
        name: String,
        span: Span,
    },
    Access {
        base: Box<Expr>,
        member: String,
        op: AccessOperator,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Await {
        expr: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Assignment {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    StructLiteral {
        path: Box<Expr>,
        fields: Vec<StructLiteralField>,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    Block(Block),
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    Lambda(LambdaExpr),
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(lit) => lit.span(),
            Expr::Identifier { span, .. }
            | Expr::Access { span, .. }
            | Expr::Call { span, .. }
            | Expr::Await { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Assignment { span, .. }
            | Expr::StructLiteral { span, .. }
            | Expr::ArrayLiteral { span, .. }
            | Expr::Try { span, .. }
            | Expr::Lambda(LambdaExpr { span, .. })
            | Expr::Block(Block { span, .. })
            | Expr::Index { span, .. }
            | Expr::MethodCall { span, .. } => *span,
        }
    }

    pub fn is_path_like(&self) -> bool {
        matches!(
            self,
            Expr::Identifier { .. }
                | Expr::Access {
                    op: AccessOperator::Path | AccessOperator::Dot,
                    ..
                }
        )
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer { value: String, span: Span },
    Float { value: String, span: Span },
    String { value: String, span: Span },
    Char { value: char, span: Span },
    Bool { value: bool, span: Span },
}

impl Literal {
    pub fn span(&self) -> Span {
        match self {
            Literal::Integer { span, .. }
            | Literal::Float { span, .. }
            | Literal::String { span, .. }
            | Literal::Char { span, .. }
            | Literal::Bool { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructLiteralField {
    pub name: String,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LambdaExpr {
    pub is_async: bool,
    pub params: Vec<LambdaParam>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LambdaParam {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessOperator {
    Dot,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
    Borrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    LogicalOr,
    LogicalAnd,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named(NamedType),
    Array {
        element: Box<TypeExpr>,
        size: usize,
        span: Span,
    },
    Slice {
        element: Box<TypeExpr>,
        span: Span,
    },
    Tuple {
        elements: Vec<TypeExpr>,
        span: Span,
    },
    Reference {
        mutable: bool,
        inner: Box<TypeExpr>,
        span: Span,
    },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named(ty) => ty.span,
            TypeExpr::Array { span, .. } => *span,
            TypeExpr::Slice { span, .. } => *span,
            TypeExpr::Tuple { span, .. } => *span,
            TypeExpr::Reference { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NamedType {
    pub segments: Vec<TypeSegment>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeSegment {
    pub name: String,
    pub generics: Vec<TypeExpr>,
    pub span: Span,
}
