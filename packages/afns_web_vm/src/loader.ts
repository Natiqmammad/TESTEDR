/**
 * AFBC (ApexForge Bytecode) Loader
 * Parses .afbc binary files into executable modules
 */

export const MAGIC = new TextEncoder().encode('AFBC');
export const VERSION = 1;

// Constant tags
export const ConstantTag = {
    UTF8: 1,
    INT64: 2,
    FLOAT64: 3,
    BOOL: 4,
    NULL: 5,
} as const;

export type Constant =
    | { tag: 'utf8'; value: string }
    | { tag: 'int64'; value: bigint }
    | { tag: 'float64'; value: number }
    | { tag: 'bool'; value: boolean }
    | { tag: 'null' };

export interface FunctionEntry {
    nameIdx: number;
    arity: number;
    locals: number;
    codeOffset: number;
    codeLen: number;
}

export interface SourceMapEntry {
    codeStart: number;
    codeEnd: number;
    line: number;
    column: number;
}

export interface AfbcModule {
    version: number;
    flags: number;
    constants: Constant[];
    functions: FunctionEntry[];
    bytecode: Uint8Array;
    sourceMap: SourceMapEntry[];
}

/**
 * Binary reader helper
 */
class BinaryReader {
    private view: DataView;
    private offset: number = 0;

    constructor(buffer: ArrayBuffer) {
        this.view = new DataView(buffer);
    }

    readU8(): number {
        const value = this.view.getUint8(this.offset);
        this.offset += 1;
        return value;
    }

    readU16(): number {
        const value = this.view.getUint16(this.offset, true);
        this.offset += 2;
        return value;
    }

    readU32(): number {
        const value = this.view.getUint32(this.offset, true);
        this.offset += 4;
        return value;
    }

    readI64(): bigint {
        const value = this.view.getBigInt64(this.offset, true);
        this.offset += 8;
        return value;
    }

    readF64(): number {
        const value = this.view.getFloat64(this.offset, true);
        this.offset += 8;
        return value;
    }

    readBytes(length: number): Uint8Array {
        const bytes = new Uint8Array(this.view.buffer, this.offset, length);
        this.offset += length;
        return bytes.slice(); // Copy to avoid view issues
    }

    readString(): string {
        const length = this.readU32();
        const bytes = this.readBytes(length);
        return new TextDecoder().decode(bytes);
    }
}

/**
 * Load and parse an AFBC module from binary data
 */
export function loadAfbcModule(buffer: ArrayBuffer): AfbcModule {
    const reader = new BinaryReader(buffer);

    // Check magic
    const magic = reader.readBytes(4);
    if (
        magic[0] !== MAGIC[0] ||
        magic[1] !== MAGIC[1] ||
        magic[2] !== MAGIC[2] ||
        magic[3] !== MAGIC[3]
    ) {
        throw new Error('Invalid AFBC magic');
    }

    // Version
    const version = reader.readU16();
    if (version !== VERSION) {
        throw new Error(`Unsupported AFBC version: ${version}`);
    }

    // Flags
    const flags = reader.readU32();

    // Constants
    const constCount = reader.readU32();
    const constants: Constant[] = [];
    for (let i = 0; i < constCount; i++) {
        const tag = reader.readU8();
        switch (tag) {
            case ConstantTag.UTF8:
                constants.push({ tag: 'utf8', value: reader.readString() });
                break;
            case ConstantTag.INT64:
                constants.push({ tag: 'int64', value: reader.readI64() });
                break;
            case ConstantTag.FLOAT64:
                constants.push({ tag: 'float64', value: reader.readF64() });
                break;
            case ConstantTag.BOOL:
                constants.push({ tag: 'bool', value: reader.readU8() !== 0 });
                break;
            case ConstantTag.NULL:
                constants.push({ tag: 'null' });
                break;
            default:
                throw new Error(`Unknown constant tag: ${tag}`);
        }
    }

    // Functions
    const funcCount = reader.readU32();
    const functions: FunctionEntry[] = [];
    for (let i = 0; i < funcCount; i++) {
        functions.push({
            nameIdx: reader.readU32(),
            arity: reader.readU16(),
            locals: reader.readU16(),
            codeOffset: reader.readU32(),
            codeLen: reader.readU32(),
        });
    }

    // Bytecode
    const bytecodeLen = reader.readU32();
    const bytecode = reader.readBytes(bytecodeLen);

    // Source map (optional)
    const hasDebug = reader.readU8() !== 0;
    const sourceMap: SourceMapEntry[] = [];
    if (hasDebug) {
        const mapCount = reader.readU32();
        for (let i = 0; i < mapCount; i++) {
            sourceMap.push({
                codeStart: reader.readU32(),
                codeEnd: reader.readU32(),
                line: reader.readU32(),
                column: reader.readU32(),
            });
        }
    }

    return {
        version,
        flags,
        constants,
        functions,
        bytecode,
        sourceMap,
    };
}

/**
 * Fetch and load an AFBC module from URL
 */
export async function fetchAfbcModule(url: string): Promise<AfbcModule> {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`Failed to fetch AFBC module: ${response.status}`);
    }
    const buffer = await response.arrayBuffer();
    return loadAfbcModule(buffer);
}

/**
 * Get constant as string
 */
export function getConstantString(module: AfbcModule, index: number): string {
    const constant = module.constants[index];
    if (!constant) {
        throw new Error(`Invalid constant index: ${index}`);
    }
    if (constant.tag !== 'utf8') {
        throw new Error(`Expected string constant at index ${index}, got ${constant.tag}`);
    }
    return constant.value;
}
