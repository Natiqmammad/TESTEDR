use anyhow::Result;
use nightscript_android::lsp::server;

#[tokio::main]
async fn main() -> Result<()> {
    server::run_stdio_server().await
}
