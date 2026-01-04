//! AFBC (ApexForge Bytecode) format for web VM execution.
//!
//! Binary format (v1):
//! - Magic: "AFBC" (4 bytes)
//! - Version: u16 (little-endian)
//! - Flags: u32 (reserved)
//! - Constant pool count: u32
//! - Constant pool entries: [tag: u8, payload...]
//! - Function count: u32
//! - Function table: [name_idx: u32, arity: u16, code_offset: u32, code_len: u32]
//! - Bytecode section: raw bytes
//! - Optional debug section

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};

/// AFBC file magic bytes
pub const MAGIC: &[u8; 4] = b"AFBC";

/// Current format version
pub const VERSION: u16 = 1;

// ============================================================================
// Opcodes
// ============================================================================

/// VM opcodes for web execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    /// Push constant from pool: CONST <u16 index>
    Const = 0x01,
    /// Load local variable: LOAD_LOCAL <u16 slot>
    LoadLocal = 0x02,
    /// Store local variable: STORE_LOCAL <u16 slot>
    StoreLocal = 0x03,
    /// Load global variable: LOAD_GLOBAL <u16 name_idx>
    LoadGlobal = 0x04,
    /// Store global variable: STORE_GLOBAL <u16 name_idx>
    StoreGlobal = 0x05,
    /// Call function: CALL <u16 func_idx> <u8 argc>
    Call = 0x10,
    /// Return from function: RET
    Ret = 0x11,
    /// Unconditional jump: JUMP <i16 offset>
    Jump = 0x20,
    /// Jump if top of stack is false: JUMP_IF_FALSE <i16 offset>
    JumpIfFalse = 0x21,
    /// Create closure: MAKE_CLOSURE <u16 func_idx> <u8 capture_count>
    MakeClosure = 0x30,
    /// Invoke closure: INVOKE_CLOSURE <u8 argc>
    InvokeClosure = 0x31,
    /// Create new vector: NEW_VEC
    NewVec = 0x40,
    /// Push to vector: VEC_PUSH
    VecPush = 0x41,
    /// Create new map: NEW_MAP
    NewMap = 0x42,
    /// Set map entry: MAP_SET
    MapSet = 0x43,
    /// Create widget: GUI_CREATE_WIDGET <u16 type_idx>
    GuiCreateWidget = 0x50,
    /// Set widget property: GUI_SET_PROP <u16 key_idx>
    GuiSetProp = 0x51,
    /// Add child to widget: GUI_ADD_CHILD
    GuiAddChild = 0x52,
    /// Set event handler on widget: GUI_SET_HANDLER <u16 event_type_idx>
    GuiSetHandler = 0x53,
    /// Commit widget as root: GUI_COMMIT_ROOT
    GuiCommitRoot = 0x54,
    /// Log info: LOG_INFO
    LogInfo = 0x60,
    /// Binary operations
    Add = 0x70,
    Sub = 0x71,
    Mul = 0x72,
    Div = 0x73,
    Mod = 0x74,
    Eq = 0x75,
    Ne = 0x76,
    Lt = 0x77,
    Le = 0x78,
    Gt = 0x79,
    Ge = 0x7A,
    And = 0x7B,
    Or = 0x7C,
    Not = 0x7D,
    Neg = 0x7E,
    /// Duplicate top of stack
    Dup = 0x80,
    /// Pop and discard top of stack
    Pop = 0x81,
    /// Get property: GET_PROP <u16 key_idx>
    GetProp = 0x82,
    /// Set property: SET_PROP <u16 key_idx>
    SetProp = 0x83,
    /// String concatenation
    Concat = 0x84,
    /// State operations
    StateCreate = 0x90,
    StateGet = 0x91,
    StateSet = 0x92,
}

impl Opcode {
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Self::Const),
            0x02 => Some(Self::LoadLocal),
            0x03 => Some(Self::StoreLocal),
            0x04 => Some(Self::LoadGlobal),
            0x05 => Some(Self::StoreGlobal),
            0x10 => Some(Self::Call),
            0x11 => Some(Self::Ret),
            0x20 => Some(Self::Jump),
            0x21 => Some(Self::JumpIfFalse),
            0x30 => Some(Self::MakeClosure),
            0x31 => Some(Self::InvokeClosure),
            0x40 => Some(Self::NewVec),
            0x41 => Some(Self::VecPush),
            0x42 => Some(Self::NewMap),
            0x43 => Some(Self::MapSet),
            0x50 => Some(Self::GuiCreateWidget),
            0x51 => Some(Self::GuiSetProp),
            0x52 => Some(Self::GuiAddChild),
            0x53 => Some(Self::GuiSetHandler),
            0x54 => Some(Self::GuiCommitRoot),
            0x60 => Some(Self::LogInfo),
            0x70 => Some(Self::Add),
            0x71 => Some(Self::Sub),
            0x72 => Some(Self::Mul),
            0x73 => Some(Self::Div),
            0x74 => Some(Self::Mod),
            0x75 => Some(Self::Eq),
            0x76 => Some(Self::Ne),
            0x77 => Some(Self::Lt),
            0x78 => Some(Self::Le),
            0x79 => Some(Self::Gt),
            0x7A => Some(Self::Ge),
            0x7B => Some(Self::And),
            0x7C => Some(Self::Or),
            0x7D => Some(Self::Not),
            0x7E => Some(Self::Neg),
            0x80 => Some(Self::Dup),
            0x81 => Some(Self::Pop),
            0x82 => Some(Self::GetProp),
            0x83 => Some(Self::SetProp),
            0x84 => Some(Self::Concat),
            0x90 => Some(Self::StateCreate),
            0x91 => Some(Self::StateGet),
            0x92 => Some(Self::StateSet),
            _ => None,
        }
    }
}

// ============================================================================
// Constant Pool
// ============================================================================

/// Constant pool entry tags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConstantTag {
    Utf8 = 1,
    Int64 = 2,
    Float64 = 3,
    Bool = 4,
    Null = 5,
}

/// A constant value in the constant pool
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Utf8(String),
    Int64(i64),
    Float64(f64),
    Bool(bool),
    Null,
}

impl Constant {
    pub fn tag(&self) -> ConstantTag {
        match self {
            Self::Utf8(_) => ConstantTag::Utf8,
            Self::Int64(_) => ConstantTag::Int64,
            Self::Float64(_) => ConstantTag::Float64,
            Self::Bool(_) => ConstantTag::Bool,
            Self::Null => ConstantTag::Null,
        }
    }

    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&[self.tag() as u8])?;
        match self {
            Self::Utf8(s) => {
                let bytes = s.as_bytes();
                w.write_all(&(bytes.len() as u32).to_le_bytes())?;
                w.write_all(bytes)?;
            }
            Self::Int64(n) => {
                w.write_all(&n.to_le_bytes())?;
            }
            Self::Float64(n) => {
                w.write_all(&n.to_le_bytes())?;
            }
            Self::Bool(b) => {
                w.write_all(&[if *b { 1 } else { 0 }])?;
            }
            Self::Null => {}
        }
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut tag = [0u8; 1];
        r.read_exact(&mut tag)?;
        match tag[0] {
            1 => {
                let mut len_bytes = [0u8; 4];
                r.read_exact(&mut len_bytes)?;
                let len = u32::from_le_bytes(len_bytes) as usize;
                let mut bytes = vec![0u8; len];
                r.read_exact(&mut bytes)?;
                Ok(Self::Utf8(String::from_utf8_lossy(&bytes).into_owned()))
            }
            2 => {
                let mut bytes = [0u8; 8];
                r.read_exact(&mut bytes)?;
                Ok(Self::Int64(i64::from_le_bytes(bytes)))
            }
            3 => {
                let mut bytes = [0u8; 8];
                r.read_exact(&mut bytes)?;
                Ok(Self::Float64(f64::from_le_bytes(bytes)))
            }
            4 => {
                let mut bytes = [0u8; 1];
                r.read_exact(&mut bytes)?;
                Ok(Self::Bool(bytes[0] != 0))
            }
            5 => Ok(Self::Null),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown constant tag: {}", tag[0]),
            )),
        }
    }
}

// ============================================================================
// Function Entry
// ============================================================================

/// A function entry in the function table
#[derive(Debug, Clone)]
pub struct FunctionEntry {
    /// Index into constant pool for function name
    pub name_idx: u32,
    /// Number of parameters
    pub arity: u16,
    /// Number of local variables (including parameters)
    pub locals: u16,
    /// Offset into bytecode section
    pub code_offset: u32,
    /// Length of bytecode for this function
    pub code_len: u32,
}

impl FunctionEntry {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.name_idx.to_le_bytes())?;
        w.write_all(&self.arity.to_le_bytes())?;
        w.write_all(&self.locals.to_le_bytes())?;
        w.write_all(&self.code_offset.to_le_bytes())?;
        w.write_all(&self.code_len.to_le_bytes())?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf4 = [0u8; 4];
        let mut buf2 = [0u8; 2];

        r.read_exact(&mut buf4)?;
        let name_idx = u32::from_le_bytes(buf4);
        r.read_exact(&mut buf2)?;
        let arity = u16::from_le_bytes(buf2);
        r.read_exact(&mut buf2)?;
        let locals = u16::from_le_bytes(buf2);
        r.read_exact(&mut buf4)?;
        let code_offset = u32::from_le_bytes(buf4);
        r.read_exact(&mut buf4)?;
        let code_len = u32::from_le_bytes(buf4);

        Ok(Self {
            name_idx,
            arity,
            locals,
            code_offset,
            code_len,
        })
    }
}

// ============================================================================
// Source Map (Debug Section)
// ============================================================================

/// Source map entry for debugging
#[derive(Debug, Clone)]
pub struct SourceMapEntry {
    /// Start byte offset in bytecode
    pub code_start: u32,
    /// End byte offset in bytecode
    pub code_end: u32,
    /// Source line number
    pub line: u32,
    /// Source column
    pub column: u32,
}

impl SourceMapEntry {
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.code_start.to_le_bytes())?;
        w.write_all(&self.code_end.to_le_bytes())?;
        w.write_all(&self.line.to_le_bytes())?;
        w.write_all(&self.column.to_le_bytes())?;
        Ok(())
    }

    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let code_start = u32::from_le_bytes(buf);
        r.read_exact(&mut buf)?;
        let code_end = u32::from_le_bytes(buf);
        r.read_exact(&mut buf)?;
        let line = u32::from_le_bytes(buf);
        r.read_exact(&mut buf)?;
        let column = u32::from_le_bytes(buf);
        Ok(Self {
            code_start,
            code_end,
            line,
            column,
        })
    }
}

// ============================================================================
// AFBC Module
// ============================================================================

/// Complete AFBC module ready for serialization
#[derive(Debug, Clone)]
pub struct AfbcModule {
    /// Version flags
    pub flags: u32,
    /// Constant pool
    pub constants: Vec<Constant>,
    /// Function table
    pub functions: Vec<FunctionEntry>,
    /// Raw bytecode
    pub bytecode: Vec<u8>,
    /// Optional source map
    pub source_map: Vec<SourceMapEntry>,
}

impl AfbcModule {
    pub fn new() -> Self {
        Self {
            flags: 0,
            constants: Vec::new(),
            functions: Vec::new(),
            bytecode: Vec::new(),
            source_map: Vec::new(),
        }
    }

    /// Add a constant to the pool, returning its index
    pub fn add_constant(&mut self, constant: Constant) -> u32 {
        // Check if constant already exists
        for (i, c) in self.constants.iter().enumerate() {
            if c == &constant {
                return i as u32;
            }
        }
        let idx = self.constants.len() as u32;
        self.constants.push(constant);
        idx
    }

    /// Add a string constant
    pub fn add_string(&mut self, s: &str) -> u32 {
        self.add_constant(Constant::Utf8(s.to_string()))
    }

    /// Add a function
    pub fn add_function(&mut self, entry: FunctionEntry) -> u32 {
        let idx = self.functions.len() as u32;
        self.functions.push(entry);
        idx
    }

    /// Write to binary format
    pub fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        // Magic
        w.write_all(MAGIC)?;

        // Version
        w.write_all(&VERSION.to_le_bytes())?;

        // Flags
        w.write_all(&self.flags.to_le_bytes())?;

        // Constant pool count and entries
        w.write_all(&(self.constants.len() as u32).to_le_bytes())?;
        for constant in &self.constants {
            constant.write(w)?;
        }

        // Function count and table
        w.write_all(&(self.functions.len() as u32).to_le_bytes())?;
        for func in &self.functions {
            func.write(w)?;
        }

        // Bytecode section
        w.write_all(&(self.bytecode.len() as u32).to_le_bytes())?;
        w.write_all(&self.bytecode)?;

        // Debug section (source map)
        let has_debug = !self.source_map.is_empty();
        w.write_all(&[if has_debug { 1 } else { 0 }])?;
        if has_debug {
            w.write_all(&(self.source_map.len() as u32).to_le_bytes())?;
            for entry in &self.source_map {
                entry.write(w)?;
            }
        }

        Ok(())
    }

    /// Read from binary format
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        // Magic
        let mut magic = [0u8; 4];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid AFBC magic",
            ));
        }

        // Version
        let mut version_bytes = [0u8; 2];
        r.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported AFBC version: {}", version),
            ));
        }

        // Flags
        let mut flags_bytes = [0u8; 4];
        r.read_exact(&mut flags_bytes)?;
        let flags = u32::from_le_bytes(flags_bytes);

        // Constant pool
        let mut count_bytes = [0u8; 4];
        r.read_exact(&mut count_bytes)?;
        let const_count = u32::from_le_bytes(count_bytes) as usize;
        let mut constants = Vec::with_capacity(const_count);
        for _ in 0..const_count {
            constants.push(Constant::read(r)?);
        }

        // Function table
        r.read_exact(&mut count_bytes)?;
        let func_count = u32::from_le_bytes(count_bytes) as usize;
        let mut functions = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            functions.push(FunctionEntry::read(r)?);
        }

        // Bytecode section
        r.read_exact(&mut count_bytes)?;
        let bytecode_len = u32::from_le_bytes(count_bytes) as usize;
        let mut bytecode = vec![0u8; bytecode_len];
        r.read_exact(&mut bytecode)?;

        // Debug section
        let mut has_debug = [0u8; 1];
        r.read_exact(&mut has_debug)?;
        let source_map = if has_debug[0] != 0 {
            r.read_exact(&mut count_bytes)?;
            let map_count = u32::from_le_bytes(count_bytes) as usize;
            let mut map = Vec::with_capacity(map_count);
            for _ in 0..map_count {
                map.push(SourceMapEntry::read(r)?);
            }
            map
        } else {
            Vec::new()
        };

        Ok(Self {
            flags,
            constants,
            functions,
            bytecode,
            source_map,
        })
    }

    /// Compute SHA256 hash of the module
    pub fn hash(&self) -> String {
        let mut buf = Vec::new();
        self.write(&mut buf).expect("write to vec should not fail");
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let result = hasher.finalize();
        hex::encode(&result[..])
    }

    /// Compute short build ID (first 12 hex chars of hash)
    pub fn build_id(&self) -> String {
        self.hash()[..12].to_string()
    }
}

impl Default for AfbcModule {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Bytecode Builder Helper
// ============================================================================

/// Helper for building bytecode
pub struct BytecodeBuilder {
    code: Vec<u8>,
}

impl BytecodeBuilder {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    pub fn emit(&mut self, op: Opcode) {
        self.code.push(op as u8);
    }

    pub fn emit_u8(&mut self, value: u8) {
        self.code.push(value);
    }

    pub fn emit_u16(&mut self, value: u16) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    pub fn emit_i16(&mut self, value: i16) {
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    pub fn emit_const(&mut self, idx: u16) {
        self.emit(Opcode::Const);
        self.emit_u16(idx);
    }

    pub fn emit_load_local(&mut self, slot: u16) {
        self.emit(Opcode::LoadLocal);
        self.emit_u16(slot);
    }

    pub fn emit_store_local(&mut self, slot: u16) {
        self.emit(Opcode::StoreLocal);
        self.emit_u16(slot);
    }

    pub fn emit_call(&mut self, func_idx: u16, argc: u8) {
        self.emit(Opcode::Call);
        self.emit_u16(func_idx);
        self.emit_u8(argc);
    }

    pub fn emit_jump(&mut self, offset: i16) {
        self.emit(Opcode::Jump);
        self.emit_i16(offset);
    }

    pub fn emit_jump_if_false(&mut self, offset: i16) {
        self.emit(Opcode::JumpIfFalse);
        self.emit_i16(offset);
    }

    pub fn emit_gui_create_widget(&mut self, type_idx: u16) {
        self.emit(Opcode::GuiCreateWidget);
        self.emit_u16(type_idx);
    }

    pub fn emit_gui_set_prop(&mut self, key_idx: u16) {
        self.emit(Opcode::GuiSetProp);
        self.emit_u16(key_idx);
    }

    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    pub fn patch_jump(&mut self, offset: usize, target: i16) {
        let bytes = target.to_le_bytes();
        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
    }

    pub fn finish(self) -> Vec<u8> {
        self.code
    }
}

impl Default for BytecodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut module = AfbcModule::new();
        module.add_string("apex");
        module.add_string("text");
        module.add_constant(Constant::Int64(42));
        module.add_constant(Constant::Float64(3.14));
        module.add_constant(Constant::Bool(true));
        module.add_constant(Constant::Null);

        module.functions.push(FunctionEntry {
            name_idx: 0,
            arity: 0,
            locals: 2,
            code_offset: 0,
            code_len: 10,
        });

        module.bytecode = vec![0x01, 0x00, 0x00, 0x11, 0x50, 0x01, 0x00, 0x51, 0x01, 0x00];

        module.source_map.push(SourceMapEntry {
            code_start: 0,
            code_end: 4,
            line: 1,
            column: 0,
        });

        let mut buf = Vec::new();
        module.write(&mut buf).unwrap();

        let loaded = AfbcModule::read(&mut buf.as_slice()).unwrap();

        assert_eq!(loaded.constants.len(), module.constants.len());
        assert_eq!(loaded.functions.len(), module.functions.len());
        assert_eq!(loaded.bytecode, module.bytecode);
        assert_eq!(loaded.source_map.len(), module.source_map.len());
    }

    #[test]
    fn test_build_id_determinism() {
        let mut module1 = AfbcModule::new();
        module1.add_string("hello");
        module1.add_constant(Constant::Int64(123));

        let mut module2 = AfbcModule::new();
        module2.add_string("hello");
        module2.add_constant(Constant::Int64(123));

        assert_eq!(module1.build_id(), module2.build_id());

        // Different content should produce different ID
        let mut module3 = AfbcModule::new();
        module3.add_string("world");
        module3.add_constant(Constant::Int64(123));

        assert_ne!(module1.build_id(), module3.build_id());
    }

    #[test]
    fn test_bytecode_builder() {
        let mut builder = BytecodeBuilder::new();
        builder.emit_const(0);
        builder.emit_load_local(1);
        builder.emit(Opcode::Add);
        builder.emit(Opcode::Ret);

        let code = builder.finish();
        assert_eq!(code.len(), 8);
        assert_eq!(code[0], Opcode::Const as u8);
    }
}
