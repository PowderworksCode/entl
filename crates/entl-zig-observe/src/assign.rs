//! Where a field is assigned, and what it is called on.
//!
//! A field's declaration says what it holds. It does not say where the value
//! came from, and for a pointer that is the whole question: `self.gradient =
//! bun.create(gpa, Gradient, .{})` allocates, `self.globalThis = globalObject`
//! borrows a parameter, and `.data = Data.Store.append(x)` puts the value in an
//! arena that frees it in bulk. All three are spelled `*Foo` at the declaration.
//!
//! This reports the assignments and the method calls, as written. It does not
//! interpret them — whether `bun.create` allocates is a table of library
//! knowledge, and it belongs to whoever holds that table.
//!
//! ## The grammar calls an assignment a declaration
//!
//! `self.field = value;` as a statement parses as `variable_declaration` whose
//! first child is a `field_expression`, not as any kind of assignment node. Only
//! `.field = value` inside an initialiser list gets `assignment_expression`. Both
//! are reported here as [`FieldAssignment`], distinguished by [`AssignmentForm`],
//! so a consumer never has to know that.

use std::path::{Path, PathBuf};

use entl_tree_sitter::ParsedFile;
use tree_sitter::Node;

use crate::Span;

/// How the assignment was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssignmentForm {
    /// `self.field = value;` — a statement.
    Statement,
    /// `.field = value` — inside a struct initialiser.
    Initializer,
}

/// One assignment to a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAssignment {
    pub path: PathBuf,
    /// What the field was reached through: `self`, `this`, `poll.store`, or
    /// empty for the `.field =` form, which names no receiver.
    pub receiver: String,
    pub field: String,
    /// The right-hand side, as source text.
    pub value: String,
    pub form: AssignmentForm,
    /// The whole assignment.
    pub span: Span,
    /// Just the right-hand side.
    pub value_span: Span,
}

/// One method call, with the expression it was called on.
///
/// `self.str.deref()` is how a Zig field releases a reference it holds, which
/// is a thing a consumer wants to know about `str` and cannot learn from `str`'s
/// declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodCall {
    pub path: PathBuf,
    /// The receiver as source text: `self.str`.
    pub receiver: String,
    /// The last identifier of the receiver, which is usually the field:
    /// `str` for `self.str.deref()`.
    pub receiver_tail: String,
    pub method: String,
    pub span: Span,
}

/// Every field assignment written in one parsed file, in source order.
pub fn assignments(file: &ParsedFile) -> Vec<FieldAssignment> {
    let mut out = Vec::new();
    collect_assignments(file.tree.root_node(), &file.source, &file.path, &mut out);
    out
}

/// Every method call written in one parsed file, in source order.
pub fn method_calls(file: &ParsedFile) -> Vec<MethodCall> {
    let mut out = Vec::new();
    collect_calls(file.tree.root_node(), &file.source, &file.path, &mut out);
    out
}

fn text(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(str::to_string)
}

/// Split a `field_expression` into what it was reached through and the final
/// name. `self.str` is (`self`, `str`); `.vm` is (``, `vm`).
fn split_field_expression(node: Node<'_>, source: &[u8]) -> Option<(String, String)> {
    if node.kind() != "field_expression" {
        return None;
    }
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
    let dot = children.iter().rposition(|child| child.kind() == ".")?;
    let field = children
        .get(dot + 1)
        .filter(|child| child.kind() == "identifier")
        .and_then(|child| text(*child, source))?;
    let receiver = children
        .get(..dot)
        .and_then(|before| before.last())
        .and_then(|child| text(*child, source))
        .unwrap_or_default();
    Some((receiver, field))
}

fn collect_assignments(node: Node<'_>, source: &[u8], path: &Path, out: &mut Vec<FieldAssignment>) {
    // Both forms are `<field_expression> = <value>`; only the node kind above
    // them differs, and only because of how the grammar reads a statement.
    let form = match node.kind() {
        "variable_declaration" => Some(AssignmentForm::Statement),
        "assignment_expression" => Some(AssignmentForm::Initializer),
        _ => None,
    };
    if let Some(form) = form {
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        if let Some(assign) = children.iter().position(|child| child.kind() == "=")
            && let Some(left) = children.get(assign.wrapping_sub(1))
            && let Some((receiver, field)) = split_field_expression(*left, source)
            && let Some(value) = children.get(assign + 1)
            && let Some(value_text) = text(*value, source)
        {
            out.push(FieldAssignment {
                path: path.to_path_buf(),
                receiver,
                field,
                value: value_text,
                form,
                span: Span::of(node),
                value_span: Span::of(*value),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_assignments(child, source, path, out);
    }
}

fn collect_calls(node: Node<'_>, source: &[u8], path: &Path, out: &mut Vec<MethodCall>) {
    if node.kind() == "call_expression"
        && let Some(callee) = node.child(0)
        && let Some((receiver, method)) = split_field_expression(callee, source)
        && !receiver.is_empty()
    {
        let tail = receiver
            .rsplit('.')
            .next()
            .unwrap_or(&receiver)
            .trim_start_matches(['?', '*', '&'])
            .to_string();
        out.push(MethodCall {
            path: path.to_path_buf(),
            receiver,
            receiver_tail: tail,
            method,
            span: Span::of(node),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls(child, source, path, out);
    }
}
