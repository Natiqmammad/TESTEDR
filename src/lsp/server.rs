use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::io::{stdin, stdout};
use tokio::sync::Mutex;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::ast::Item;
use crate::diagnostics::AfnsError;
use crate::lexer;
use crate::parser;
use crate::span::Span;
use crate::token::TokenKind;

pub async fn run_stdio_server() -> Result<()> {
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin(), stdout(), socket).serve(service).await;
    Ok(())
}

struct Backend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn set_document(&self, uri: &Url, text: String) {
        self.documents.lock().await.insert(uri.clone(), text);
    }

    async fn remove_document(&self, uri: &Url) {
        self.documents.lock().await.remove(uri);
    }

    async fn get_document(&self, uri: &Url) -> Option<String> {
        self.documents.lock().await.get(uri).cloned()
    }

    async fn publish_diagnostics(&self, uri: &Url, text: &str) {
        let diagnostics = collect_diagnostics(text);
        let _ = self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    completion_item: None,
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "ApexForge NightScript LSP".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let _ = self.client.log_message(MessageType::INFO, "Apex LSP ready").await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.set_document(&uri, text.clone()).await;
        self.publish_diagnostics(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.set_document(&uri, change.text.clone()).await;
            self.publish_diagnostics(&uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.remove_document(&uri).await;
        let _ = self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn completion(
        &self,
        _: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        let mut items = Vec::new();
        for keyword in completion_keywords() {
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                insert_text: Some(keyword.to_string()),
                ..CompletionItem::default()
            });
        }
        for pkg in completion_packages() {
            items.push(CompletionItem {
                label: pkg.to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("package".to_string()),
                insert_text: Some(pkg.to_string()),
                ..CompletionItem::default()
            });
        }
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        if let Some(text) = self.get_document(&uri).await {
            if let Some((label, range)) = token_at_position(&text, params.text_document_position_params.position) {
                let contents = HoverContents::Scalar(MarkedString::String(label));
                return Ok(Some(Hover { contents, range: Some(range) }));
            }
        }
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        if let Some(text) = self.get_document(&uri).await {
            if let Some(range) = find_definition(&text, params.text_document_position_params.position) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location { uri, range })));
            }
        }
        Ok(None)
    }
}

fn completion_keywords() -> &'static [&'static str] {
    &[
        "fun", "struct", "enum", "import", "async", "await", "return", "let", "var", "impl",
        "trait", "for", "while", "switch", "try", "catch",
    ]
}

fn completion_packages() -> &'static [&'static str] {
    &["forge", "forge.io", "forge.math", "math", "http", "io"]
}

fn collect_diagnostics(source: &str) -> Vec<Diagnostic> {
    match lexer::lex(source) {
        Ok(tokens) => match parser::parse_tokens(source, tokens) {
            Ok(_) => Vec::new(),
            Err(err) => vec![diagnostic_from_error(source, &err)],
        },
        Err(err) => vec![diagnostic_from_error(source, &AfnsError::from(err))],
    }
}

fn diagnostic_from_error(source: &str, error: &AfnsError) -> Diagnostic {
    let message = format!("{error}");
    let range = error
        .span()
        .map(span_to_range)
        .unwrap_or_else(|| Range::new(Position::new(0, 0), Position::new(0, 1)));
    Diagnostic {
        severity: Some(DiagnosticSeverity::ERROR),
        range,
        message,
        source: Some("afns".to_string()),
        ..Diagnostic::default()
    }
}

fn span_to_range(span: Span) -> Range {
    let line = span.line.saturating_sub(1) as u32;
    let start_col = span.column.saturating_sub(1) as u32;
    let len = span.end.saturating_sub(span.start) as u32;
    let end_col = start_col + len.max(1);
    Range::new(Position::new(line, start_col), Position::new(line, end_col))
}

fn token_at_position(text: &str, position: Position) -> Option<(String, Range)> {
    let tokens = lexer::lex(text).ok()?;
    for token in tokens {
        if let TokenKind::Eof = token.kind {
            continue;
        }
        if position_in_range(position, span_to_range(token.span)) {
            let label = format!("{:?}", token.kind);
            return Some((label, span_to_range(token.span)));
        }
    }
    None
}

fn position_in_range(pos: Position, range: Range) -> bool {
    (pos.line > range.start.line || (pos.line == range.start.line && pos.character >= range.start.character))
        && (pos.line < range.end.line || (pos.line == range.end.line && pos.character <= range.end.character))
}

fn find_definition(text: &str, position: Position) -> Option<Range> {
    let word = word_at_position(text, position)?;
    let tokens = lexer::lex(text).ok()?;
    let ast = parser::parse_tokens(text, tokens).ok()?;
    for item in ast.items {
        if let Item::Function(func) = item {
            if func.signature.name == word {
                return Some(span_to_range(func.signature.span));
            }
        }
    }
    None
}

fn word_at_position(text: &str, position: Position) -> Option<String> {
    let line = text.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line.chars().collect();
    if (position.character as usize) > chars.len() {
        return None;
    }
    let mut start = position.character as isize - 1;
    let mut end = position.character as usize;
    while start >= 0 && is_word_char(chars[start as usize]) {
        start -= 1;
    }
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    if (start + 1) as usize >= end {
        return None;
    }
    Some(chars[(start + 1) as usize..end].iter().collect())
}

fn is_word_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

trait ErrorSpan {
    fn span(&self) -> Option<Span>;
}

impl ErrorSpan for AfnsError {
    fn span(&self) -> Option<Span> {
        match self {
            AfnsError::Lex(err) => err.span(),
            AfnsError::Parse(err) => err.span(),
        }
    }
}
