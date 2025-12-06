//! Docstring and return type extraction from Python AST.
//!
//! This module handles extracting documentation and type information
//! from Python function definitions.

use super::FixtureDatabase;
use rustpython_parser::ast::{Expr, Stmt};

impl FixtureDatabase {
    /// Extract docstring from a function body.
    /// The docstring is the first statement if it's a string literal.
    pub(crate) fn extract_docstring(&self, body: &[Stmt]) -> Option<String> {
        if let Some(Stmt::Expr(expr_stmt)) = body.first() {
            if let Expr::Constant(constant) = &*expr_stmt.value {
                if let rustpython_parser::ast::Constant::Str(s) = &constant.value {
                    return Some(super::string_utils::format_docstring(s.to_string()));
                }
            }
        }
        None
    }

    /// Extract return type from a function's return annotation.
    /// For yield fixtures (generators), extracts the yielded type from Generator[T, ...].
    pub(crate) fn extract_return_type(
        &self,
        returns: &Option<Box<rustpython_parser::ast::Expr>>,
        body: &[Stmt],
        content: &str,
    ) -> Option<String> {
        if let Some(return_expr) = returns {
            let has_yield = self.contains_yield(body);

            if has_yield {
                return self.extract_yielded_type(return_expr, content);
            } else {
                return Some(self.expr_to_string(return_expr, content));
            }
        }
        None
    }

    /// Check if a function body contains yield statements.
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn contains_yield(&self, body: &[Stmt]) -> bool {
        for stmt in body {
            match stmt {
                Stmt::Expr(expr_stmt) => {
                    if let Expr::Yield(_) | Expr::YieldFrom(_) = &*expr_stmt.value {
                        return true;
                    }
                }
                Stmt::If(if_stmt) => {
                    if self.contains_yield(&if_stmt.body) || self.contains_yield(&if_stmt.orelse) {
                        return true;
                    }
                }
                Stmt::For(for_stmt) => {
                    if self.contains_yield(&for_stmt.body) || self.contains_yield(&for_stmt.orelse)
                    {
                        return true;
                    }
                }
                Stmt::While(while_stmt) => {
                    if self.contains_yield(&while_stmt.body)
                        || self.contains_yield(&while_stmt.orelse)
                    {
                        return true;
                    }
                }
                Stmt::With(with_stmt) => {
                    if self.contains_yield(&with_stmt.body) {
                        return true;
                    }
                }
                Stmt::Try(try_stmt) => {
                    if self.contains_yield(&try_stmt.body)
                        || self.contains_yield(&try_stmt.orelse)
                        || self.contains_yield(&try_stmt.finalbody)
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Extract the yielded type from a Generator/Iterator type annotation.
    /// For Generator[T, None, None] or Iterator[T], returns T.
    fn extract_yielded_type(
        &self,
        expr: &rustpython_parser::ast::Expr,
        content: &str,
    ) -> Option<String> {
        if let Expr::Subscript(subscript) = expr {
            if let Expr::Tuple(tuple) = &*subscript.slice {
                if let Some(first_elem) = tuple.elts.first() {
                    return Some(self.expr_to_string(first_elem, content));
                }
            } else {
                return Some(self.expr_to_string(&subscript.slice, content));
            }
        }
        Some(self.expr_to_string(expr, content))
    }

    /// Convert a Python type expression AST node to a string representation.
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn expr_to_string(
        &self,
        expr: &rustpython_parser::ast::Expr,
        content: &str,
    ) -> String {
        match expr {
            Expr::Name(name) => name.id.to_string(),
            Expr::Attribute(attr) => {
                format!(
                    "{}.{}",
                    self.expr_to_string(&attr.value, content),
                    attr.attr
                )
            }
            Expr::Subscript(subscript) => {
                let base = self.expr_to_string(&subscript.value, content);
                let slice = self.expr_to_string(&subscript.slice, content);
                format!("{}[{}]", base, slice)
            }
            Expr::Tuple(tuple) => {
                let elements: Vec<String> = tuple
                    .elts
                    .iter()
                    .map(|e| self.expr_to_string(e, content))
                    .collect();
                elements.join(", ")
            }
            Expr::Constant(constant) => {
                format!("{:?}", constant.value)
            }
            Expr::BinOp(binop) if matches!(binop.op, rustpython_parser::ast::Operator::BitOr) => {
                format!(
                    "{} | {}",
                    self.expr_to_string(&binop.left, content),
                    self.expr_to_string(&binop.right, content)
                )
            }
            _ => "Any".to_string(),
        }
    }
}
