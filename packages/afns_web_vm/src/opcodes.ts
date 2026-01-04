/**
 * AFNS Web VM Opcodes
 * Must match src/bytecode/mod.rs Opcode enum exactly
 */

export const Opcode = {
    // Constants and variables
    CONST: 0x01,
    LOAD_LOCAL: 0x02,
    STORE_LOCAL: 0x03,
    LOAD_GLOBAL: 0x04,
    STORE_GLOBAL: 0x05,

    // Function calls
    CALL: 0x10,
    RET: 0x11,

    // Control flow
    JUMP: 0x20,
    JUMP_IF_FALSE: 0x21,

    // Closures
    MAKE_CLOSURE: 0x30,
    INVOKE_CLOSURE: 0x31,

    // Collections
    NEW_VEC: 0x40,
    VEC_PUSH: 0x41,
    NEW_MAP: 0x42,
    MAP_SET: 0x43,

    // GUI operations
    GUI_CREATE_WIDGET: 0x50,
    GUI_SET_PROP: 0x51,
    GUI_ADD_CHILD: 0x52,
    GUI_SET_HANDLER: 0x53,
    GUI_COMMIT_ROOT: 0x54,

    // Logging
    LOG_INFO: 0x60,

    // Binary operations
    ADD: 0x70,
    SUB: 0x71,
    MUL: 0x72,
    DIV: 0x73,
    MOD: 0x74,
    EQ: 0x75,
    NE: 0x76,
    LT: 0x77,
    LE: 0x78,
    GT: 0x79,
    GE: 0x7a,
    AND: 0x7b,
    OR: 0x7c,
    NOT: 0x7d,
    NEG: 0x7e,

    // Stack operations
    DUP: 0x80,
    POP: 0x81,
    GET_PROP: 0x82,
    SET_PROP: 0x83,
    CONCAT: 0x84,

    // State operations
    STATE_CREATE: 0x90,
    STATE_GET: 0x91,
    STATE_SET: 0x92,
} as const;

export type OpcodeValue = (typeof Opcode)[keyof typeof Opcode];

export function opcodeName(op: number): string {
    for (const [name, value] of Object.entries(Opcode)) {
        if (value === op) return name;
    }
    return `UNKNOWN(0x${op.toString(16)})`;
}
