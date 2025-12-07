use std::fmt;

/// Core IR types used across the mid-end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrType {
    I1,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Bool,
    F32,
    F64,
    Void,
    Ptr(Box<IrType>),
    Func {
        params: Vec<IrType>,
        ret: Box<IrType>,
    },
    Array {
        elem: Box<IrType>,
        len: u32,
    },
    Slice {
        elem: Box<IrType>,
    },
    Tuple(Vec<IrType>),
    Struct {
        name: String,
        fields: Vec<IrType>,
    },
    Str, // fat pointer {data,len}
    Opaque(&'static str),
}

impl IrType {
    pub fn ptr(inner: IrType) -> Self {
        IrType::Ptr(Box::new(inner))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Ult,
    Ule,
    Ugt,
    Uge,
    Flt,
    Fle,
    Fgt,
    Fge,
}

#[derive(Debug, Clone)]
pub enum GepIndex {
    Const(u32),
    Value(u32),
}

#[derive(Debug, Clone)]
pub enum IrIntrinsic {
    LogInfo,
    Panic,
    MemAlloc,
    MemFree,
    MemCopy,
    MemSet,
    MemZero,
    StringLen,
    StringConcat,
    VecNew,
    VecLen,
    VecPush,
    VecPop,
    RangeIterNext, // placeholder for for-loop lowering
}

#[derive(Debug, Clone)]
pub enum IrInstr {
    // Constants
    LoadConstInt {
        dst: u32,
        value: i128,
        ty: IrType,
    },
    LoadConstFloat {
        dst: u32,
        value: f64,
        ty: IrType,
    },
    LoadConstBool {
        dst: u32,
        value: bool,
    },
    LoadConstStr {
        dst: u32,
        sid: u32,
    },
    Undef {
        dst: u32,
        ty: IrType,
    },

    // Stack/global memory
    Alloca {
        dst: u32,
        ty: IrType,
    },
    Load {
        dst: u32,
        ptr: u32,
        ty: IrType,
    },
    Store {
        src: u32,
        ptr: u32,
        ty: IrType,
    },
    Gep {
        dst: u32,
        base: u32,
        indices: Vec<GepIndex>,
    },
    PtrCast {
        dst: u32,
        src: u32,
        ty: IrType,
    },

    // Arithmetic (int)
    Add {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    Sub {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    Mul {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    Div {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
        signed: bool,
    },
    Rem {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
        signed: bool,
    },
    Neg {
        dst: u32,
        val: u32,
        ty: IrType,
    },

    // Floating-point
    FAdd {
        dst: u32,
        a: u32,
        b: u32,
    },
    FSub {
        dst: u32,
        a: u32,
        b: u32,
    },
    FMul {
        dst: u32,
        a: u32,
        b: u32,
    },
    FDiv {
        dst: u32,
        a: u32,
        b: u32,
    },
    FRem {
        dst: u32,
        a: u32,
        b: u32,
    },
    FNeg {
        dst: u32,
        val: u32,
    },

    // Bitwise / logical
    And {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    Or {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    Xor {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    Shl {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    LShr {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    AShr {
        dst: u32,
        a: u32,
        b: u32,
        ty: IrType,
    },
    Not {
        dst: u32,
        val: u32,
        ty: IrType,
    },
    Select {
        dst: u32,
        cond: u32,
        then_v: u32,
        else_v: u32,
    },

    // Comparisons
    Cmp {
        dst: u32,
        a: u32,
        b: u32,
        cond: CmpOp,
        ty: IrType,
    },

    // Aggregates
    StructInit {
        dst: u32,
        fields: Vec<u32>,
        ty: IrType,
    },
    StructExtract {
        dst: u32,
        base: u32,
        field_idx: u32,
    },
    StructInsert {
        dst: u32,
        base: u32,
        field_idx: u32,
        value: u32,
    },
    TupleInit {
        dst: u32,
        items: Vec<u32>,
    },
    TupleExtract {
        dst: u32,
        base: u32,
        idx: u32,
    },
    ArrayInit {
        dst: u32,
        elems: Vec<u32>,
        elem_ty: IrType,
    },
    ArrayGet {
        dst: u32,
        base: u32,
        index: u32,
        elem_ty: IrType,
    },
    ArraySet {
        base: u32,
        index: u32,
        value: u32,
        elem_ty: IrType,
    },
    SliceFromArray {
        dst: u32,
        base: u32,
        len: u32,
        elem_ty: IrType,
    },

    // Calls
    Call {
        dst: Option<u32>,
        func: u32,
        args: Vec<u32>,
    },
    CallIntrinsic {
        dst: Option<u32>,
        intrinsic: IrIntrinsic,
        args: Vec<u32>,
    },
    CallExtern {
        dst: Option<u32>,
        symbol: u32,
        args: Vec<u32>,
        sig: IrType,
    },
    MakeClosure {
        dst: u32,
        func: usize,
        env: u32,
    },
    LoadCapture {
        dst: u32,
        env: u32,
        idx: u32,
        ty: IrType,
    },
    Await {
        dst: u32,
        fut: u32,
    },

    // Memory/runtime helpers
    HeapAlloc {
        dst: u32,
        size: u32,
        align: u32,
    },
    HeapFree {
        ptr: u32,
    },
    MemCopy {
        dst: u32,
        src: u32,
        len: u32,
    },
    MemSet {
        dst: u32,
        value: u32,
        len: u32,
    },
    MemZero {
        dst: u32,
        len: u32,
    },

    // SSA plumbing
    Phi {
        dst: u32,
        incomings: Vec<(u32, u32)>,
    },

    // Legacy aliases (kept for compatibility with old backend)
    LoadConstI32 {
        dst: u32,
        value: i32,
    },
    AddI32 {
        dst: u32,
        a: u32,
        b: u32,
    },
    CmpEq {
        dst: u32,
        a: u32,
        b: u32,
    },
    PrintStr {
        sid: u32,
    },
}

impl IrInstr {
    pub fn result(&self) -> Option<u32> {
        match self {
            IrInstr::LoadConstInt { dst, .. }
            | IrInstr::LoadConstFloat { dst, .. }
            | IrInstr::LoadConstBool { dst, .. }
            | IrInstr::LoadConstStr { dst, .. }
            | IrInstr::Undef { dst, .. }
            | IrInstr::Alloca { dst, .. }
            | IrInstr::Load { dst, .. }
            | IrInstr::Gep { dst, .. }
            | IrInstr::PtrCast { dst, .. }
            | IrInstr::Add { dst, .. }
            | IrInstr::Sub { dst, .. }
            | IrInstr::Mul { dst, .. }
            | IrInstr::Div { dst, .. }
            | IrInstr::Rem { dst, .. }
            | IrInstr::Neg { dst, .. }
            | IrInstr::FAdd { dst, .. }
            | IrInstr::FSub { dst, .. }
            | IrInstr::FMul { dst, .. }
            | IrInstr::FDiv { dst, .. }
            | IrInstr::FRem { dst, .. }
            | IrInstr::FNeg { dst, .. }
            | IrInstr::And { dst, .. }
            | IrInstr::Or { dst, .. }
            | IrInstr::Xor { dst, .. }
            | IrInstr::Shl { dst, .. }
            | IrInstr::LShr { dst, .. }
            | IrInstr::AShr { dst, .. }
            | IrInstr::Not { dst, .. }
            | IrInstr::Select { dst, .. }
            | IrInstr::Cmp { dst, .. }
            | IrInstr::StructInit { dst, .. }
            | IrInstr::StructExtract { dst, .. }
            | IrInstr::StructInsert { dst, .. }
            | IrInstr::TupleInit { dst, .. }
            | IrInstr::TupleExtract { dst, .. }
            | IrInstr::ArrayInit { dst, .. }
            | IrInstr::ArrayGet { dst, .. }
            | IrInstr::SliceFromArray { dst, .. }
            | IrInstr::Call { dst: Some(dst), .. }
            | IrInstr::CallIntrinsic { dst: Some(dst), .. }
            | IrInstr::CallExtern { dst: Some(dst), .. }
            | IrInstr::MakeClosure { dst, .. }
            | IrInstr::LoadCapture { dst, .. }
            | IrInstr::Await { dst, .. }
            | IrInstr::HeapAlloc { dst, .. }
            | IrInstr::MemZero { dst, .. }
            | IrInstr::Phi { dst, .. }
            | IrInstr::LoadConstI32 { dst, .. }
            | IrInstr::AddI32 { dst, .. }
            | IrInstr::CmpEq { dst, .. } => Some(*dst),
            IrInstr::Store { .. }
            | IrInstr::ArraySet { .. }
            | IrInstr::Call { dst: None, .. }
            | IrInstr::CallIntrinsic { dst: None, .. }
            | IrInstr::CallExtern { dst: None, .. }
            | IrInstr::HeapFree { .. }
            | IrInstr::MemCopy { .. }
            | IrInstr::MemSet { .. }
            | IrInstr::PrintStr { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum IrTerm {
    Ret {
        value: Option<u32>,
    },
    Br {
        target: u32,
    },
    CondBr {
        cond: u32,
        then_b: u32,
        else_b: u32,
    },
    SwitchInt {
        scrutinee: u32,
        cases: Vec<(i128, u32)>,
        default: u32,
    },
    Unreachable,
    Invoke {
        call: Box<IrInstr>,
        normal: u32,
        unwind: u32,
    },
}

#[derive(Debug, Clone)]
pub struct IrBlock {
    pub id: u32,
    pub body: Vec<IrInstr>,
    pub term: Option<IrTerm>,
}

impl IrBlock {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            body: Vec::new(),
            term: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<IrType>,
    pub ret: IrType,
    pub blocks: Vec<IrBlock>,
    pub next_value: u32,
    pub is_async: bool,
}

impl IrFunction {
    pub fn new(name: impl Into<String>, params: Vec<IrType>, ret: IrType) -> Self {
        Self {
            name: name.into(),
            params,
            ret,
            blocks: Vec::new(),
            next_value: 0,
            is_async: false,
        }
    }

    pub fn new_block(&mut self) -> u32 {
        let id = self.blocks.len() as u32;
        self.blocks.push(IrBlock::new(id));
        id
    }

    pub fn block_mut(&mut self, id: u32) -> &mut IrBlock {
        self.blocks
            .iter_mut()
            .find(|b| b.id == id)
            .expect("invalid block id")
    }

    pub fn allocate_value(&mut self) -> u32 {
        let id = self.next_value;
        self.next_value += 1;
        id
    }
}

#[derive(Debug, Clone)]
pub enum GlobalInit {
    Zeroed,
    Bytes(Vec<u8>),
    Const { value: i128, ty: IrType },
    FromString(u32),
}

#[derive(Debug, Clone)]
pub struct IrGlobal {
    pub id: u32,
    pub name: String,
    pub ty: IrType,
    pub mutable: bool,
    pub init: GlobalInit,
}

#[derive(Debug, Clone, Default)]
pub struct IrModule {
    pub funcs: Vec<IrFunction>,
    pub globals: Vec<IrGlobal>,
    pub strings: Vec<String>,
}

impl fmt::Display for IrModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for g in &self.globals {
            writeln!(f, "@{} : {:?} = {:?}", g.name, g.ty, g.init)?;
        }
        for func in &self.funcs {
            writeln!(f, "fn {}(", func.name)?;
            for (idx, p) in func.params.iter().enumerate() {
                writeln!(f, "  %arg{}: {:?}", idx, p)?;
            }
            writeln!(f, ") -> {:?} {{", func.ret)?;
            for block in &func.blocks {
                writeln!(f, "  block{}:", block.id)?;
                for instr in &block.body {
                    writeln!(f, "    {}", InstrFmt(instr, self))?;
                }
                if let Some(term) = &block.term {
                    writeln!(f, "    {}", TermFmt(term))?;
                }
            }
            writeln!(f, "}}\n")?;
        }
        Ok(())
    }
}

struct InstrFmt<'a>(&'a IrInstr, &'a IrModule);

impl<'a> fmt::Display for InstrFmt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let module = self.1;
        match self.0 {
            IrInstr::LoadConstInt { dst, value, ty } => {
                write!(f, "%{dst} = load_const {:?} {value}", ty)
            }
            IrInstr::LoadConstFloat { dst, value, ty } => {
                write!(f, "%{dst} = load_const {:?} {}", ty, value)
            }
            IrInstr::LoadConstBool { dst, value } => {
                write!(f, "%{dst} = load_const bool {}", value)
            }
            IrInstr::LoadConstStr { dst, sid } => {
                let text = module
                    .strings
                    .get(*sid as usize)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                write!(f, "%{dst} = load_const_str \"{}\"", text.escape_default())
            }
            IrInstr::Undef { dst, ty } => write!(f, "%{dst} = undef {:?}", ty),
            IrInstr::Alloca { dst, ty } => write!(f, "%{dst} = alloca {:?}", ty),
            IrInstr::Load { dst, ptr, ty } => write!(f, "%{dst} = load {:?}, %{ptr}", ty),
            IrInstr::Store { src, ptr, ty } => write!(f, "store {:?} %{src}, %{ptr}", ty),
            IrInstr::Gep { dst, base, indices } => {
                let idxs: Vec<String> = indices
                    .iter()
                    .map(|i| match i {
                        GepIndex::Const(c) => c.to_string(),
                        GepIndex::Value(v) => format!("%{}", v),
                    })
                    .collect();
                write!(f, "%{dst} = gep %{base}, [{}]", idxs.join(", "))
            }
            IrInstr::PtrCast { dst, src, ty } => write!(f, "%{dst} = ptrcast %{src} to {:?}", ty),
            IrInstr::Add { dst, a, b, ty } => write!(f, "%{dst} = add {:?} %{a}, %{b}", ty),
            IrInstr::Sub { dst, a, b, ty } => write!(f, "%{dst} = sub {:?} %{a}, %{b}", ty),
            IrInstr::Mul { dst, a, b, ty } => write!(f, "%{dst} = mul {:?} %{a}, %{b}", ty),
            IrInstr::Div {
                dst,
                a,
                b,
                ty,
                signed,
            } => write!(f, "%{dst} = div {:?} %{a}, %{b} (signed={})", ty, signed),
            IrInstr::Rem {
                dst,
                a,
                b,
                ty,
                signed,
            } => write!(f, "%{dst} = rem {:?} %{a}, %{b} (signed={})", ty, signed),
            IrInstr::Neg { dst, val, ty } => write!(f, "%{dst} = neg {:?} %{val}", ty),
            IrInstr::FAdd { dst, a, b } => write!(f, "%{dst} = fadd %{a}, %{b}"),
            IrInstr::FSub { dst, a, b } => write!(f, "%{dst} = fsub %{a}, %{b}"),
            IrInstr::FMul { dst, a, b } => write!(f, "%{dst} = fmul %{a}, %{b}"),
            IrInstr::FDiv { dst, a, b } => write!(f, "%{dst} = fdiv %{a}, %{b}"),
            IrInstr::FRem { dst, a, b } => write!(f, "%{dst} = frem %{a}, %{b}"),
            IrInstr::FNeg { dst, val } => write!(f, "%{dst} = fneg %{val}"),
            IrInstr::And { dst, a, b, ty } => write!(f, "%{dst} = and {:?} %{a}, %{b}", ty),
            IrInstr::Or { dst, a, b, ty } => write!(f, "%{dst} = or {:?} %{a}, %{b}", ty),
            IrInstr::Xor { dst, a, b, ty } => write!(f, "%{dst} = xor {:?} %{a}, %{b}", ty),
            IrInstr::Shl { dst, a, b, ty } => write!(f, "%{dst} = shl {:?} %{a}, %{b}", ty),
            IrInstr::LShr { dst, a, b, ty } => write!(f, "%{dst} = lshr {:?} %{a}, %{b}", ty),
            IrInstr::AShr { dst, a, b, ty } => write!(f, "%{dst} = ashr {:?} %{a}, %{b}", ty),
            IrInstr::Not { dst, val, ty } => write!(f, "%{dst} = not {:?} %{val}", ty),
            IrInstr::Select {
                dst,
                cond,
                then_v,
                else_v,
            } => write!(f, "%{dst} = select %{cond}, %{then_v}, %{else_v}"),
            IrInstr::Cmp {
                dst,
                a,
                b,
                cond,
                ty,
            } => write!(f, "%{dst} = cmp {:?} {:?} %{a}, %{b}", cond, ty),
            IrInstr::StructInit { dst, fields, ty } => write!(
                f,
                "%{dst} = struct_init {:?} [{}]",
                ty,
                display_list(fields)
            ),
            IrInstr::StructExtract {
                dst,
                base,
                field_idx,
            } => write!(f, "%{dst} = struct_extract %{base}, {}", field_idx),
            IrInstr::StructInsert {
                dst,
                base,
                field_idx,
                value,
            } => write!(f, "%{dst} = struct_insert %{base}, {}, %{value}", field_idx),
            IrInstr::TupleInit { dst, items } => {
                write!(f, "%{dst} = tuple [{}]", display_list(items))
            }
            IrInstr::TupleExtract { dst, base, idx } => {
                write!(f, "%{dst} = tuple_extract %{base}, {}", idx)
            }
            IrInstr::ArrayInit {
                dst,
                elems,
                elem_ty,
            } => write!(
                f,
                "%{dst} = array_init {:?} [{}]",
                elem_ty,
                display_list(elems)
            ),
            IrInstr::ArrayGet {
                dst, base, index, ..
            } => write!(f, "%{dst} = array_get %{base}, %{index}"),
            IrInstr::ArraySet {
                base, index, value, ..
            } => write!(f, "array_set %{base}, %{index}, %{value}"),
            IrInstr::SliceFromArray { dst, base, len, .. } => {
                write!(f, "%{dst} = slice_from_array %{base}, %{len}")
            }
            IrInstr::Call { dst, func, args } => {
                if let Some(d) = dst {
                    write!(f, "%{d} = call %{func}({})", display_list(args))
                } else {
                    write!(f, "call %{func}({})", display_list(args))
                }
            }
            IrInstr::CallIntrinsic {
                dst,
                intrinsic,
                args,
            } => {
                if let Some(d) = dst {
                    write!(
                        f,
                        "%{d} = call_intrinsic {:?}({})",
                        intrinsic,
                        display_list(args)
                    )
                } else {
                    write!(f, "call_intrinsic {:?}({})", intrinsic, display_list(args))
                }
            }
            IrInstr::CallExtern {
                dst,
                symbol,
                args,
                sig,
            } => {
                if let Some(d) = dst {
                    write!(
                        f,
                        "%{d} = call_extern @{symbol:?} {:?}({})",
                        sig,
                        display_list(args)
                    )
                } else {
                    write!(
                        f,
                        "call_extern @{symbol:?} {:?}({})",
                        sig,
                        display_list(args)
                    )
                }
            }
            IrInstr::MakeClosure { dst, func, env } => {
                write!(f, "%{dst} = make_closure func#{func}, %{env}")
            }
            IrInstr::LoadCapture { dst, env, idx, .. } => {
                write!(f, "%{dst} = load_capture %{env}, {}", idx)
            }
            IrInstr::Await { dst, fut } => write!(f, "%{dst} = await %{fut}"),
            IrInstr::HeapAlloc { dst, size, align } => {
                write!(f, "%{dst} = heap_alloc %{size}, align {}", align)
            }
            IrInstr::HeapFree { ptr } => write!(f, "heap_free %{ptr}"),
            IrInstr::MemCopy { dst, src, len } => write!(f, "mem_copy %{dst}, %{src}, %{len}"),
            IrInstr::MemSet { dst, value, len } => write!(f, "mem_set %{dst}, %{value}, %{len}"),
            IrInstr::MemZero { dst, len } => write!(f, "%{dst} = mem_zero %{len}"),
            IrInstr::Phi { dst, incomings } => {
                let pairs: Vec<String> = incomings
                    .iter()
                    .map(|(b, v)| format!("(block{}, %{})", b, v))
                    .collect();
                write!(f, "%{dst} = phi {}", pairs.join(", "))
            }
            // legacy
            IrInstr::LoadConstI32 { dst, value } => write!(f, "%{dst} = load_const_i32 {}", value),
            IrInstr::AddI32 { dst, a, b } => write!(f, "%{dst} = add_i32 %{a}, %{b}"),
            IrInstr::CmpEq { dst, a, b } => write!(f, "%{dst} = cmp_eq %{a}, %{b}"),
            IrInstr::PrintStr { sid } => {
                let text = module
                    .strings
                    .get(*sid as usize)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                write!(f, "print_str \"{}\"", text.escape_default())
            }
        }
    }
}

fn display_list(values: &[u32]) -> String {
    values
        .iter()
        .map(|v| format!("%{}", v))
        .collect::<Vec<_>>()
        .join(", ")
}

struct TermFmt<'a>(&'a IrTerm);

impl<'a> fmt::Display for TermFmt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            IrTerm::Ret { value } => {
                if let Some(v) = value {
                    write!(f, "ret %{}", v)
                } else {
                    write!(f, "ret")
                }
            }
            IrTerm::Br { target } => write!(f, "br block{}", target),
            IrTerm::CondBr {
                cond,
                then_b,
                else_b,
            } => {
                write!(f, "condbr %{}, block{}, block{}", cond, then_b, else_b)
            }
            IrTerm::SwitchInt {
                scrutinee,
                cases,
                default,
            } => {
                let arms: Vec<String> = cases
                    .iter()
                    .map(|(c, b)| format!("{} -> block{}", c, b))
                    .collect();
                write!(
                    f,
                    "switch %{}, [{}], default block{}",
                    scrutinee,
                    arms.join(", "),
                    default
                )
            }
            IrTerm::Unreachable => write!(f, "unreachable"),
            IrTerm::Invoke {
                call,
                normal,
                unwind,
            } => {
                write!(
                    f,
                    "invoke {} to block{}, unwind block{}",
                    InstrFmt(call, &IrModule::default()),
                    normal,
                    unwind
                )
            }
        }
    }
}
