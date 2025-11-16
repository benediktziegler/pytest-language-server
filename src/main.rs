use pytest_language_server::FixtureDatabase;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{info, warn};

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
                    .log_message(
                        MessageType::INFO,
                        format!("Scanning workspace: {:?}", root_path),
                    )
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
                name: "pytest-language-server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
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
            .log_message(MessageType::INFO, "pytest-language-server initialized")
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

        info!(
            "goto_definition request: uri={:?}, line={}, char={}",
            uri, position.line, position.character
        );

        if let Ok(file_path) = uri.to_file_path() {
            info!(
                "Looking for fixture definition at {:?}:{}:{}",
                file_path, position.line, position.character
            );

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

        info!(
            "hover request: uri={:?}, line={}, char={}",
            uri, position.line, position.character
        );

        if let Ok(file_path) = uri.to_file_path() {
            info!(
                "Looking for fixture at {:?}:{}:{}",
                file_path, position.line, position.character
            );

            if let Some(definition) = self.fixture_db.find_fixture_definition(
                &file_path,
                position.line,
                position.character,
            ) {
                info!("Found fixture definition for hover: {:?}", definition.name);

                // Build hover content
                let mut content = String::new();

                // Header with fixture name
                content.push_str(&format!(
                    "```python\n@pytest.fixture\ndef {}(...):\n```\n",
                    definition.name
                ));

                // Add file path
                if let Some(file_name) = definition.file_path.file_name() {
                    content.push_str(&format!(
                        "\n**Defined in:** `{}`\n",
                        file_name.to_string_lossy()
                    ));
                }

                // Add docstring if present
                if let Some(ref docstring) = definition.docstring {
                    content.push_str("\n---\n\n");

                    // Check if docstring looks like it contains markdown formatting
                    // (contains headers, lists, code blocks, etc.)
                    let looks_like_markdown = docstring.contains("```")
                        || docstring.contains("# ")
                        || docstring.lines().any(|l| {
                            l.trim_start().starts_with("- ") || l.trim_start().starts_with("* ")
                        });

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

        info!(
            "references request: uri={:?}, line={}, char={}",
            uri, position.line, position.character
        );

        if let Ok(file_path) = uri.to_file_path() {
            info!(
                "Looking for fixture references at {:?}:{}:{}",
                file_path, position.line, position.character
            );

            // First, find which fixture we're looking at (definition or usage)
            if let Some(fixture_name) = self.fixture_db.find_fixture_at_position(
                &file_path,
                position.line,
                position.character,
            ) {
                info!(
                    "Found fixture: {}, determining which definition to use",
                    fixture_name
                );

                // Determine which specific definition the user is referring to
                // This could be a definition (if they clicked on the fixture name in a def line)
                // or a usage (in which case we need to resolve it)
                let target_definition = self.fixture_db.find_fixture_definition(
                    &file_path,
                    position.line,
                    position.character,
                );

                let references = if let Some(definition) = target_definition {
                    info!(
                        "Found definition at {:?}:{}, finding references that resolve to it",
                        definition.file_path, definition.line
                    );
                    // Find only references that resolve to this specific definition
                    self.fixture_db.find_references_for_definition(&definition)
                } else {
                    info!("No specific definition found, finding all references by name");
                    // Fallback to finding all references by name
                    self.fixture_db.find_fixture_references(&fixture_name)
                };

                if references.is_empty() {
                    info!("No references found for fixture: {}", fixture_name);
                    return Ok(None);
                }

                info!(
                    "Found {} references for fixture: {}",
                    references.len(),
                    fixture_name
                );

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
    // Set up stderr logging with env-filter support
    // Users can control verbosity with RUST_LOG env var:
    // RUST_LOG=debug pytest-language-server
    // RUST_LOG=info pytest-language-server
    // RUST_LOG=warn pytest-language-server (default)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    info!("pytest-language-server starting");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let fixture_db = Arc::new(FixtureDatabase::new());

    let (service, socket) = LspService::new(|client| Backend {
        client,
        fixture_db: fixture_db.clone(),
    });

    info!("LSP server ready");
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use pytest_language_server::FixtureDefinition;
    use std::path::PathBuf;

    #[test]
    fn test_hover_content_with_leading_newline() {
        // Create a mock fixture definition with docstring
        let definition = FixtureDefinition {
            name: "my_fixture".to_string(),
            file_path: PathBuf::from("/tmp/test/conftest.py"),
            line: 4,
            docstring: Some("This is a test fixture.\n\nIt does something useful.".to_string()),
        };

        // Build hover content (same logic as hover method)
        let mut content = String::new();

        // Header with fixture name
        content.push_str(&format!(
            "```python\n@pytest.fixture\ndef {}(...):\n```\n",
            definition.name
        ));

        // Add file path
        if let Some(file_name) = definition.file_path.file_name() {
            content.push_str(&format!(
                "\n**Defined in:** `{}`\n",
                file_name.to_string_lossy()
            ));
        }

        // Add docstring if present
        if let Some(ref docstring) = definition.docstring {
            content.push_str("\n---\n\n");
            content.push_str(docstring);
        }

        // Verify the structure with empty line after code block
        let lines: Vec<&str> = content.lines().collect();

        // The structure should be:
        // 0: ```python
        // 1: @pytest.fixture
        // 2: def my_fixture(...):
        // 3: ```
        // 4: (empty line)
        // 5: **Defined in:** `conftest.py`
        // 6: (empty line from \n---\n)
        // 7: ---
        // 8: (empty line)
        // 9+: docstring content

        assert_eq!(lines[0], "```python");
        assert_eq!(lines[1], "@pytest.fixture");
        assert!(lines[2].starts_with("def my_fixture"));
        assert_eq!(lines[3], "```");
        assert_eq!(lines[4], ""); // Empty line after code block
        assert!(
            lines[5].starts_with("**Defined in:**"),
            "Line 5 should be 'Defined in', got: '{}'",
            lines[5]
        );
    }

    #[test]
    fn test_hover_content_structure_without_docstring() {
        // Create a mock fixture definition without docstring
        let definition = FixtureDefinition {
            name: "simple_fixture".to_string(),
            file_path: PathBuf::from("/tmp/test/conftest.py"),
            line: 4,
            docstring: None,
        };

        // Build hover content
        let mut content = String::new();

        content.push_str(&format!(
            "```python\n@pytest.fixture\ndef {}(...):\n```\n",
            definition.name
        ));

        if let Some(file_name) = definition.file_path.file_name() {
            content.push_str(&format!(
                "\n**Defined in:** `{}`\n",
                file_name.to_string_lossy()
            ));
        }

        // For a fixture without docstring, the content should end with "Defined in"
        // with an empty line before it
        let lines: Vec<&str> = content.lines().collect();

        assert_eq!(lines.len(), 6); // code block (4 lines) + empty line + defined in (1 line)
        assert_eq!(lines[4], ""); // Empty line
        assert!(lines[5].starts_with("**Defined in:**"));
    }
}
