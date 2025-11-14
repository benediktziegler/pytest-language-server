mod fixtures;

use fixtures::FixtureDatabase;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{info, warn, debug};

#[derive(Debug)]
struct Backend {
    client: Client,
    fixture_db: Arc<FixtureDatabase>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Initialize request received");
        
        // Scan the workspace for fixtures on initialization
        if let Some(root_uri) = params.root_uri.clone() {
            if let Ok(root_path) = root_uri.to_file_path() {
                info!("Scanning workspace: {:?}", root_path);
                self.client
                    .log_message(MessageType::INFO, format!("Scanning workspace: {:?}", root_path))
                    .await;
                self.fixture_db.scan_workspace(&root_path);
                info!("Workspace scan complete");
                self.client
                    .log_message(MessageType::INFO, "Workspace scan complete")
                    .await;
            }
        } else {
            warn!("No root URI provided in initialize");
        }

        info!("Returning initialize result with capabilities");
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "pytest-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("Server initialized notification received");
        self.client
            .log_message(MessageType::INFO, "pytest-lsp server initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        info!("did_open: {:?}", uri);
        if let Ok(file_path) = uri.to_file_path() {
            info!("Analyzing file: {:?}", file_path);
            self.fixture_db
                .analyze_file(file_path, &params.text_document.text);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        info!("did_change: {:?}", uri);
        if let Ok(file_path) = uri.to_file_path() {
            if let Some(change) = params.content_changes.first() {
                info!("Re-analyzing file: {:?}", file_path);
                self.fixture_db.analyze_file(file_path, &change.text);
            }
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!("goto_definition request: uri={:?}, line={}, char={}", uri, position.line, position.character);

        if let Ok(file_path) = uri.to_file_path() {
            info!("Looking for fixture definition at {:?}:{}:{}", file_path, position.line, position.character);
            
            if let Some(definition) = self.fixture_db.find_fixture_definition(
                &file_path,
                position.line,
                position.character,
            ) {
                info!("Found definition: {:?}", definition);
                let def_uri = match Url::from_file_path(&definition.file_path) {
                    Ok(uri) => uri,
                    Err(_) => {
                        warn!("Failed to convert path to URI: {:?}", definition.file_path);
                        return Ok(None);
                    }
                };
                
                let location = Location {
                    uri: def_uri.clone(),
                    range: Range {
                        start: Position {
                            line: (definition.line.saturating_sub(1)) as u32,
                            character: 0,
                        },
                        end: Position {
                            line: (definition.line.saturating_sub(1)) as u32,
                            character: 0,
                        },
                    },
                };
                info!("Returning location: {:?}", location);
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            } else {
                info!("No fixture definition found");
            }
        } else {
            warn!("Failed to convert URI to file path: {:?}", uri);
        }

        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!("hover request: uri={:?}, line={}, char={}", uri, position.line, position.character);

        if let Ok(file_path) = uri.to_file_path() {
            info!("Looking for fixture at {:?}:{}:{}", file_path, position.line, position.character);
            
            if let Some(definition) = self.fixture_db.find_fixture_definition(
                &file_path,
                position.line,
                position.character,
            ) {
                info!("Found fixture definition for hover: {:?}", definition.name);
                
                // Build hover content
                let mut content = String::new();
                
                // Header with fixture name
                content.push_str(&format!("```python\n@pytest.fixture\ndef {}(...):\n```\n", definition.name));
                
                // Add file path
                if let Some(file_name) = definition.file_path.file_name() {
                    content.push_str(&format!("\n**Defined in:** `{}`\n", file_name.to_string_lossy()));
                }
                
                // Add docstring if present
                if let Some(ref docstring) = definition.docstring {
                    content.push_str("\n---\n\n");
                    
                    // Check if docstring looks like it contains markdown formatting
                    // (contains headers, lists, code blocks, etc.)
                    let looks_like_markdown = docstring.contains("```") 
                        || docstring.contains("# ")
                        || docstring.lines().any(|l| l.trim_start().starts_with("- ") || l.trim_start().starts_with("* "));
                    
                    if looks_like_markdown {
                        // Render as markdown
                        content.push_str(docstring);
                    } else {
                        // Render as plain text (no extra escaping needed, just display)
                        content.push_str(docstring);
                    }
                }
                
                info!("Returning hover with content");
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: content,
                    }),
                    range: None,
                }));
            } else {
                info!("No fixture found for hover");
            }
        } else {
            warn!("Failed to convert URI to file path: {:?}", uri);
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        info!("references request: uri={:?}, line={}, char={}", uri, position.line, position.character);

        if let Ok(file_path) = uri.to_file_path() {
            info!("Looking for fixture references at {:?}:{}:{}", file_path, position.line, position.character);
            
            // First, find which fixture we're looking at (definition or usage)
            if let Some(fixture_name) = self.fixture_db.find_fixture_at_position(
                &file_path,
                position.line,
                position.character,
            ) {
                info!("Found fixture: {}, searching for all references", fixture_name);
                
                // Find all references to this fixture
                let references = self.fixture_db.find_fixture_references(&fixture_name);
                
                if references.is_empty() {
                    info!("No references found for fixture: {}", fixture_name);
                    return Ok(None);
                }
                
                info!("Found {} references for fixture: {}", references.len(), fixture_name);
                
                // Convert references to LSP Locations
                let mut locations = Vec::new();
                for reference in references {
                    let ref_uri = match Url::from_file_path(&reference.file_path) {
                        Ok(uri) => uri,
                        Err(_) => {
                            warn!("Failed to convert path to URI: {:?}", reference.file_path);
                            continue;
                        }
                    };
                    
                    let location = Location {
                        uri: ref_uri,
                        range: Range {
                            start: Position {
                                line: (reference.line.saturating_sub(1)) as u32,
                                character: 0,
                            },
                            end: Position {
                                line: (reference.line.saturating_sub(1)) as u32,
                                character: 0,
                            },
                        },
                    };
                    locations.push(location);
                }
                
                info!("Returning {} locations", locations.len());
                return Ok(Some(locations));
            } else {
                info!("No fixture found at this position");
            }
        } else {
            warn!("Failed to convert URI to file path: {:?}", uri);
        }

        Ok(None)
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Set up file logging
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let log_path = std::path::Path::new(&home_dir).join(".pytest_lsp.log");
    
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");
    
    let (non_blocking, _guard) = tracing_appender::non_blocking(file);
    
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("pytest-lsp starting up, logging to {:?}", log_path);

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let fixture_db = Arc::new(FixtureDatabase::new());

    let (service, socket) = LspService::new(|client| Backend {
        client,
        fixture_db: fixture_db.clone(),
    });
    
    info!("LSP server starting");
    Server::new(stdin, stdout, socket).serve(service).await;
}
