//! Code Lens provider for pytest fixtures.
//!
//! Shows "N usages" above fixture definitions.

use super::Backend;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tracing::info;

impl Backend {
    /// Handle code lens request - returns lenses for all fixtures in the file
    pub async fn handle_code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = &params.text_document.uri;

        info!("code_lens request: uri={:?}", uri);

        let Some(file_path) = self.uri_to_path(uri) else {
            return Ok(None);
        };

        // Get all definitions in this file
        let mut lenses = Vec::new();

        for entry in self.fixture_db.definitions.iter() {
            for def in entry.value() {
                // Only show lenses for fixtures defined in this file
                if def.file_path != file_path || def.is_third_party {
                    continue;
                }

                // Count usages for this definition
                let references = self.fixture_db.find_references_for_definition(def);
                let usage_count = references.len();

                let line = Self::internal_line_to_lsp(def.line);
                let range = Self::create_range(line, 0, line, 0);

                let title = if usage_count == 1 {
                    "1 usage".to_string()
                } else {
                    format!("{} usages", usage_count)
                };

                let lens = CodeLens {
                    range,
                    command: Some(Command {
                        title,
                        command: "pytest-lsp.findReferences".to_string(),
                        arguments: Some(vec![
                            serde_json::to_value(uri.to_string()).unwrap(),
                            serde_json::to_value(line).unwrap(),
                            serde_json::to_value(def.start_char).unwrap(),
                        ]),
                    }),
                    data: None,
                };

                lenses.push(lens);
            }
        }

        info!("Returning {} code lenses for {:?}", lenses.len(), file_path);

        if lenses.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lenses))
        }
    }
}
