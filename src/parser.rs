use crate::ast::*;
use crate::diagnostics::{AfnsError, ParseError};
use crate::lexer;
use crate::span::Span;
use crate::token::{Keyword, Token, TokenKind};

#[allow(dead_code)]
pub fn parse(source: &str) -> Result<File, AfnsError> {
    let tokens = lexer::lex(source)?;
    parse_tokens(source, tokens)
}

pub fn parse_tokens(source: &str, tokens: Vec<Token>) -> Result<File, AfnsError> {
    let parser = Parser::new(source, tokens);
    parser.parse().map_err(AfnsError::from)
}

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    index: usize,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            index: 0,
            errors: Vec::new(),
        }
    }

    fn parse(mut self) -> Result<File, ParseError> {
        let mut imports = Vec::new();
        while self.check_keyword(Keyword::Import) {
            match self.parse_import() {
                Ok(import) => imports.push(import),
                Err(err) => {
                    self.record_error(err);
                    self.synchronize_top();
                }
            }
        }

        let mut items = Vec::new();
        while !self.is_at_end() {
            let before = self.index;
            match self.parse_top_item() {
                Ok(Some(item)) => items.push(item),
                Ok(None) => {}
                Err(err) => {
                    self.record_error(err);
                    self.synchronize_top();
                }
            }
            // Safety guard: ensure the parser always makes progress.
            // If no tokens were consumed, skip one token to avoid infinite loops.
            if self.index == before {
                if self.is_at_end() {
                    break;
                }
                self.advance();
            }
        }

        if let Some(err) = self.errors.first().cloned() {
            return Err(err);
        }

        let span = if let Some(first) = self
            .tokens
            .iter()
            .find(|t| !matches!(t.kind, TokenKind::Eof))
        {
            let last = self
                .tokens
                .iter()
                .rev()
                .find(|t| !matches!(t.kind, TokenKind::Eof))
                .unwrap_or(first);
            first.span.merge(last.span)
        } else {
            Span::new(0, 0, 1, 1)
        };

        Ok(File {
            imports,
            items,
            span,
        })
    }

    fn parse_import(&mut self) -> Result<Import, ParseError> {
        let start = self.expect_keyword(Keyword::Import)?.span;
        let (first, mut span) = self.expect_name("import path")?;
        let mut path = vec![first];
        while self.match_with(|k| matches!(k, TokenKind::ColonColon | TokenKind::Dot)) {
            let (segment, seg_span) = self.expect_name("import path segment")?;
            span = span.merge(seg_span);
            path.push(segment);
        }
        let alias = if self.match_keyword(Keyword::As) {
            let (alias, alias_span) = self.expect_plain_identifier("import alias")?;
            span = span.merge(alias_span);
            Some(alias)
        } else {
            None
        };
        self.expect_with("';'", |k| matches!(k, TokenKind::Semicolon))?;
        Ok(Import {
            path,
            alias,
            span: start.merge(span),
        })
    }

    fn parse_top_item(&mut self) -> Result<Option<Item>, ParseError> {
        if self.is_at_end() {
            return Ok(None);
        }
        while self.match_with(|k| matches!(k, TokenKind::RightBrace)) {
            if self.is_at_end() {
                return Ok(None);
            }
        }
        if self.is_at_end() {
            return Ok(None);
        }
        let attributes = self.parse_attributes()?;
        if self.is_at_end() {
            return Ok(None);
        }
        if self.check_keyword(Keyword::Struct) {
            return Ok(Some(Item::Struct(self.parse_struct(attributes)?)));
        }
        if self.check_keyword(Keyword::Enum) {
            return Ok(Some(Item::Enum(self.parse_enum(attributes)?)));
        }
        if self.check_keyword(Keyword::Trait) {
            return Ok(Some(Item::Trait(self.parse_trait(attributes)?)));
        }
        if self.check_keyword(Keyword::Impl) {
            return Ok(Some(Item::Impl(self.parse_impl(attributes)?)));
        }
        if self.check_keyword(Keyword::Extern) {
            return Ok(Some(Item::ExternFunction(self.parse_extern(attributes)?)));
        }
        if self.check_keyword(Keyword::Async) || self.check_keyword(Keyword::Fun) {
            return Ok(Some(Item::Function(self.parse_function(attributes)?)));
        }
        if self.check(|k| matches!(k, TokenKind::Eof)) {
            return Ok(None);
        }
        let token = self.peek().clone();
        Err(ParseError::UnexpectedToken {
            expected: "top level declaration",
            found: token.kind,
            span: token.span,
        })
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, ParseError> {
        let mut attrs = Vec::new();
        while self.match_with(|k| matches!(k, TokenKind::At)) {
            let start = self.prev().span;
            let (name, name_span) = self.expect_identifier("attribute name")?;
            let mut args = Vec::new();
            let mut span = start.merge(name_span);
            if self.match_with(|k| matches!(k, TokenKind::LeftParen)) {
                if !self.check(|k| matches!(k, TokenKind::RightParen)) {
                    loop {
                        let token = self.advance();
                        match token.kind.clone() {
                            TokenKind::StringLiteral(value) => {
                                args.push(AttributeArg::String {
                                    value,
                                    span: token.span,
                                });
                            }
                            _ => {
                                return Err(ParseError::UnexpectedToken {
                                    expected: "string literal inside attribute",
                                    found: token.kind,
                                    span: token.span,
                                })
                            }
                        }
                        if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                            continue;
                        }
                        break;
                    }
                }
                let end = self
                    .expect_with("')'", |k| matches!(k, TokenKind::RightParen))?
                    .span;
                span = span.merge(end);
            }
            attrs.push(Attribute { name, args, span });
        }
        Ok(attrs)
    }

    fn parse_struct(&mut self, attributes: Vec<Attribute>) -> Result<StructDef, ParseError> {
        let start = self.expect_keyword(Keyword::Struct)?.span;
        let (name, _) = self.expect_identifier("struct name")?;
        let type_params = self.parse_type_params()?;
        self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut fields = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RightBrace)) {
            let (field_name, field_span) = self.expect_identifier("struct field name")?;
            self.expect_with("'::'", |k| matches!(k, TokenKind::ColonColon))?;
            let ty = self.parse_type()?;
            let mut span = field_span.merge(ty.span());
            if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                span = span.merge(self.prev().span);
            }
            fields.push(StructField {
                name: field_name,
                ty,
                span,
            });
        }
        let end = self
            .expect_with("'}'", |k| matches!(k, TokenKind::RightBrace))?
            .span;
        Ok(StructDef {
            attributes,
            name,
            type_params,
            fields,
            span: start.merge(end),
        })
    }

    fn parse_enum(&mut self, attributes: Vec<Attribute>) -> Result<EnumDef, ParseError> {
        let start = self.expect_keyword(Keyword::Enum)?.span;
        let (name, _) = self.expect_identifier("enum name")?;
        let type_params = self.parse_type_params()?;
        self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut variants = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RightBrace)) {
            let (variant_name, variant_span) = self.expect_identifier("enum variant")?;
            let mut payload = Vec::new();
            let mut span = variant_span;
            if self.match_with(|k| matches!(k, TokenKind::LeftParen)) {
                if !self.check(|k| matches!(k, TokenKind::RightParen)) {
                    loop {
                        payload.push(self.parse_type()?);
                        if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                            continue;
                        }
                        break;
                    }
                }
                let close = self
                    .expect_with("')'", |k| matches!(k, TokenKind::RightParen))?
                    .span;
                span = span.merge(close);
            }
            if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                span = span.merge(self.prev().span);
            }
            variants.push(EnumVariant {
                name: variant_name,
                payload,
                span,
            });
        }
        let end = self
            .expect_with("'}'", |k| matches!(k, TokenKind::RightBrace))?
            .span;
        Ok(EnumDef {
            attributes,
            name,
            type_params,
            variants,
            span: start.merge(end),
        })
    }

    fn parse_trait(&mut self, attributes: Vec<Attribute>) -> Result<TraitDef, ParseError> {
        let start = self.expect_keyword(Keyword::Trait)?.span;
        let (name, _) = self.expect_identifier("trait name")?;
        let type_params = self.parse_type_params()?;
        self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut methods = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RightBrace)) {
            let sig = self.parse_function_signature()?;
            self.expect_with("';'", |k| matches!(k, TokenKind::Semicolon))?;
            methods.push(sig);
        }
        let end = self
            .expect_with("'}'", |k| matches!(k, TokenKind::RightBrace))?
            .span;
        Ok(TraitDef {
            attributes,
            name,
            type_params,
            methods,
            span: start.merge(end),
        })
    }

    fn parse_impl(&mut self, attributes: Vec<Attribute>) -> Result<ImplBlock, ParseError> {
        let start = self.expect_keyword(Keyword::Impl)?.span;
        let type_params = self.parse_type_params()?;
        let target_or_trait = self.parse_type()?;
        let (trait_type, target) = if self.match_keyword(Keyword::For) {
            let target = self.parse_type()?;
            (Some(target_or_trait), target)
        } else {
            (None, target_or_trait)
        };
        self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut methods = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RightBrace)) {
            let attrs = self.parse_attributes()?;
            methods.push(self.parse_function(attrs)?);
        }
        let end = self
            .expect_with("'}'", |k| matches!(k, TokenKind::RightBrace))?
            .span;
        Ok(ImplBlock {
            attributes,
            type_params,
            trait_type,
            target,
            methods,
            span: start.merge(end),
        })
    }

    fn parse_extern(&mut self, attributes: Vec<Attribute>) -> Result<ExternFunction, ParseError> {
        let start = self.expect_keyword(Keyword::Extern)?.span;
        let abi_token = self.expect_with("string literal ABI", |k| {
            matches!(k, TokenKind::StringLiteral(_))
        })?;
        let abi = if let TokenKind::StringLiteral(value) = abi_token.kind.clone() {
            value
        } else {
            unreachable!()
        };
        let signature = self.parse_function_signature()?;
        self.expect_with("';'", |k| matches!(k, TokenKind::Semicolon))?;
        let span = start.merge(signature.span);
        Ok(ExternFunction {
            attributes,
            abi,
            signature,
            span,
        })
    }

    fn parse_function(&mut self, attributes: Vec<Attribute>) -> Result<Function, ParseError> {
        let signature = self.parse_function_signature()?;
        let body = self.parse_block()?;
        Ok(Function {
            attributes,
            signature,
            body,
        })
    }

    fn parse_function_signature(&mut self) -> Result<FunctionSignature, ParseError> {
        let mut is_async = false;
        let mut start_span = self.peek().span;
        if self.match_keyword(Keyword::Async) {
            is_async = true;
            start_span = self.prev().span;
        }
        let fun_token = self.expect_keyword(Keyword::Fun)?;
        if !is_async {
            start_span = fun_token.span;
        }
        let (name, _) = self.expect_identifier("function name")?;
        let type_params = self.parse_type_params()?;
        self.expect_with("'('", |k| matches!(k, TokenKind::LeftParen))?;
        let mut params = Vec::new();
        if !self.check(|k| matches!(k, TokenKind::RightParen)) {
            loop {
                let (param_name, param_span) = self.expect_identifier("parameter name")?;
                self.expect_with("'::'", |k| matches!(k, TokenKind::ColonColon))?;
                let ty = self.parse_type()?;
                params.push(Param {
                    name: param_name,
                    span: param_span.merge(ty.span()),
                    ty,
                });
                if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                    continue;
                }
                break;
            }
        }
        let close_paren = self.expect_with("')'", |k| matches!(k, TokenKind::RightParen))?;
        let mut returns_async = false;
        let mut return_type = None;
        if self.match_with(|k| matches!(k, TokenKind::ThinArrow)) {
            if self.match_keyword(Keyword::Async) {
                returns_async = true;
            }
            return_type = Some(self.parse_type()?);
        }
        let end_span = return_type
            .as_ref()
            .map(|ty| ty.span())
            .unwrap_or(close_paren.span);
        let span = start_span.merge(end_span);
        Ok(FunctionSignature {
            name,
            is_async,
            returns_async,
            params,
            return_type,
            type_params,
            span,
        })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let open = self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut statements = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RightBrace)) {
            statements.push(self.parse_statement()?);
        }
        let close = self.expect_with("'}'", |k| matches!(k, TokenKind::RightBrace))?;
        Ok(Block {
            statements,
            span: open.span.merge(close.span),
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.check_keyword(Keyword::Let) || self.check_keyword(Keyword::Var) {
            self.parse_var_decl()
        } else if self.match_keyword(Keyword::Return) {
            let start = self.prev().span;
            let value = if !self.check(|k| matches!(k, TokenKind::Semicolon)) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let end = self
                .expect_with("';'", |k| matches!(k, TokenKind::Semicolon))?
                .span;
            Ok(Stmt::Return {
                value,
                span: start.merge(end),
            })
        } else if self.match_keyword(Keyword::If) {
            let start = self.prev().span;
            Ok(Stmt::If(self.parse_if(start)?))
        } else if self.match_keyword(Keyword::While) {
            let start = self.prev().span;
            let condition = self.parse_expression()?;
            let body = self.parse_block()?;
            let span = start.merge(body.span);
            Ok(Stmt::While {
                condition,
                body,
                span,
            })
        } else if self.match_keyword(Keyword::For) {
            let start = self.prev().span;
            let (name, name_span) = self.expect_identifier("for binding")?;
            self.expect_keyword(Keyword::In)?;
            let iterable = self.parse_expression()?;
            let body = self.parse_block()?;
            let span = start.merge(body.span).merge(name_span);
            Ok(Stmt::For {
                var: name,
                iterable,
                body,
                span,
            })
        } else if self.match_keyword(Keyword::Switch) {
            Ok(Stmt::Switch(self.parse_switch(self.prev().span)?))
        } else if self.match_keyword(Keyword::Try) {
            Ok(Stmt::Try(self.parse_try(self.prev().span)?))
        } else if self.match_keyword(Keyword::Unsafe) {
            let start = self.prev().span;
            let body = self.parse_block()?;
            let span = start.merge(body.span);
            Ok(Stmt::Unsafe { body, span })
        } else if self.match_keyword(Keyword::Assembly) {
            self.parse_assembly()
        } else if self.check(|k| matches!(k, TokenKind::LeftBrace)) {
            let block = self.parse_block()?;
            Ok(Stmt::Block(block))
        } else {
            let expr = self.parse_expression()?;
            self.expect_with("';'", |k| matches!(k, TokenKind::Semicolon))?;
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_var_decl(&mut self) -> Result<Stmt, ParseError> {
        let kind = if self.match_keyword(Keyword::Let) {
            VarKind::Let
        } else {
            self.expect_keyword(Keyword::Var)?;
            VarKind::Var
        };
        let (name, name_span) = self.expect_identifier("variable name")?;
        let mut span = name_span;
        let ty = if self.match_with(|k| matches!(k, TokenKind::ColonColon)) {
            let ty = self.parse_type()?;
            span = span.merge(ty.span());
            Some(ty)
        } else {
            None
        };
        self.expect_with("'='", |k| matches!(k, TokenKind::Equals))?;
        let value = self.parse_expression()?;
        span = span.merge(value.span());
        let end = self
            .expect_with("';'", |k| matches!(k, TokenKind::Semicolon))?
            .span;
        span = span.merge(end);
        Ok(Stmt::VarDecl(VarDecl {
            kind,
            name,
            ty,
            value,
            span,
        }))
    }

    fn parse_if(&mut self, start: Span) -> Result<IfStmt, ParseError> {
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;
        let mut else_if = Vec::new();
        let mut else_branch = None;
        while self.match_keyword(Keyword::Else) {
            if self.match_keyword(Keyword::If) {
                let cond = self.parse_expression()?;
                let block = self.parse_block()?;
                else_if.push((cond, block));
            } else {
                else_branch = Some(self.parse_block()?);
                break;
            }
        }
        let span = if let Some(else_block) = &else_branch {
            start.merge(else_block.span)
        } else if let Some((_, block)) = else_if.last() {
            start.merge(block.span)
        } else {
            start.merge(then_branch.span)
        };
        Ok(IfStmt {
            condition,
            then_branch,
            else_if,
            else_branch,
            span,
        })
    }

    fn parse_switch(&mut self, start: Span) -> Result<SwitchStmt, ParseError> {
        let expr = self.parse_expression()?;
        self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut arms = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RightBrace)) {
            let pattern = self.parse_pattern()?;
            self.expect_with("'->'", |k| matches!(k, TokenKind::ThinArrow))?;
            let value = self.parse_expression()?;
            let span = pattern.span().merge(value.span());
            arms.push(SwitchArm {
                pattern,
                expr: value,
                span,
            });
            if !self.match_with(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        let end = self
            .expect_with("'}'", |k| matches!(k, TokenKind::RightBrace))?
            .span;
        Ok(SwitchStmt {
            expr,
            arms,
            span: start.merge(end),
        })
    }

    fn parse_try(&mut self, start: Span) -> Result<TryCatch, ParseError> {
        let try_block = self.parse_block()?;
        self.expect_keyword(Keyword::Catch)?;
        let catch_binding = if self.match_with(|k| matches!(k, TokenKind::LeftParen)) {
            let (name, _) = self.expect_identifier("catch binding")?;
            self.expect_with("')'", |k| matches!(k, TokenKind::RightParen))?;
            Some(name)
        } else {
            None
        };
        let catch_block = self.parse_block()?;
        let span = start.merge(catch_block.span);
        Ok(TryCatch {
            try_block,
            catch_binding,
            catch_block,
            span,
        })
    }

    fn parse_assembly(&mut self) -> Result<Stmt, ParseError> {
        let start = self.prev().span;
        let open = self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut depth = 1;
        let mut end = open.span;
        while depth > 0 {
            let token = self.advance();
            match token.kind {
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace => {
                    depth -= 1;
                    end = token.span;
                }
                TokenKind::Eof => return Err(ParseError::UnbalancedBlock { span: open.span }),
                _ => {}
            }
        }
        let slice = self
            .source
            .get(open.span.start..end.end)
            .unwrap_or("")
            .to_string();
        Ok(Stmt::Assembly(AssemblyBlock {
            body: slice,
            span: start.merge(end),
        }))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let token = self.advance();
        match token.kind.clone() {
            TokenKind::Identifier(name) => {
                if name == "_" {
                    Ok(Pattern::Wildcard { span: token.span })
                } else {
                    let mut segments = vec![name.clone()];
                    let mut span = token.span;
                    while self.match_with(|k| matches!(k, TokenKind::ColonColon)) {
                        let (seg, seg_span) = self.expect_identifier("pattern path segment")?;
                        span = span.merge(seg_span);
                        segments.push(seg);
                    }
                    if self.match_with(|k| matches!(k, TokenKind::LeftParen)) {
                        let mut bindings = Vec::new();
                        if !self.check(|k| matches!(k, TokenKind::RightParen)) {
                            loop {
                                let (binding, _) = self.expect_identifier("pattern binding")?;
                                bindings.push(binding);
                                if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                                    continue;
                                }
                                break;
                            }
                        }
                        let close = self
                            .expect_with("')'", |k| matches!(k, TokenKind::RightParen))?
                            .span;
                        span = span.merge(close);
                        Ok(Pattern::Enum {
                            path: segments,
                            bindings,
                            span,
                        })
                    } else if segments.len() == 1 {
                        Ok(Pattern::Binding {
                            name,
                            span: token.span,
                        })
                    } else {
                        Ok(Pattern::Path { segments, span })
                    }
                }
            }
            TokenKind::IntegerLiteral(value) => Ok(Pattern::Literal(Literal::Integer {
                value,
                span: token.span,
            })),
            TokenKind::FloatLiteral(value) => Ok(Pattern::Literal(Literal::Float {
                value,
                span: token.span,
            })),
            TokenKind::StringLiteral(value) => Ok(Pattern::Literal(Literal::String {
                value,
                span: token.span,
            })),
            TokenKind::CharLiteral(value) => Ok(Pattern::Literal(Literal::Char {
                value,
                span: token.span,
            })),
            _ => Err(ParseError::UnexpectedToken {
                expected: "pattern",
                found: token.kind,
                span: token.span,
            }),
        }
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_or()?;
        if self.match_with(|k| matches!(k, TokenKind::Equals)) {
            let value = self.parse_assignment()?;
            let span = expr.span().merge(value.span());
            expr = Expr::Assignment {
                target: Box::new(expr),
                value: Box::new(value),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_logical_and()?;
        while self.match_with(|k| matches!(k, TokenKind::PipePipe)) {
            let right = self.parse_logical_and()?;
            let span = expr.span().merge(right.span());
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::LogicalOr,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;
        while self.match_with(|k| matches!(k, TokenKind::AmpersandAmpersand)) {
            let right = self.parse_equality()?;
            let span = expr.span().merge(right.span());
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::LogicalAnd,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        loop {
            if self.match_with(|k| matches!(k, TokenKind::EqualEqual)) {
                let right = self.parse_comparison()?;
                let span = expr.span().merge(right.span());
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Equal,
                    right: Box::new(right),
                    span,
                };
            } else if self.match_with(|k| matches!(k, TokenKind::BangEqual)) {
                let right = self.parse_comparison()?;
                let span = expr.span().merge(right.span());
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::NotEqual,
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;
        loop {
            let op = if self.match_with(|k| matches!(k, TokenKind::Less)) {
                Some(BinaryOp::Less)
            } else if self.match_with(|k| matches!(k, TokenKind::LessEqual)) {
                Some(BinaryOp::LessEqual)
            } else if self.match_with(|k| matches!(k, TokenKind::Greater)) {
                Some(BinaryOp::Greater)
            } else if self.match_with(|k| matches!(k, TokenKind::GreaterEqual)) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_term()?;
                let span = expr.span().merge(right.span());
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = if self.match_with(|k| matches!(k, TokenKind::Plus)) {
                Some(BinaryOp::Add)
            } else if self.match_with(|k| matches!(k, TokenKind::Minus)) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_factor()?;
                let span = expr.span().merge(right.span());
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.match_with(|k| matches!(k, TokenKind::Star)) {
                Some(BinaryOp::Multiply)
            } else if self.match_with(|k| matches!(k, TokenKind::Slash)) {
                Some(BinaryOp::Divide)
            } else if self.match_with(|k| matches!(k, TokenKind::Percent)) {
                Some(BinaryOp::Modulo)
            } else {
                None
            };
            if let Some(op) = op {
                let right = self.parse_unary()?;
                let span = expr.span().merge(right.span());
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_keyword(Keyword::Await) {
            let keyword_span = self.prev().span;
            let inner = self.parse_unary()?;
            let span = keyword_span.merge(inner.span());
            return Ok(Expr::Await {
                expr: Box::new(inner),
                span,
            });
        }
        if self.match_with(|k| matches!(k, TokenKind::Bang)) {
            let op_span = self.prev().span;
            let inner = self.parse_unary()?;
            let span = op_span.merge(inner.span());
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(inner),
                span,
            });
        }
        if self.match_with(|k| matches!(k, TokenKind::Minus)) {
            let op_span = self.prev().span;
            let inner = self.parse_unary()?;
            let span = op_span.merge(inner.span());
            return Ok(Expr::Unary {
                op: UnaryOp::Negate,
                expr: Box::new(inner),
                span,
            });
        }
        if self.match_with(|k| matches!(k, TokenKind::Ampersand)) {
            let op_span = self.prev().span;
            let inner = self.parse_unary()?;
            let span = op_span.merge(inner.span());
            return Ok(Expr::Unary {
                op: UnaryOp::Borrow,
                expr: Box::new(inner),
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_with(|k| matches!(k, TokenKind::LeftParen)) {
                let mut args = Vec::new();
                if !self.check(|k| matches!(k, TokenKind::RightParen)) {
                    loop {
                        args.push(self.parse_expression()?);
                        if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                            continue;
                        }
                        break;
                    }
                }
                let close = self.expect_with("')'", |k| matches!(k, TokenKind::RightParen))?;
                let span = expr.span().merge(close.span);
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
                continue;
            }
            if self.match_with(|k| matches!(k, TokenKind::LeftBracket)) {
                let index_expr = self.parse_expression()?;
                let close = self.expect_with("']'", |k| matches!(k, TokenKind::RightBracket))?;
                let span = expr.span().merge(close.span);
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index_expr),
                    span,
                };
                continue;
            }
            if self.match_with(|k| matches!(k, TokenKind::Dot | TokenKind::ColonColon)) {
                let op_token = self.prev().clone();
                let (member, member_span) = self.expect_identifier("member name")?;

                // Check if this is a method call (followed by parentheses)
                if self.check(|k| matches!(k, TokenKind::LeftParen)) {
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if !self.check(|k| matches!(k, TokenKind::RightParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                                continue;
                            }
                            break;
                        }
                    }
                    let close = self.expect_with("')'", |k| matches!(k, TokenKind::RightParen))?;
                    let span = expr.span().merge(close.span);
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method: member,
                        args,
                        span,
                    };
                } else {
                    // Regular member access
                    let span = expr.span().merge(member_span);
                    let op = if matches!(op_token.kind, TokenKind::Dot) {
                        AccessOperator::Dot
                    } else {
                        AccessOperator::Path
                    };
                    expr = Expr::Access {
                        base: Box::new(expr),
                        member,
                        op,
                        span,
                    };
                }
                continue;
            }
            if self.match_with(|k| matches!(k, TokenKind::Question)) {
                let span = expr.span().merge(self.prev().span);
                expr = Expr::Try {
                    expr: Box::new(expr),
                    span,
                };
                continue;
            }
            if self.check(|k| matches!(k, TokenKind::LeftBrace))
                && expr.is_path_like()
                && self.struct_literal_follows()
            {
                expr = self.parse_struct_literal(expr)?;
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().kind.clone() {
            TokenKind::IntegerLiteral(_) => {
                let token = self.advance();
                if let TokenKind::IntegerLiteral(value) = token.kind.clone() {
                    Ok(Expr::Literal(Literal::Integer {
                        value,
                        span: token.span,
                    }))
                } else {
                    unreachable!()
                }
            }
            TokenKind::FloatLiteral(_) => {
                let token = self.advance();
                if let TokenKind::FloatLiteral(value) = token.kind.clone() {
                    Ok(Expr::Literal(Literal::Float {
                        value,
                        span: token.span,
                    }))
                } else {
                    unreachable!()
                }
            }
            TokenKind::StringLiteral(_) => {
                let token = self.advance();
                if let TokenKind::StringLiteral(value) = token.kind.clone() {
                    Ok(Expr::Literal(Literal::String {
                        value,
                        span: token.span,
                    }))
                } else {
                    unreachable!()
                }
            }
            TokenKind::CharLiteral(_) => {
                let token = self.advance();
                if let TokenKind::CharLiteral(value) = token.kind.clone() {
                    Ok(Expr::Literal(Literal::Char {
                        value,
                        span: token.span,
                    }))
                } else {
                    unreachable!()
                }
            }
            TokenKind::Keyword(Keyword::True) => {
                let token = self.advance();
                Ok(Expr::Literal(Literal::Bool {
                    value: true,
                    span: token.span,
                }))
            }
            TokenKind::Keyword(Keyword::False) => {
                let token = self.advance();
                Ok(Expr::Literal(Literal::Bool {
                    value: false,
                    span: token.span,
                }))
            }
            TokenKind::Keyword(Keyword::Fun) => self.parse_lambda(),
            TokenKind::Keyword(Keyword::Async) => {
                if matches!(self.peek_kind_at(1), Some(TokenKind::Keyword(Keyword::Fun))) {
                    self.parse_lambda()
                } else {
                    let span = self.advance().span;
                    Ok(Expr::Identifier {
                        name: "async".to_string(),
                        span,
                    })
                }
            }
            TokenKind::Identifier(_) => {
                let token = self.advance();
                if let TokenKind::Identifier(name) = token.kind.clone() {
                    Ok(Expr::Identifier {
                        name,
                        span: token.span,
                    })
                } else {
                    unreachable!()
                }
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_with("')'", |k| matches!(k, TokenKind::RightParen))?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                let start = self.advance().span;
                let mut elements = Vec::new();
                if !self.check(|k| matches!(k, TokenKind::RightBracket)) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                            continue;
                        }
                        break;
                    }
                }
                let close = self.expect_with("']'", |k| matches!(k, TokenKind::RightBracket))?;
                Ok(Expr::ArrayLiteral {
                    elements,
                    span: start.merge(close.span),
                })
            }
            TokenKind::LeftBrace => {
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }
            _ => {
                let token = self.advance();
                Err(ParseError::UnexpectedToken {
                    expected: "expression",
                    found: token.kind,
                    span: token.span,
                })
            }
        }
    }

    fn parse_struct_literal(&mut self, path: Expr) -> Result<Expr, ParseError> {
        self.expect_with("'{'", |k| matches!(k, TokenKind::LeftBrace))?;
        let mut fields = Vec::new();
        while !self.check(|k| matches!(k, TokenKind::RightBrace)) {
            let (field_name, field_span) = self.expect_identifier("struct literal field")?;
            self.expect_with("':'", |k| matches!(k, TokenKind::Colon))?;
            let expr = self.parse_expression()?;
            let span = field_span.merge(expr.span());
            fields.push(StructLiteralField {
                name: field_name,
                expr,
                span,
            });
            if !self.match_with(|k| matches!(k, TokenKind::Comma)) {
                break;
            }
        }
        let close = self.expect_with("'}'", |k| matches!(k, TokenKind::RightBrace))?;
        let span = path.span().merge(close.span);
        Ok(Expr::StructLiteral {
            path: Box::new(path),
            fields,
            span,
        })
    }

    fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        let mut is_async = false;
        let mut start = self.peek().span;
        if self.match_keyword(Keyword::Async) {
            is_async = true;
            start = self.prev().span;
        }
        self.expect_keyword(Keyword::Fun)?;
        self.expect_with("'('", |k| matches!(k, TokenKind::LeftParen))?;
        let mut params = Vec::new();
        if !self.check(|k| matches!(k, TokenKind::RightParen)) {
            loop {
                let (name, name_span) = self.expect_identifier("lambda parameter")?;
                let ty = if self.match_with(|k| matches!(k, TokenKind::ColonColon)) {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                params.push(LambdaParam {
                    name,
                    ty,
                    span: name_span,
                });
                if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                    continue;
                }
                break;
            }
        }
        self.expect_with("')'", |k| matches!(k, TokenKind::RightParen))?;
        let mut return_type = None;
        if self.match_with(|k| matches!(k, TokenKind::ThinArrow)) {
            return_type = Some(self.parse_type()?);
        }
        let body = self.parse_block()?;
        let span = start.merge(body.span);
        Ok(Expr::Lambda(LambdaExpr {
            is_async,
            params,
            return_type,
            body,
            span,
        }))
    }

    fn parse_type(&mut self) -> Result<TypeExpr, ParseError> {
        if self.match_with(|k| matches!(k, TokenKind::Ampersand)) {
            let start = self.prev().span;
            let mutable = self.match_keyword(Keyword::Mut);
            let inner = self.parse_type()?;
            let span = start.merge(inner.span());
            return Ok(TypeExpr::Reference {
                mutable,
                inner: Box::new(inner),
                span,
            });
        }
        match self.peek().kind.clone() {
            TokenKind::Keyword(Keyword::Slice) => {
                let keyword_span = self.advance().span;
                self.expect_with("'<'", |k| matches!(k, TokenKind::Less))?;
                let inner = self.parse_type()?;
                let close = self
                    .expect_with("'>'", |k| matches!(k, TokenKind::Greater))?
                    .span;
                Ok(TypeExpr::Slice {
                    element: Box::new(inner),
                    span: keyword_span.merge(close),
                })
            }
            TokenKind::Keyword(Keyword::Tuple) => {
                let keyword_span = self.advance().span;
                self.expect_with("'('", |k| matches!(k, TokenKind::LeftParen))?;
                let mut elements = Vec::new();
                if !self.check(|k| matches!(k, TokenKind::RightParen)) {
                    loop {
                        elements.push(self.parse_type()?);
                        if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                            continue;
                        }
                        break;
                    }
                }
                let close = self
                    .expect_with("')'", |k| matches!(k, TokenKind::RightParen))?
                    .span;
                Ok(TypeExpr::Tuple {
                    elements,
                    span: keyword_span.merge(close),
                })
            }
            TokenKind::LeftBracket => {
                let open = self.advance().span;
                let element = self.parse_type()?;
                self.expect_with("';'", |k| matches!(k, TokenKind::Semicolon))?;
                let size_token =
                    self.expect_with("array size", |k| matches!(k, TokenKind::IntegerLiteral(_)))?;
                let size = if let TokenKind::IntegerLiteral(ref literal) = size_token.kind {
                    literal.replace('_', "").parse::<usize>().ok()
                } else {
                    None
                }
                .ok_or(ParseError::InvalidArraySize {
                    span: size_token.span,
                })?;
                let close = self
                    .expect_with("']'", |k| matches!(k, TokenKind::RightBracket))?
                    .span;
                Ok(TypeExpr::Array {
                    element: Box::new(element),
                    size,
                    span: open.merge(close),
                })
            }
            _ => self.parse_named_type(),
        }
    }

    fn parse_named_type(&mut self) -> Result<TypeExpr, ParseError> {
        let (name, mut span) = self.expect_identifier("type name")?;
        let mut segments = Vec::new();
        let generics = if self.match_with(|k| matches!(k, TokenKind::Less)) {
            let args = self.parse_type_arguments()?;
            span = span.merge(self.prev().span);
            args
        } else {
            Vec::new()
        };
        segments.push(TypeSegment {
            name,
            generics,
            span,
        });
        let mut overall = span;
        while self.match_with(|k| matches!(k, TokenKind::ColonColon | TokenKind::Dot)) {
            let (segment_name, mut seg_span) = self.expect_identifier("type segment")?;
            let args = if self.match_with(|k| matches!(k, TokenKind::Less)) {
                let parsed = self.parse_type_arguments()?;
                seg_span = seg_span.merge(self.prev().span);
                parsed
            } else {
                Vec::new()
            };
            overall = overall.merge(seg_span);
            segments.push(TypeSegment {
                name: segment_name,
                generics: args,
                span: seg_span,
            });
        }
        Ok(TypeExpr::Named(NamedType {
            segments,
            span: overall,
        }))
    }

    fn parse_type_arguments(&mut self) -> Result<Vec<TypeExpr>, ParseError> {
        let mut args = Vec::new();
        loop {
            args.push(self.parse_type()?);
            if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                continue;
            }
            break;
        }
        self.expect_with("'>'", |k| matches!(k, TokenKind::Greater))?;
        Ok(args)
    }

    fn parse_type_params(&mut self) -> Result<Vec<TypeParam>, ParseError> {
        if !self.match_with(|k| matches!(k, TokenKind::Less)) {
            return Ok(Vec::new());
        }
        let mut params = Vec::new();
        loop {
            let (name, span) = self.expect_identifier("type parameter")?;
            params.push(TypeParam { name, span });
            if self.match_with(|k| matches!(k, TokenKind::Comma)) {
                continue;
            }
            break;
        }
        self.expect_with("'>'", |k| matches!(k, TokenKind::Greater))?;
        Ok(params)
    }

    fn check_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(k) if k == keyword)
    }

    fn match_keyword(&mut self, keyword: Keyword) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check<F>(&self, predicate: F) -> bool
    where
        F: Fn(&TokenKind) -> bool,
    {
        predicate(&self.peek().kind)
    }

    fn match_with<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(&TokenKind) -> bool,
    {
        if predicate(&self.peek().kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<Token, ParseError> {
        self.expect_with(
            "keyword",
            |k| matches!(k, TokenKind::Keyword(kw) if *kw == keyword),
        )
    }

    fn expect_with<F>(&mut self, expected: &'static str, predicate: F) -> Result<Token, ParseError>
    where
        F: Fn(&TokenKind) -> bool,
    {
        let token = self.advance();
        if predicate(&token.kind) {
            Ok(token)
        } else {
            Err(ParseError::UnexpectedToken {
                expected,
                found: token.kind,
                span: token.span,
            })
        }
    }

    fn expect_identifier(&mut self, context: &'static str) -> Result<(String, Span), ParseError> {
        let token = self.expect_with(context, |k| matches!(k, TokenKind::Identifier(_)))?;
        if let TokenKind::Identifier(name) = token.kind.clone() {
            Ok((name, token.span))
        } else {
            unreachable!()
        }
    }

    fn expect_name(&mut self, context: &'static str) -> Result<(String, Span), ParseError> {
        let token = self.advance();
        match token.kind.clone() {
            TokenKind::Identifier(name) => Ok((name, token.span)),
            TokenKind::Keyword(keyword) => Ok((keyword.lexeme().to_string(), token.span)),
            _ => Err(ParseError::UnexpectedToken {
                expected: context,
                found: token.kind,
                span: token.span,
            }),
        }
    }

    fn expect_plain_identifier(
        &mut self,
        context: &'static str,
    ) -> Result<(String, Span), ParseError> {
        let token = self.expect_with(context, |k| matches!(k, TokenKind::Identifier(_)))?;
        if let TokenKind::Identifier(name) = token.kind {
            Ok((name, token.span))
        } else {
            unreachable!()
        }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.index)
            .unwrap_or_else(|| self.tokens.last().expect("tokens non-empty"))
    }

    fn prev(&self) -> &Token {
        if self.index == 0 {
            self.tokens.first().unwrap()
        } else {
            self.tokens.get(self.index - 1).unwrap()
        }
    }

    fn advance(&mut self) -> Token {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            token
        } else {
            self.tokens.last().expect("tokens not empty").clone()
        }
    }

    fn peek_kind_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.index + offset).map(|t| &t.kind)
    }

    fn struct_literal_follows(&self) -> bool {
        match (self.peek_kind_at(1), self.peek_kind_at(2)) {
            (Some(TokenKind::Identifier(_)), Some(TokenKind::Colon)) => true,
            (Some(TokenKind::RightBrace), _) => true,
            _ => false,
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn record_error(&mut self, err: ParseError) {
        self.errors.push(err);
    }

    fn synchronize_top(&mut self) {
        while !self.is_at_end() {
            if matches!(
                self.prev().kind,
                TokenKind::Semicolon | TokenKind::RightBrace
            ) {
                return;
            }
            match self.peek().kind {
                TokenKind::Keyword(Keyword::Struct)
                | TokenKind::Keyword(Keyword::Enum)
                | TokenKind::Keyword(Keyword::Trait)
                | TokenKind::Keyword(Keyword::Impl)
                | TokenKind::Keyword(Keyword::Extern)
                | TokenKind::Keyword(Keyword::Fun)
                | TokenKind::Keyword(Keyword::Async)
                | TokenKind::Keyword(Keyword::Import) => return,
                _ => {
                    self.advance();
                }
            }
        }
    }
}
