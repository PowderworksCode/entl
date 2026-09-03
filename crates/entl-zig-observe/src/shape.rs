//! What a pointer type actually says, and the three constructs that talk about
//! lifetime directly.
//!
//! [`crate::ContainerField::zig_type`] is the type as written, which is the
//! right thing to carry and the wrong thing to test with `contains('*')`. A
//! `*const Foo` cannot be freed through, a `?*Foo` may be absent, and a `[*]u8`
//! is a C array rather than a pointer to one value. All three read alike to a
//! substring search and mean different things to a port.
//!
//! The rest is the syntax that names lifetime out loud:
//!
//! - `defer` and `errdefer` say who cleans up. `errdefer alloc.destroy(self)`
//!   in a constructor is the author stating that until the function returns,
//!   *this* frame owns the memory.
//! - `@fieldParentPtr` recovers a container from a field embedded in it. It is
//!   the intrusive-collection idiom, and a field reached that way is pointed at
//!   by something that does not own it.

use std::path::PathBuf;

use entl_tree_sitter::ParsedFile;
use tree_sitter::Node;

use crate::{Span, text};

/// What kind of pointer a type expression is, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PointerShape {
    /// Not a pointer at all.
    None,
    /// `*T` — one value, mutable through.
    Single,
    /// `*const T` — one value, not writable through, so not freeable through.
    SingleConst,
    /// `[*]T` or `[*c]T` — a C-style run of values with no length.
    Many,
    /// `[]T` or `[]const T` — a pointer and a length together.
    Slice,
}

impl PointerShape {
    pub fn is_pointer(self) -> bool {
        self != PointerShape::None
    }

    /// Whether memory could be released through this pointer.
    ///
    /// A `*const T` cannot be passed to a `free` that takes a mutable pointer,
    /// so a field declared `*const` is evidence *against* ownership that the
    /// written type carries and a `contains('*')` test throws away.
    pub fn can_free_through(self) -> bool {
        matches!(
            self,
            PointerShape::Single | PointerShape::Many | PointerShape::Slice
        )
    }
}

/// What a type expression says about pointer-ness, nullability and constness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeShape {
    pub pointer: PointerShape,
    /// `?T` — the type admits absence.
    pub optional: bool,
    /// The pointee is `const`.
    pub constant: bool,
}

impl TypeShape {
    /// Read a type as written.
    ///
    /// Text rather than tree, because `*jsc.VirtualMachine` does not parse as a
    /// pointer to a qualified name — the grammar groups it as a field access on
    /// `*jsc` — and the written form is what the author meant. See the note on
    /// [`crate::ContainerField`].
    pub fn of(written: &str) -> Self {
        let trimmed = written.trim();
        let optional = trimmed.starts_with('?');
        let rest = trimmed.trim_start_matches('?').trim_start();
        let (pointer, after) = if let Some(rest) = rest.strip_prefix("[*c]") {
            (PointerShape::Many, rest)
        } else if let Some(rest) = rest.strip_prefix("[*]") {
            (PointerShape::Many, rest)
        } else if let Some(rest) = rest.strip_prefix("[]") {
            (PointerShape::Slice, rest)
        } else if let Some(rest) = rest.strip_prefix('*') {
            (PointerShape::Single, rest)
        } else {
            (PointerShape::None, rest)
        };
        let constant = after.trim_start().starts_with("const ");
        let pointer = match (pointer, constant) {
            (PointerShape::Single, true) => PointerShape::SingleConst,
            (shape, _) => shape,
        };
        TypeShape {
            pointer,
            optional,
            constant,
        }
    }

    pub fn is_pointer(self) -> bool {
        self.pointer.is_pointer()
    }

    /// The type with its pointer, optional and const markers removed.
    pub fn pointee(written: &str) -> &str {
        written
            .trim()
            .trim_start_matches('?')
            .trim_start()
            .trim_start_matches("[*c]")
            .trim_start_matches("[*]")
            .trim_start_matches("[]")
            .trim_start_matches('*')
            .trim_start()
            .trim_start_matches("const ")
            .trim()
    }
}

/// Whether the cleanup runs always or only on the way out of a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeferKind {
    /// `defer` — runs when the scope ends, however it ends.
    Always,
    /// `errdefer` — runs only if the scope is left by an error.
    OnError,
}

/// A deferred cleanup, and what it was written on.
///
/// The distinction matters for ownership. `errdefer alloc.destroy(self)` says
/// this frame owns the value *until it successfully hands it over*, so the
/// value is transferred rather than kept. A plain `defer thing.deinit()` says
/// this frame owns it outright and nothing outlives the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deferred {
    pub path: PathBuf,
    pub kind: DeferKind,
    /// The deferred expression as written: `alloc.destroy(self)`.
    pub expression: String,
    pub span: Span,
}

/// A `@fieldParentPtr` recovery, which names the field it walks back through.
///
/// `@fieldParentPtr("node", node)` says: this `node` is embedded in some
/// container, give me the container. A field used this way is a hook something
/// else points at, not memory this container allocated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentRecovery {
    pub path: PathBuf,
    /// The field name the recovery walks back through, when written as a
    /// literal. Empty when the argument is computed.
    pub field: String,
    /// The expression the container is recovered from.
    pub value: String,
    pub span: Span,
}

/// Every `defer` and `errdefer` written in one parsed file, in source order.
pub fn deferred(file: &ParsedFile) -> Vec<Deferred> {
    let mut out = Vec::new();
    visit(file.tree.root_node(), &mut |node| {
        let kind = match node.kind() {
            "defer_statement" => DeferKind::Always,
            "errdefer_statement" => DeferKind::OnError,
            _ => return,
        };
        // The statement holds the keyword and then the expression.
        let mut cursor = node.walk();
        let expression = node
            .children(&mut cursor)
            .find(|child| !matches!(child.kind(), "defer" | "errdefer" | "|"))
            .and_then(|child| text(child, &file.source))
            .unwrap_or_default();
        out.push(Deferred {
            path: file.path.clone(),
            kind,
            expression: expression.trim_end_matches(';').trim().to_owned(),
            span: Span::of(node),
        });
    });
    out
}

/// Every `@fieldParentPtr` written in one parsed file, in source order.
pub fn parent_recoveries(file: &ParsedFile) -> Vec<ParentRecovery> {
    let mut out = Vec::new();
    visit(file.tree.root_node(), &mut |node| {
        if node.kind() != "builtin_function" {
            return;
        }
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        let named = children
            .first()
            .and_then(|child| text(*child, &file.source))
            .unwrap_or_default();
        if named != "@fieldParentPtr" {
            return;
        }
        let arguments: Vec<String> = children
            .iter()
            .find(|child| child.kind() == "arguments")
            .map(|list| {
                let mut inner = list.walk();
                list.children(&mut inner)
                    .filter(|child| !matches!(child.kind(), "(" | ")" | ","))
                    .filter_map(|child| text(child, &file.source))
                    .collect()
            })
            .unwrap_or_default();
        // Zig 0.14 dropped the leading type argument, so the field name is the
        // first argument in current source and the second in older source.
        // Taking the first string literal reads both.
        let field = arguments
            .iter()
            .find(|argument| argument.starts_with('"'))
            .map(|argument| argument.trim_matches('"').to_owned())
            .unwrap_or_default();
        let value = arguments.last().cloned().unwrap_or_default();
        out.push(ParentRecovery {
            path: file.path.clone(),
            field,
            value,
            span: Span::of(node),
        });
    });
    out
}

fn visit(node: Node<'_>, act: &mut impl FnMut(Node<'_>)) {
    act(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, act);
    }
}
