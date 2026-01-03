use std::collections::HashMap;

use crate::ast::{
    BinaryOp, Block, Expr, File, Function, IfStmt, Item, Literal, Stmt, VarDecl, VarKind,
};

use super::{instr::CmpOp, IrBuilder, IrInstr, IrModule, IrTerm, IrType};

pub fn build_ir(ast: &File) -> IrModule {
    let mut builder = IrBuilder::new();
    if let Some(func) = find_function(ast, "apex") {
        lower_function(&mut builder, func);
    } else {
        let func_id = builder.new_function("apex", Vec::new(), IrType::Void);
        let block = builder.new_block(func_id);
        builder.set_term(func_id, block, IrTerm::Ret { value: None });
    }
    builder.finish()
}

pub fn format_ir(module: &IrModule) -> String {
    format!("{}", module)
}

fn find_function<'a>(ast: &'a File, name: &str) -> Option<&'a Function> {
    ast.items.iter().find_map(|item| match item {
        Item::Function(func) if func.signature.name == name => Some(func),
        _ => None,
    })
}

fn lower_function(builder: &mut IrBuilder, func: &Function) {
    let func_id = builder.new_function(func.signature.name.clone(), Vec::new(), IrType::Void);
    let entry = builder.new_block(func_id);
    let mut ctx = FnLower::new(builder, func_id, entry);
    ctx.lower_block(&func.body);
    ctx.finish();
}

#[derive(Clone)]
enum Binding {
    Value { value: u32, ty: IrType },
}

#[derive(Clone, Copy)]
struct LoopContext {
    break_target: u32,
    continue_target: u32,
}

struct BranchResult {
    env: HashMap<String, Binding>,
    reaches_merge: bool,
    exit_block: Option<u32>,
    produced: Option<u32>,
}

struct FnLower<'a> {
    builder: &'a mut IrBuilder,
    func_id: usize,
    block_id: u32,
    env: HashMap<String, Binding>,
    scope_stack: Vec<Vec<String>>,
    loop_stack: Vec<LoopContext>,
    terminated: bool,
}

impl<'a> FnLower<'a> {
    fn new(builder: &'a mut IrBuilder, func_id: usize, block_id: u32) -> Self {
        let mut slf = Self {
            builder,
            func_id,
            block_id,
            env: HashMap::new(),
            scope_stack: Vec::new(),
            loop_stack: Vec::new(),
            terminated: false,
        };
        slf.begin_scope();
        slf
    }

    fn finish(mut self) {
        self.end_scope();
        if !self.builder.block_has_term(self.func_id, self.block_id) {
            self.builder
                .set_term(self.func_id, self.block_id, IrTerm::Ret { value: None });
        }
    }

    fn begin_scope(&mut self) {
        self.scope_stack.push(Vec::new());
    }

    fn end_scope(&mut self) {
        if let Some(vars) = self.scope_stack.pop() {
            for name in vars {
                self.env.remove(&name);
            }
        }
    }

    fn declare_var(&mut self, name: String, binding: Binding) {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.push(name.clone());
        }
        self.env.insert(name, binding);
    }

    fn push_loop(&mut self, break_target: u32, continue_target: u32) {
        self.loop_stack.push(LoopContext {
            break_target,
            continue_target,
        });
    }

    fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    fn lower_block(&mut self, block: &Block) {
        self.begin_scope();
        for stmt in &block.statements {
            if self.terminated {
                break;
            }
            self.lower_stmt(stmt);
        }
        self.end_scope();
    }

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            Stmt::Return { value, .. } => {
                let val = value.as_ref().and_then(|expr| self.lower_expr(expr));
                self.builder
                    .set_term(self.func_id, self.block_id, IrTerm::Ret { value: val });
                self.terminated = true;
            }
            Stmt::VarDecl(var) => {
                self.lower_var_decl(var);
            }
            Stmt::If(if_stmt) => {
                self.lower_if(if_stmt);
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.lower_while(condition, body);
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.lower_for(var, iterable, body);
            }
            Stmt::Block(block) => {
                self.lower_block(block);
            }
            Stmt::Break(_) => {
                self.lower_loop_control(true);
            }
            Stmt::Continue(_) => {
                self.lower_loop_control(false);
            }
            _ => {}
        }
    }

    fn lower_var_decl(&mut self, decl: &VarDecl) {
        let init_val = self
            .lower_expr(&decl.value)
            .unwrap_or_else(|| self.emit_int(0));
        let ty = decl
            .ty
            .as_ref()
            .map(ir_type_from_hint)
            .unwrap_or(IrType::I32);
        match decl.kind {
            VarKind::Let => {
                self.declare_var(
                    decl.name.clone(),
                    Binding::Value {
                        value: init_val,
                        ty,
                    },
                );
            }
            VarKind::Var => {
                self.declare_var(
                    decl.name.clone(),
                    Binding::Value {
                        value: init_val,
                        ty,
                    },
                );
            }
            VarKind::Const => {
                self.declare_var(
                    decl.name.clone(),
                    Binding::Value {
                        value: init_val,
                        ty,
                    },
                );
            }
        }
    }

    fn lower_if(&mut self, stmt: &IfStmt) {
        let cond = self
            .lower_expr(&stmt.condition)
            .unwrap_or_else(|| self.emit_bool(false));
        let current_block = self.block_id;
        let then_block = self.builder.new_block(self.func_id);
        let merge_block = self.builder.new_block(self.func_id);
        let has_else = !stmt.else_if.is_empty() || stmt.else_branch.is_some();
        let else_block = if has_else {
            self.builder.new_block(self.func_id)
        } else {
            merge_block
        };

        self.builder.set_term(
            self.func_id,
            current_block,
            IrTerm::CondBr {
                cond,
                then_b: then_block,
                else_b: if has_else { else_block } else { merge_block },
            },
        );
        self.terminated = true;

        let pre_env = self.env.clone();
        let then_result = self.lower_branch(then_block, &stmt.then_branch, merge_block);
        let else_result = if has_else {
            if let Some(block) = build_else_block(stmt) {
                self.lower_branch(else_block, &block, merge_block)
            } else {
                BranchResult {
                    env: pre_env.clone(),
                    reaches_merge: true,
                    exit_block: Some(current_block),
                    produced: None,
                }
            }
        } else {
            BranchResult {
                env: pre_env.clone(),
                reaches_merge: true,
                exit_block: Some(current_block),
                produced: None,
            }
        };

        self.block_id = merge_block;
        self.terminated = false;
        self.merge_envs(&pre_env, &then_result, &else_result, merge_block);
    }

    fn lower_branch(&mut self, block_id: u32, block: &Block, merge_target: u32) -> BranchResult {
        let saved_block = self.block_id;
        let saved_env = self.env.clone();
        let saved_terminated = self.terminated;

        self.block_id = block_id;
        self.terminated = false;
        self.lower_block(block);
        let reaches_merge = if let Some(term) = self.builder.block_term(self.func_id, block_id) {
            matches!(term, IrTerm::Br { target } if target == merge_target)
        } else {
            self.builder.set_term(
                self.func_id,
                block_id,
                IrTerm::Br {
                    target: merge_target,
                },
            );
            true
        };
        let branch_env = self.env.clone();

        self.block_id = saved_block;
        self.env = saved_env;
        self.terminated = saved_terminated;

        BranchResult {
            env: branch_env,
            reaches_merge,
            exit_block: if reaches_merge { Some(block_id) } else { None },
            produced: None,
        }
    }

    fn merge_envs(
        &mut self,
        pre_env: &HashMap<String, Binding>,
        then_branch: &BranchResult,
        else_branch: &BranchResult,
        merge_block: u32,
    ) {
        let mut merged = pre_env.clone();
        for (name, base_binding) in pre_env {
            if let Binding::Value {
                value: base_val,
                ty,
            } = base_binding
            {
                let then_val = match then_branch.env.get(name) {
                    Some(Binding::Value { value, .. }) => *value,
                    _ => *base_val,
                };
                let else_val = match else_branch.env.get(name) {
                    Some(Binding::Value { value, .. }) => *value,
                    _ => *base_val,
                };
                match (then_branch.reaches_merge, else_branch.reaches_merge) {
                    (true, true) => {
                        if then_val != else_val {
                            let dst = self.builder.next_value(self.func_id);
                            let mut incomings = Vec::new();
                            if let Some(block) = then_branch.exit_block {
                                incomings.push((block, then_val));
                            }
                            if let Some(block) = else_branch.exit_block {
                                incomings.push((block, else_val));
                            }
                            if incomings.len() == 1 {
                                merged.insert(
                                    name.clone(),
                                    Binding::Value {
                                        value: incomings[0].1,
                                        ty: ty.clone(),
                                    },
                                );
                            } else {
                                self.builder.emit(
                                    self.func_id,
                                    merge_block,
                                    IrInstr::Phi { dst, incomings },
                                );
                                merged.insert(
                                    name.clone(),
                                    Binding::Value {
                                        value: dst,
                                        ty: ty.clone(),
                                    },
                                );
                            }
                        } else {
                            merged.insert(
                                name.clone(),
                                Binding::Value {
                                    value: then_val,
                                    ty: ty.clone(),
                                },
                            );
                        }
                    }
                    (true, false) => {
                        merged.insert(
                            name.clone(),
                            Binding::Value {
                                value: then_val,
                                ty: ty.clone(),
                            },
                        );
                    }
                    (false, true) => {
                        merged.insert(
                            name.clone(),
                            Binding::Value {
                                value: else_val,
                                ty: ty.clone(),
                            },
                        );
                    }
                    (false, false) => {
                        merged.insert(
                            name.clone(),
                            Binding::Value {
                                value: *base_val,
                                ty: ty.clone(),
                            },
                        );
                    }
                }
            }
        }
        self.env = merged;
    }

    fn lower_if_expr(&mut self, stmt: &IfStmt) -> Option<u32> {
        let cond = self
            .lower_expr(&stmt.condition)
            .unwrap_or_else(|| self.emit_bool(false));
        let current_block = self.block_id;
        let then_block = self.builder.new_block(self.func_id);
        let merge_block = self.builder.new_block(self.func_id);
        let has_else = !stmt.else_if.is_empty() || stmt.else_branch.is_some();
        let else_block = if has_else {
            self.builder.new_block(self.func_id)
        } else {
            merge_block
        };

        self.builder.set_term(
            self.func_id,
            current_block,
            IrTerm::CondBr {
                cond,
                then_b: then_block,
                else_b: if has_else { else_block } else { merge_block },
            },
        );
        self.terminated = true;

        let pre_env = self.env.clone();
        let then_result = self.lower_branch_value(then_block, &stmt.then_branch, merge_block);
        let else_result = if has_else {
            if let Some(block) = build_else_block(stmt) {
                self.lower_branch_value(else_block, &block, merge_block)
            } else {
                BranchResult {
                    env: pre_env.clone(),
                    reaches_merge: true,
                    exit_block: Some(current_block),
                    produced: None,
                }
            }
        } else {
            BranchResult {
                env: pre_env.clone(),
                reaches_merge: true,
                exit_block: Some(current_block),
                produced: None,
            }
        };

        self.block_id = merge_block;
        self.terminated = false;
        self.merge_envs(&pre_env, &then_result, &else_result, merge_block);

        let mut incomings = Vec::new();
        if then_result.reaches_merge {
            if let Some(val) = then_result.produced {
                if let Some(b) = then_result.exit_block {
                    incomings.push((b, val));
                }
            }
        }
        if else_result.reaches_merge {
            if let Some(val) = else_result.produced {
                if let Some(b) = else_result.exit_block {
                    incomings.push((b, val));
                }
            }
        }
        match incomings.as_slice() {
            [] => None,
            [(_, value)] => Some(*value),
            _ => {
                let dst = self.builder.next_value(self.func_id);
                self.builder
                    .emit(self.func_id, merge_block, IrInstr::Phi { dst, incomings });
                Some(dst)
            }
        }
    }

    fn lower_branch_value(
        &mut self,
        block_id: u32,
        block: &Block,
        merge_target: u32,
    ) -> BranchResult {
        let saved_block = self.block_id;
        let saved_env = self.env.clone();
        let saved_terminated = self.terminated;

        self.block_id = block_id;
        self.terminated = false;
        let produced = self.lower_block_value(block);
        let reaches_merge = if let Some(term) = self.builder.block_term(self.func_id, block_id) {
            matches!(term, IrTerm::Br { target } if target == merge_target)
        } else {
            self.builder.set_term(
                self.func_id,
                block_id,
                IrTerm::Br {
                    target: merge_target,
                },
            );
            true
        };
        let branch_env = self.env.clone();

        self.block_id = saved_block;
        self.env = saved_env;
        self.terminated = saved_terminated;

        BranchResult {
            env: branch_env,
            reaches_merge,
            exit_block: if reaches_merge { Some(block_id) } else { None },
            produced,
        }
    }

    fn lower_block_value(&mut self, block: &Block) -> Option<u32> {
        self.begin_scope();
        let mut last = None;
        for stmt in &block.statements {
            if self.terminated {
                break;
            }
            match stmt {
                Stmt::Expr(expr) => {
                    last = self.lower_expr(expr);
                }
                _ => self.lower_stmt(stmt),
            }
        }
        self.end_scope();
        last
    }

    fn lower_while(&mut self, condition: &Expr, body: &Block) {
        let head = self.builder.new_block(self.func_id);
        let loop_body = self.builder.new_block(self.func_id);
        let exit = self.builder.new_block(self.func_id);

        let current = self.block_id;
        self.builder
            .set_term(self.func_id, current, IrTerm::Br { target: head });
        self.terminated = true;

        // header
        self.block_id = head;
        self.terminated = false;
        let cond_val = self
            .lower_expr(condition)
            .unwrap_or_else(|| self.emit_bool(false));
        self.builder.set_term(
            self.func_id,
            head,
            IrTerm::CondBr {
                cond: cond_val,
                then_b: loop_body,
                else_b: exit,
            },
        );

        // body
        self.block_id = loop_body;
        self.terminated = false;
        self.push_loop(exit, head);
        self.lower_block(body);
        self.pop_loop();
        if !self.builder.block_has_term(self.func_id, self.block_id) {
            self.builder
                .set_term(self.func_id, self.block_id, IrTerm::Br { target: head });
        }

        self.block_id = exit;
        self.terminated = false;
    }

    fn lower_for(&mut self, var: &str, iterable: &Expr, body: &Block) {
        if let Some((start_expr, end_expr)) = extract_range(iterable) {
            let start = self
                .lower_expr(start_expr)
                .unwrap_or_else(|| self.emit_int(0));
            let end = self
                .lower_expr(end_expr)
                .unwrap_or_else(|| self.emit_int(0));

            let head = self.builder.new_block(self.func_id);
            let loop_body = self.builder.new_block(self.func_id);
            let step = self.builder.new_block(self.func_id);
            let exit = self.builder.new_block(self.func_id);

            let pre_block = self.block_id;
            self.builder
                .set_term(self.func_id, pre_block, IrTerm::Br { target: head });
            self.terminated = true;

            self.block_id = head;
            self.terminated = false;
            let idx_val = self.builder.next_value(self.func_id);
            self.builder.emit(
                self.func_id,
                head,
                IrInstr::Phi {
                    dst: idx_val,
                    incomings: vec![(pre_block, start)],
                },
            );
            let cond = self.builder.next_value(self.func_id);
            self.builder.emit(
                self.func_id,
                head,
                IrInstr::Cmp {
                    dst: cond,
                    a: idx_val,
                    b: end,
                    cond: CmpOp::Lt,
                    ty: IrType::I32,
                },
            );
            self.builder.set_term(
                self.func_id,
                head,
                IrTerm::CondBr {
                    cond,
                    then_b: loop_body,
                    else_b: exit,
                },
            );

            // body
            self.block_id = loop_body;
            self.terminated = false;
            self.begin_scope();
            self.declare_var(
                var.to_string(),
                Binding::Value {
                    value: idx_val,
                    ty: IrType::I32,
                },
            );
            self.push_loop(exit, step);
            self.lower_block(body);
            self.pop_loop();
            if !self.builder.block_has_term(self.func_id, self.block_id) {
                self.builder
                    .set_term(self.func_id, self.block_id, IrTerm::Br { target: step });
            }
            self.end_scope();

            // step / continue block
            self.block_id = step;
            self.terminated = false;
            let next_idx = self.builder.next_value(self.func_id);
            let one = self.emit_int(1);
            self.builder.emit(
                self.func_id,
                self.block_id,
                IrInstr::Add {
                    dst: next_idx,
                    a: idx_val,
                    b: one,
                    ty: IrType::I32,
                },
            );
            self.builder
                .set_term(self.func_id, self.block_id, IrTerm::Br { target: head });

            // patch phi for back-edge
            if let Some(incomings) = self
                .builder
                .block_instrs_mut(self.func_id, head)
                .iter_mut()
                .find_map(|i| match i {
                    IrInstr::Phi { incomings, .. } => Some(incomings),
                    _ => None,
                })
            {
                incomings.push((step, next_idx));
            }

            self.block_id = exit;
            self.terminated = false;
        } else {
            // Fallback: do nothing to keep IR valid.
        }
    }

    fn lower_loop_control(&mut self, is_break: bool) {
        let target = match self.loop_stack.last() {
            Some(ctx) if is_break => ctx.break_target,
            Some(ctx) => ctx.continue_target,
            None => return,
        };
        if !self.builder.block_has_term(self.func_id, self.block_id) {
            self.builder
                .set_term(self.func_id, self.block_id, IrTerm::Br { target });
        }
        self.terminated = true;
    }

    fn lower_expr(&mut self, expr: &Expr) -> Option<u32> {
        match expr {
            Expr::Literal(Literal::Integer { value, .. }) => {
                let parsed_text = value.replace('_', "");
                let parsed = parsed_text.parse::<i128>().unwrap_or(0);
                let dst = self.builder.next_value(self.func_id);
                self.builder.emit(
                    self.func_id,
                    self.block_id,
                    IrInstr::LoadConstInt {
                        dst,
                        value: parsed,
                        ty: IrType::I32,
                    },
                );
                Some(dst)
            }
            Expr::Literal(Literal::Bool { value, .. }) => Some(self.emit_bool(*value)),
            Expr::Literal(Literal::String { value, .. }) => {
                let sid = self.builder.intern_string(value.clone());
                let dst = self.builder.next_value(self.func_id);
                self.builder.emit(
                    self.func_id,
                    self.block_id,
                    IrInstr::LoadConstStr { dst, sid },
                );
                Some(dst)
            }
            Expr::Identifier { name, .. } => self.read_binding(name),
            Expr::Binary {
                left, op, right, ..
            } => {
                if let BinaryOp::Range = op {
                    // range values are handled by for-loop lowering; treat as tuple (start,end)
                    let start = self.lower_expr(left)?;
                    let end = self.lower_expr(right)?;
                    let dst = self.builder.next_value(self.func_id);
                    self.builder.emit(
                        self.func_id,
                        self.block_id,
                        IrInstr::TupleInit {
                            dst,
                            items: vec![start, end],
                        },
                    );
                    return Some(dst);
                }
                let lhs = self.lower_expr(left)?;
                let rhs = self.lower_expr(right)?;
                match op {
                    BinaryOp::Add => {
                        let dst = self.builder.next_value(self.func_id);
                        Some(self.emit_bin_int(IrInstr::Add {
                            dst,
                            a: lhs,
                            b: rhs,
                            ty: IrType::I32,
                        }))
                    }
                    BinaryOp::Subtract => {
                        let dst = self.builder.next_value(self.func_id);
                        Some(self.emit_bin_int(IrInstr::Sub {
                            dst,
                            a: lhs,
                            b: rhs,
                            ty: IrType::I32,
                        }))
                    }
                    BinaryOp::Multiply => {
                        let dst = self.builder.next_value(self.func_id);
                        Some(self.emit_bin_int(IrInstr::Mul {
                            dst,
                            a: lhs,
                            b: rhs,
                            ty: IrType::I32,
                        }))
                    }
                    BinaryOp::Divide => {
                        let dst = self.builder.next_value(self.func_id);
                        Some(self.emit_bin_int(IrInstr::Div {
                            dst,
                            a: lhs,
                            b: rhs,
                            ty: IrType::I32,
                            signed: true,
                        }))
                    }
                    BinaryOp::Modulo => {
                        let dst = self.builder.next_value(self.func_id);
                        Some(self.emit_bin_int(IrInstr::Rem {
                            dst,
                            a: lhs,
                            b: rhs,
                            ty: IrType::I32,
                            signed: true,
                        }))
                    }
                    BinaryOp::Equal => Some(self.emit_cmp(lhs, rhs, CmpOp::Eq)),
                    BinaryOp::NotEqual => Some(self.emit_cmp(lhs, rhs, CmpOp::Ne)),
                    BinaryOp::Less => Some(self.emit_cmp(lhs, rhs, CmpOp::Lt)),
                    BinaryOp::LessEqual => Some(self.emit_cmp(lhs, rhs, CmpOp::Le)),
                    BinaryOp::Greater => Some(self.emit_cmp(lhs, rhs, CmpOp::Gt)),
                    BinaryOp::GreaterEqual => Some(self.emit_cmp(lhs, rhs, CmpOp::Ge)),
                    BinaryOp::LogicalAnd => {
                        let dst = self.builder.next_value(self.func_id);
                        self.builder.emit(
                            self.func_id,
                            self.block_id,
                            IrInstr::And {
                                dst,
                                a: lhs,
                                b: rhs,
                                ty: IrType::Bool,
                            },
                        );
                        Some(dst)
                    }
                    BinaryOp::LogicalOr => {
                        let dst = self.builder.next_value(self.func_id);
                        self.builder.emit(
                            self.func_id,
                            self.block_id,
                            IrInstr::Or {
                                dst,
                                a: lhs,
                                b: rhs,
                                ty: IrType::Bool,
                            },
                        );
                        Some(dst)
                    }
                    _ => None,
                }
            }
            Expr::Assignment { target, value, .. } => {
                if let Expr::Identifier { name, .. } = target.as_ref() {
                    let val = self.lower_expr(value)?;
                    self.assign_binding(name, val);
                    Some(val)
                } else {
                    None
                }
            }
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                if method == "info" && is_log_object(object) {
                    if let Some(first) = args.first() {
                        if let Expr::Literal(Literal::String { value, .. }) = first {
                            let sid = self.builder.intern_string(value.clone());
                            self.builder.emit(
                                self.func_id,
                                self.block_id,
                                IrInstr::PrintStr { sid },
                            );
                        }
                    }
                }
                None
            }
            Expr::If(stmt) => self.lower_if_expr(stmt),
            _ => None,
        }
    }

    fn emit_cmp(&mut self, a: u32, b: u32, cond: CmpOp) -> u32 {
        let dst = self.builder.next_value(self.func_id);
        self.builder.emit(
            self.func_id,
            self.block_id,
            IrInstr::Cmp {
                dst,
                a,
                b,
                cond,
                ty: IrType::I32,
            },
        );
        dst
    }

    fn emit_bin_int(&mut self, instr: IrInstr) -> u32 {
        let dst = instr.result().expect("binary op must produce value");
        self.builder.emit(self.func_id, self.block_id, instr);
        dst
    }

    fn emit_bool(&mut self, value: bool) -> u32 {
        let dst = self.builder.next_value(self.func_id);
        self.builder.emit(
            self.func_id,
            self.block_id,
            IrInstr::LoadConstBool { dst, value },
        );
        dst
    }

    fn emit_int(&mut self, value: i128) -> u32 {
        let dst = self.builder.next_value(self.func_id);
        self.builder.emit(
            self.func_id,
            self.block_id,
            IrInstr::LoadConstInt {
                dst,
                value,
                ty: IrType::I32,
            },
        );
        dst
    }

    fn read_binding(&mut self, name: &str) -> Option<u32> {
        match self.env.get(name)? {
            Binding::Value { value, .. } => Some(*value),
        }
    }

    fn assign_binding(&mut self, name: &str, value: u32) {
        if let Some(binding) = self.env.get_mut(name) {
            match binding {
                Binding::Value { value: stored, .. } => {
                    *stored = value;
                }
            }
        }
    }
}

fn extract_range(expr: &Expr) -> Option<(&Expr, &Expr)> {
    if let Expr::Binary {
        left,
        op: BinaryOp::Range,
        right,
        ..
    } = expr
    {
        Some((left, right))
    } else {
        None
    }
}

fn build_else_block(stmt: &IfStmt) -> Option<Block> {
    if let Some(block) = &stmt.else_branch {
        Some(block.clone())
    } else if let Some((cond, block)) = stmt.else_if.first() {
        let nested = IfStmt {
            condition: cond.clone(),
            then_branch: block.clone(),
            else_if: stmt.else_if[1..].to_vec(),
            else_branch: stmt.else_branch.clone(),
            span: block.span,
        };
        Some(Block {
            statements: vec![Stmt::If(nested)],
            span: block.span,
        })
    } else {
        None
    }
}

fn is_log_object(expr: &Expr) -> bool {
    match expr {
        Expr::Identifier { name, .. } => name == "log",
        Expr::Access { base, member, .. } => member == "log" || is_log_object(base),
        _ => false,
    }
}

fn ir_type_from_hint(_ty: &crate::ast::TypeExpr) -> IrType {
    // Minimal mapping for now; extend as type checker lands.
    IrType::I32
}
