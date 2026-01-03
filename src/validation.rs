use crate::ast::*;
use crate::span::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub span: Span,
}

#[derive(Default)]
struct Scope {
    names: HashMap<String, Span>,
}

pub fn validate_file(file: &File) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let mut aliases: HashMap<&str, Span> = HashMap::new();
    for import in &file.imports {
        if let Some(alias) = import.alias.as_deref() {
            if let Some(prev_span) = aliases.get(alias) {
                errors.push(ValidationError {
                    message: format!("Duplicate import alias `{alias}`"),
                    span: import.span.merge(*prev_span),
                });
            } else {
                aliases.insert(alias, import.span);
            }
        }
    }
    let mut scopes: Vec<Scope> = vec![Scope::default()];
    for item in &file.items {
        validate_item(item, &mut scopes, &mut errors);
    }
    errors
}

fn validate_item(item: &Item, scopes: &mut Vec<Scope>, errors: &mut Vec<ValidationError>) {
    match item {
        Item::Function(func) => {
            validate_params_not_self(func, errors);
            validate_block(&func.body, scopes, 0, func.signature.is_async, errors)
        }
        Item::Struct(_) | Item::Enum(_) | Item::Trait(_) | Item::ExternFunction(_) => {}
        Item::Impl(imp) => {
            for method in &imp.methods {
                validate_impl_method_params(imp, method, errors);
                validate_block(&method.body, scopes, 0, method.signature.is_async, errors);
            }
        }
    }
}

fn validate_block(
    block: &Block,
    scopes: &mut Vec<Scope>,
    loop_depth: usize,
    in_async: bool,
    errors: &mut Vec<ValidationError>,
) {
    scopes.push(Scope::default());
    for stmt in &block.statements {
        validate_stmt(stmt, scopes, loop_depth, in_async, errors);
    }
    scopes.pop();
}

fn validate_stmt(
    stmt: &Stmt,
    scopes: &mut Vec<Scope>,
    loop_depth: usize,
    in_async: bool,
    errors: &mut Vec<ValidationError>,
) {
    match stmt {
        Stmt::VarDecl(decl) => {
            if scopes
                .last()
                .and_then(|s| s.names.get(&decl.name))
                .is_some()
            {
                errors.push(ValidationError {
                    message: format!("Duplicate binding `{}` in the same scope", decl.name),
                    span: decl.span,
                });
            } else if let Some(scope) = scopes.last_mut() {
                scope.names.insert(decl.name.clone(), decl.span);
            }
            validate_expr(&decl.value, scopes, loop_depth, in_async, errors)
        }
        Stmt::Expr(expr) => validate_expr(expr, scopes, loop_depth, in_async, errors),
        Stmt::Return { value, .. } => {
            if let Some(expr) = value {
                validate_expr(expr, scopes, loop_depth, in_async, errors);
            }
        }
        Stmt::If(stmt) => {
            validate_expr(&stmt.condition, scopes, loop_depth, in_async, errors);
            validate_block(&stmt.then_branch, scopes, loop_depth, in_async, errors);
            for (cond, block) in &stmt.else_if {
                validate_expr(cond, scopes, loop_depth, in_async, errors);
                validate_block(block, scopes, loop_depth, in_async, errors);
            }
            if let Some(block) = &stmt.else_branch {
                validate_block(block, scopes, loop_depth, in_async, errors);
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            validate_expr(condition, scopes, loop_depth, in_async, errors);
            validate_block(body, scopes, loop_depth + 1, in_async, errors);
        }
        Stmt::For { iterable, body, .. } => {
            validate_expr(iterable, scopes, loop_depth, in_async, errors);
            validate_block(body, scopes, loop_depth + 1, in_async, errors);
        }
        Stmt::Switch(stmt) => {
            if stmt.arms.is_empty() {
                errors.push(ValidationError {
                    message: "Switch must have at least one arm".to_string(),
                    span: stmt.span,
                });
            }
            validate_expr(&stmt.expr, scopes, loop_depth, in_async, errors);
            for arm in &stmt.arms {
                scopes.push(Scope::default());
                bind_pattern(&arm.pattern, scopes, errors);
                validate_expr(&arm.expr, scopes, loop_depth, in_async, errors);
                scopes.pop();
            }
        }
        Stmt::Try(stmt) => {
            validate_block(&stmt.try_block, scopes, loop_depth, in_async, errors);
            scopes.push(Scope::default());
            if let Some(binding) = &stmt.catch_binding {
                if let Some(current) = scopes.last_mut() {
                    current.names.insert(binding.clone(), stmt.span);
                }
            }
            validate_block(&stmt.catch_block, scopes, loop_depth, in_async, errors);
            scopes.pop();
        }
        Stmt::Block(block) => validate_block(block, scopes, loop_depth, in_async, errors),
        Stmt::Unsafe { body, .. } => validate_block(body, scopes, loop_depth, in_async, errors),
        Stmt::Assembly(_) => {}
        Stmt::Break(span) => {
            if loop_depth == 0 {
                errors.push(ValidationError {
                    message: "break used outside of a loop".to_string(),
                    span: *span,
                });
            }
        }
        Stmt::Continue(span) => {
            if loop_depth == 0 {
                errors.push(ValidationError {
                    message: "continue used outside of a loop".to_string(),
                    span: *span,
                });
            }
        }
    }
}

fn bind_pattern(pattern: &Pattern, scopes: &mut Vec<Scope>, errors: &mut Vec<ValidationError>) {
    if let Some(scope) = scopes.last_mut() {
        match pattern {
            Pattern::Binding { name, span } => {
                scope.names.insert(name.clone(), *span);
            }
            Pattern::Enum { bindings, span, .. } => {
                for b in bindings {
                    scope.names.insert(b.clone(), *span);
                }
            }
            _ => {}
        }
    }
}

fn validate_expr(
    expr: &Expr,
    scopes: &mut Vec<Scope>,
    loop_depth: usize,
    in_async: bool,
    errors: &mut Vec<ValidationError>,
) {
    match expr {
        Expr::Literal(_) | Expr::Identifier { .. } => {}
        Expr::Access { base, .. } => validate_expr(base, scopes, loop_depth, in_async, errors),
        Expr::Call { callee, args, .. } => {
            validate_expr(callee, scopes, loop_depth, in_async, errors);
            for arg in args {
                validate_expr(arg, scopes, loop_depth, in_async, errors);
            }
        }
        Expr::Await { expr, span } => {
            if !in_async {
                errors.push(ValidationError {
                    message: "`await` is only allowed inside `async` functions or blocks".to_string(),
                    span: *span,
                });
            }
            validate_expr(expr, scopes, loop_depth, in_async, errors)
        }
        Expr::Unary { expr, .. } => validate_expr(expr, scopes, loop_depth, in_async, errors),
        Expr::Binary { left, right, .. } => {
            validate_expr(left, scopes, loop_depth, in_async, errors);
            validate_expr(right, scopes, loop_depth, in_async, errors);
        }
        Expr::Assignment { target, value, .. } => {
            validate_expr(target, scopes, loop_depth, in_async, errors);
            validate_expr(value, scopes, loop_depth, in_async, errors);
        }
        Expr::StructLiteral { fields, .. } => {
            for field in fields {
                validate_expr(&field.expr, scopes, loop_depth, in_async, errors);
            }
        }
        Expr::ArrayLiteral { elements, .. } => {
            for elem in elements {
                validate_expr(elem, scopes, loop_depth, in_async, errors);
            }
        }
        Expr::TupleLiteral { elements, .. } => {
            for elem in elements {
                validate_expr(elem, scopes, loop_depth, in_async, errors);
            }
        }
        Expr::Cast { expr, .. } => {
            validate_expr(expr, scopes, loop_depth, in_async, errors);
        }
        Expr::Block(block) => validate_block(block, scopes, loop_depth, in_async, errors),
        Expr::If(stmt) => {
            validate_expr(&stmt.condition, scopes, loop_depth, in_async, errors);
            validate_block(&stmt.then_branch, scopes, loop_depth, in_async, errors);
            for (cond, block) in &stmt.else_if {
                validate_expr(cond, scopes, loop_depth, in_async, errors);
                validate_block(block, scopes, loop_depth, in_async, errors);
            }
            if let Some(block) = &stmt.else_branch {
                validate_block(block, scopes, loop_depth, in_async, errors);
            }
        }
        Expr::Try { expr, .. } => validate_expr(expr, scopes, loop_depth, in_async, errors),
        Expr::Lambda(lambda) => {
            // Lambdas inherit async context? Or simple lambdas are sync?
            // "closure syntax implementation details not fully specified, assuming sync for now unless 'async' keyword added to lambda syntax later."
            // For now, let's assume lambdas are synchronous unless specified otherwise.
            // But wait, if I am in an async function, I can await inside a closure?
            // Generally closures capture environment.
            // If the user hasn't implemented async closures yet, then `in_async` should probably be false for lambdas, or we need to check lambda properties.
            // Looking at AST, Lambda struct has `params` and `body`. No `is_async`.
            // So lambdas are synchronous.
            validate_block(&lambda.body, scopes, 0, false, errors)
        },
        Expr::Index { base, index, .. } => {
            validate_expr(base, scopes, loop_depth, in_async, errors);
            validate_expr(index, scopes, loop_depth, in_async, errors);
        }
        Expr::MethodCall { object, args, .. } => {
            validate_expr(object, scopes, loop_depth, in_async, errors);
            for arg in args {
                validate_expr(arg, scopes, loop_depth, in_async, errors);
            }
        }
        Expr::Check(check) => {
            if let Some(target) = &check.target {
                validate_expr(target, scopes, loop_depth, in_async, errors);
            }
            for arm in &check.arms {
                match &arm.pattern {
                    CheckPattern::Literal(lit) => {
                        validate_expr(&Expr::Literal(lit.clone()), scopes, loop_depth, in_async, errors)
                    }
                    CheckPattern::Guard(expr) => validate_expr(expr, scopes, loop_depth, in_async, errors),
                    CheckPattern::Wildcard { .. } => {}
                }
                validate_expr(&arm.expr, scopes, loop_depth, in_async, errors);
            }
        }
    }
}

fn validate_impl_method_params(
    imp: &ImplBlock,
    method: &Function,
    errors: &mut Vec<ValidationError>,
) {
    if method.signature.params.is_empty() {
        errors.push(ValidationError {
            message: format!(
                "Method `{}` must declare `self` or `self_mut` as the first parameter",
                method.signature.name
            ),
            span: method.signature.span,
        });
        return;
    }
    let first = &method.signature.params[0];
    if first.name != "self" && first.name != "self_mut" {
        errors.push(ValidationError {
            message: format!(
                "Method `{}` must start with `self` or `self_mut` parameter",
                method.signature.name
            ),
            span: first.span,
        });
    }
    if let TypeExpr::Named(named) = &first.ty {
        if let Some(seg) = named.segments.first() {
            let target_name = type_expr_to_name(&imp.target);
            if let Some(target_name) = target_name {
                if seg.name != target_name {
                    errors.push(ValidationError {
                        message: format!(
                            "`{}` receiver must match impl target type `{}`",
                            first.name, target_name
                        ),
                        span: first.span,
                    });
                }
            }
        }
    }
    for param in method.signature.params.iter().skip(1) {
        if param.name == "self" || param.name == "self_mut" {
            errors.push(ValidationError {
                message: format!(
                    "`{}` is reserved for the first parameter in methods",
                    param.name
                ),
                span: param.span,
            });
        }
    }
}

fn validate_params_not_self(func: &Function, errors: &mut Vec<ValidationError>) {
    for param in &func.signature.params {
        if param.name == "self" || param.name == "self_mut" {
            errors.push(ValidationError {
                message: format!(
                    "`{}` is only allowed as the first parameter of methods inside impl blocks",
                    param.name
                ),
                span: param.span,
            });
        }
    }
}

fn type_expr_to_name(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Named(named) => named.segments.first().map(|s| s.name.clone()),
        _ => None,
    }
}
