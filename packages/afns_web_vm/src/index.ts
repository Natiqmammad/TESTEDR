/**
 * AFNS Web VM - Public API
 */

export { Opcode, opcodeName } from './opcodes';
export {
    loadAfbcModule,
    fetchAfbcModule,
    getConstantString,
    type AfbcModule,
    type Constant,
    type FunctionEntry,
    type SourceMapEntry,
} from './loader';
export {
    VM,
    valueToString,
    isTruthy,
    NULL_VALUE,
    type Value,
    type VMCallbacks,
} from './vm';
