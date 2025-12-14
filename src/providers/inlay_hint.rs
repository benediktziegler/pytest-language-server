//! Inlay hints provider for pytest fixtures.
//!
//! Shows fixture return types inline for fixture parameters in test functions
//! when the fixture has an explicit return type annotation.

use super::Backend;
use std::collections::HashMap;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tracing::info;

impl Backend {
    /// Handle inlay hints request.
    ///
    /// Returns type hints for fixture parameters when the fixture has an explicit
    /// return type annotation. This helps developers understand what type each
    /// fixture provides without having to navigate to its definition.
    pub async fn handle_inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let range = params.range;

        info!("inlay_hint request: uri={:?}, range={:?}", uri, range);

        let Some(file_path) = self.uri_to_path(&uri) else {
            return Ok(None);
        };

        let Some(usages) = self.fixture_db.usages.get(&file_path) else {
            return Ok(None);
        };

        // Pre-compute a map of fixture name -> definition for O(1) lookup.
        // This avoids calling find_closest_definition for each usage.
        let available = self.fixture_db.get_available_fixtures(&file_path);
        let fixture_map: HashMap<&str, &str> = available
            .iter()
            .filter_map(|def| {
                def.return_type
                    .as_ref()
                    .map(|rt| (def.name.as_str(), rt.as_str()))
            })
            .collect();

        // Early return if no fixtures have return types
        if fixture_map.is_empty() {
            return Ok(Some(Vec::new()));
        }

        // Convert LSP range to internal line numbers (1-based)
        let start_line = Self::lsp_line_to_internal(range.start.line);
        let end_line = Self::lsp_line_to_internal(range.end.line);

        let mut hints = Vec::new();

        for usage in usages.iter() {
            // Only process usages within the requested range
            if usage.line < start_line || usage.line > end_line {
                continue;
            }

            // Look up return type from pre-computed map
            if let Some(&return_type) = fixture_map.get(usage.name.as_str()) {
                let lsp_line = Self::internal_line_to_lsp(usage.line);

                hints.push(InlayHint {
                    position: Position {
                        line: lsp_line,
                        character: usage.end_char as u32,
                    },
                    label: InlayHintLabel::String(format!(": {}", return_type)),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(format!(
                        "Fixture '{}' returns {}",
                        usage.name, return_type
                    ))),
                    padding_left: Some(false),
                    padding_right: Some(true),
                    data: None,
                });
            }
        }

        info!("Returning {} inlay hints", hints.len());
        Ok(Some(hints))
    }
}
