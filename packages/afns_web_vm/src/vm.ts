/**
 * AFNS Web VM - Bytecode execution engine
 */

import { Opcode, opcodeName } from './opcodes';
import { AfbcModule, Constant, FunctionEntry, getConstantString } from './loader';

// ============================================================================
// Value Types
// ============================================================================

export type Value =
    | { type: 'null' }
    | { type: 'bool'; value: boolean }
    | { type: 'int'; value: bigint }
    | { type: 'float'; value: number }
    | { type: 'string'; value: string }
    | { type: 'vec'; items: Value[] }
    | { type: 'map'; entries: Map<string, Value> }
    | { type: 'closure'; funcIdx: number; captures: Value[] }
    | { type: 'widget'; id: string; widgetType: string; props: Map<string, Value>; children: Value[]; handlers: Map<string, Value> }
    | { type: 'state'; id: number; value: Value };

export const NULL_VALUE: Value = { type: 'null' };

export function valueToString(v: Value): string {
    switch (v.type) {
        case 'null': return 'null';
        case 'bool': return String(v.value);
        case 'int': return String(v.value);
        case 'float': return String(v.value);
        case 'string': return v.value;
        case 'vec': return `[${v.items.map(valueToString).join(', ')}]`;
        case 'map': {
            const entries = Array.from(v.entries.entries())
                .map(([k, val]) => `${k}: ${valueToString(val)}`)
                .join(', ');
            return `{${entries}}`;
        }
        case 'closure': return `<closure:${v.funcIdx}>`;
        case 'widget': return `<widget:${v.widgetType}#${v.id}>`;
        case 'state': return `<state:${v.id}>`;
    }
}

export function isTruthy(v: Value): boolean {
    if (v.type === 'bool') return v.value;
    if (v.type === 'null') return false;
    return true;
}

// ============================================================================
// Call Frame
// ============================================================================

interface CallFrame {
    func: FunctionEntry;
    ip: number; // Instruction pointer (relative to func.codeOffset)
    bp: number; // Base pointer into stack for locals
}

// ============================================================================
// VM State
// ============================================================================

export interface VMCallbacks {
    onLog?: (message: string) => void;
    onRender?: (root: Value) => void;
    onError?: (error: Error) => void;
}

export class VM {
    private module: AfbcModule;
    private stack: Value[] = [];
    private globals: Map<string, Value> = new Map();
    private frames: CallFrame[] = [];
    private callbacks: VMCallbacks;
    private stateCounter = 0;
    private states: Map<number, Value> = new Map();
    private widgetIdCounter = 0;
    private rootWidget: Value | null = null;
    private rebuildScheduled = false;

    constructor(module: AfbcModule, callbacks: VMCallbacks = {}) {
        this.module = module;
        this.callbacks = callbacks;
    }

    /**
     * Run the module starting from the apex function
     */
    run(): void {
        const apexIdx = this.findFunction('apex');
        if (apexIdx === -1) {
            throw new Error('No apex function found in module');
        }
        this.callFunction(apexIdx, []);
        this.execute();
    }

    /**
     * Handle an event from the renderer
     */
    handleEvent(widgetId: string, eventType: string, payload: unknown): void {
        // Find the widget and its handler
        const handler = this.findHandler(this.rootWidget, widgetId, eventType);
        if (handler && handler.type === 'closure') {
            this.callClosure(handler, []);
            this.execute();
            // Schedule rebuild after handler
            this.scheduleRebuild();
        }
    }

    private findHandler(widget: Value | null, id: string, eventType: string): Value | null {
        if (!widget || widget.type !== 'widget') return null;
        if (widget.id === id) {
            return widget.handlers.get(eventType) || null;
        }
        for (const child of widget.children) {
            const found = this.findHandler(child, id, eventType);
            if (found) return found;
        }
        return null;
    }

    private scheduleRebuild(): void {
        if (this.rebuildScheduled) return;
        this.rebuildScheduled = true;
        queueMicrotask(() => {
            this.rebuildScheduled = false;
            this.rebuild();
        });
    }

    private rebuild(): void {
        // Re-run apex to get new widget tree
        this.widgetIdCounter = 0;
        this.rootWidget = null;
        this.run();
    }

    private findFunction(name: string): number {
        for (let i = 0; i < this.module.functions.length; i++) {
            const func = this.module.functions[i];
            const funcName = getConstantString(this.module, func.nameIdx);
            if (funcName === name) return i;
        }
        return -1;
    }

    private callFunction(funcIdx: number, args: Value[]): void {
        const func = this.module.functions[funcIdx];
        const bp = this.stack.length;

        // Push arguments as locals
        for (const arg of args) {
            this.stack.push(arg);
        }

        // Fill remaining locals with null
        for (let i = args.length; i < func.locals; i++) {
            this.stack.push(NULL_VALUE);
        }

        this.frames.push({ func, ip: 0, bp });
    }

    private callClosure(closure: Value, args: Value[]): void {
        if (closure.type !== 'closure') {
            throw new Error('Expected closure');
        }
        const func = this.module.functions[closure.funcIdx];
        const bp = this.stack.length;

        // Push captures first, then args
        for (const cap of closure.captures) {
            this.stack.push(cap);
        }
        for (const arg of args) {
            this.stack.push(arg);
        }

        // Fill remaining locals
        for (let i = closure.captures.length + args.length; i < func.locals; i++) {
            this.stack.push(NULL_VALUE);
        }

        this.frames.push({ func, ip: 0, bp });
    }

    private execute(): void {
        while (this.frames.length > 0) {
            const frame = this.frames[this.frames.length - 1];
            const code = this.module.bytecode;
            const baseOffset = frame.func.codeOffset;

            while (frame.ip < frame.func.codeLen) {
                const op = code[baseOffset + frame.ip];
                frame.ip++;

                switch (op) {
                    case Opcode.CONST: {
                        const idx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        this.stack.push(this.constantToValue(this.module.constants[idx]));
                        break;
                    }

                    case Opcode.LOAD_LOCAL: {
                        const slot = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        this.stack.push(this.stack[frame.bp + slot]);
                        break;
                    }

                    case Opcode.STORE_LOCAL: {
                        const slot = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        this.stack[frame.bp + slot] = this.stack.pop()!;
                        break;
                    }

                    case Opcode.LOAD_GLOBAL: {
                        const nameIdx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const name = getConstantString(this.module, nameIdx);
                        this.stack.push(this.globals.get(name) || NULL_VALUE);
                        break;
                    }

                    case Opcode.STORE_GLOBAL: {
                        const nameIdx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const name = getConstantString(this.module, nameIdx);
                        this.globals.set(name, this.stack.pop()!);
                        break;
                    }

                    case Opcode.CALL: {
                        const funcIdx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const argc = code[baseOffset + frame.ip];
                        frame.ip++;
                        const args = this.stack.splice(-argc);
                        this.callFunction(funcIdx, args);
                        break;
                    }

                    case Opcode.RET: {
                        const result = this.stack.length > frame.bp + frame.func.locals
                            ? this.stack.pop()!
                            : NULL_VALUE;
                        // Pop locals
                        this.stack.length = frame.bp;
                        this.frames.pop();
                        if (this.frames.length > 0) {
                            this.stack.push(result);
                        }
                        break;
                    }

                    case Opcode.JUMP: {
                        const offset = this.readI16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        frame.ip += offset;
                        break;
                    }

                    case Opcode.JUMP_IF_FALSE: {
                        const offset = this.readI16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const cond = this.stack.pop()!;
                        if (!isTruthy(cond)) {
                            frame.ip += offset;
                        }
                        break;
                    }

                    case Opcode.MAKE_CLOSURE: {
                        const funcIdx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const captureCount = code[baseOffset + frame.ip];
                        frame.ip++;
                        const captures = this.stack.splice(-captureCount);
                        this.stack.push({ type: 'closure', funcIdx, captures });
                        break;
                    }

                    case Opcode.INVOKE_CLOSURE: {
                        const argc = code[baseOffset + frame.ip];
                        frame.ip++;
                        const args = this.stack.splice(-argc);
                        const closure = this.stack.pop()!;
                        if (closure.type !== 'closure') {
                            throw new Error('Expected closure');
                        }
                        this.callClosure(closure, args);
                        break;
                    }

                    case Opcode.NEW_VEC:
                        this.stack.push({ type: 'vec', items: [] });
                        break;

                    case Opcode.VEC_PUSH: {
                        const item = this.stack.pop()!;
                        const vec = this.stack[this.stack.length - 1];
                        if (vec.type !== 'vec') throw new Error('Expected vec');
                        vec.items.push(item);
                        break;
                    }

                    case Opcode.NEW_MAP:
                        this.stack.push({ type: 'map', entries: new Map() });
                        break;

                    case Opcode.MAP_SET: {
                        const value = this.stack.pop()!;
                        const key = this.stack.pop()!;
                        const map = this.stack[this.stack.length - 1];
                        if (map.type !== 'map') throw new Error('Expected map');
                        if (key.type !== 'string') throw new Error('Map key must be string');
                        map.entries.set(key.value, value);
                        break;
                    }

                    case Opcode.GUI_CREATE_WIDGET: {
                        const typeIdx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const widgetType = getConstantString(this.module, typeIdx);
                        const id = `w${this.widgetIdCounter++}`;
                        this.stack.push({
                            type: 'widget',
                            id,
                            widgetType,
                            props: new Map(),
                            children: [],
                            handlers: new Map(),
                        });
                        break;
                    }

                    case Opcode.GUI_SET_PROP: {
                        const keyIdx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const key = getConstantString(this.module, keyIdx);
                        const value = this.stack.pop()!;
                        const widget = this.stack[this.stack.length - 1];
                        if (widget.type !== 'widget') throw new Error('Expected widget');
                        widget.props.set(key, value);
                        break;
                    }

                    case Opcode.GUI_ADD_CHILD: {
                        const child = this.stack.pop()!;
                        const widget = this.stack[this.stack.length - 1];
                        if (widget.type !== 'widget') throw new Error('Expected widget');
                        if (child.type !== 'widget') throw new Error('Expected widget child');
                        widget.children.push(child);
                        break;
                    }

                    case Opcode.GUI_SET_HANDLER: {
                        const eventIdx = this.readU16(code, baseOffset + frame.ip);
                        frame.ip += 2;
                        const eventType = getConstantString(this.module, eventIdx);
                        const handler = this.stack.pop()!;
                        const widget = this.stack[this.stack.length - 1];
                        if (widget.type !== 'widget') throw new Error('Expected widget');
                        widget.handlers.set(eventType, handler);
                        break;
                    }

                    case Opcode.GUI_COMMIT_ROOT: {
                        const widget = this.stack.pop()!;
                        if (widget.type !== 'widget') throw new Error('Expected widget');
                        this.rootWidget = widget;
                        this.callbacks.onRender?.(widget);
                        break;
                    }

                    case Opcode.LOG_INFO: {
                        const value = this.stack.pop()!;
                        const message = valueToString(value);
                        console.log('[AFNS]', message);
                        this.callbacks.onLog?.(message);
                        break;
                    }

                    case Opcode.ADD: this.binaryOp((a, b) => a + b); break;
                    case Opcode.SUB: this.binaryOp((a, b) => a - b); break;
                    case Opcode.MUL: this.binaryOp((a, b) => a * b); break;
                    case Opcode.DIV: this.binaryOp((a, b) => a / b); break;
                    case Opcode.MOD: this.binaryOp((a, b) => a % b); break;
                    case Opcode.EQ: this.compareOp((a, b) => a === b); break;
                    case Opcode.NE: this.compareOp((a, b) => a !== b); break;
                    case Opcode.LT: this.compareOp((a, b) => a < b); break;
                    case Opcode.LE: this.compareOp((a, b) => a <= b); break;
                    case Opcode.GT: this.compareOp((a, b) => a > b); break;
                    case Opcode.GE: this.compareOp((a, b) => a >= b); break;

                    case Opcode.AND: {
                        const b = this.stack.pop()!;
                        const a = this.stack.pop()!;
                        this.stack.push({ type: 'bool', value: isTruthy(a) && isTruthy(b) });
                        break;
                    }

                    case Opcode.OR: {
                        const b = this.stack.pop()!;
                        const a = this.stack.pop()!;
                        this.stack.push({ type: 'bool', value: isTruthy(a) || isTruthy(b) });
                        break;
                    }

                    case Opcode.NOT: {
                        const v = this.stack.pop()!;
                        this.stack.push({ type: 'bool', value: !isTruthy(v) });
                        break;
                    }

                    case Opcode.NEG: {
                        const v = this.stack.pop()!;
                        if (v.type === 'int') {
                            this.stack.push({ type: 'int', value: -v.value });
                        } else if (v.type === 'float') {
                            this.stack.push({ type: 'float', value: -v.value });
                        } else {
                            throw new Error('Cannot negate non-number');
                        }
                        break;
                    }

                    case Opcode.DUP:
                        this.stack.push(this.stack[this.stack.length - 1]);
                        break;

                    case Opcode.POP:
                        this.stack.pop();
                        break;

                    case Opcode.CONCAT: {
                        const b = this.stack.pop()!;
                        const a = this.stack.pop()!;
                        this.stack.push({ type: 'string', value: valueToString(a) + valueToString(b) });
                        break;
                    }

                    case Opcode.STATE_CREATE: {
                        const initial = this.stack.pop()!;
                        const id = this.stateCounter++;
                        this.states.set(id, initial);
                        this.stack.push({ type: 'state', id, value: initial });
                        break;
                    }

                    case Opcode.STATE_GET: {
                        const state = this.stack.pop()!;
                        if (state.type !== 'state') throw new Error('Expected state');
                        this.stack.push(this.states.get(state.id) || NULL_VALUE);
                        break;
                    }

                    case Opcode.STATE_SET: {
                        const value = this.stack.pop()!;
                        const state = this.stack.pop()!;
                        if (state.type !== 'state') throw new Error('Expected state');
                        this.states.set(state.id, value);
                        this.scheduleRebuild();
                        break;
                    }

                    default:
                        throw new Error(`Unknown opcode: ${opcodeName(op)}`);
                }
            }

            // Implicit return
            if (this.frames[this.frames.length - 1] === frame) {
                this.stack.length = frame.bp;
                this.frames.pop();
            }
        }
    }

    private readU16(code: Uint8Array, offset: number): number {
        return code[offset] | (code[offset + 1] << 8);
    }

    private readI16(code: Uint8Array, offset: number): number {
        const u = this.readU16(code, offset);
        return u > 0x7fff ? u - 0x10000 : u;
    }

    private constantToValue(c: Constant): Value {
        switch (c.tag) {
            case 'utf8': return { type: 'string', value: c.value };
            case 'int64': return { type: 'int', value: c.value };
            case 'float64': return { type: 'float', value: c.value };
            case 'bool': return { type: 'bool', value: c.value };
            case 'null': return NULL_VALUE;
        }
    }

    private binaryOp(op: (a: number | bigint, b: number | bigint) => number | bigint): void {
        const b = this.stack.pop()!;
        const a = this.stack.pop()!;

        if (a.type === 'int' && b.type === 'int') {
            this.stack.push({ type: 'int', value: op(a.value, b.value) as bigint });
        } else if (a.type === 'float' && b.type === 'float') {
            this.stack.push({ type: 'float', value: op(a.value, b.value) as number });
        } else if ((a.type === 'int' || a.type === 'float') && (b.type === 'int' || b.type === 'float')) {
            const aNum = a.type === 'int' ? Number(a.value) : a.value;
            const bNum = b.type === 'int' ? Number(b.value) : b.value;
            this.stack.push({ type: 'float', value: op(aNum, bNum) as number });
        } else {
            throw new Error(`Cannot perform binary op on ${a.type} and ${b.type}`);
        }
    }

    private compareOp(op: (a: unknown, b: unknown) => boolean): void {
        const b = this.stack.pop()!;
        const a = this.stack.pop()!;

        let aVal: unknown, bVal: unknown;

        if (a.type === 'int') aVal = a.value;
        else if (a.type === 'float') aVal = a.value;
        else if (a.type === 'string') aVal = a.value;
        else if (a.type === 'bool') aVal = a.value;
        else aVal = null;

        if (b.type === 'int') bVal = b.value;
        else if (b.type === 'float') bVal = b.value;
        else if (b.type === 'string') bVal = b.value;
        else if (b.type === 'bool') bVal = b.value;
        else bVal = null;

        this.stack.push({ type: 'bool', value: op(aVal, bVal) });
    }
}
