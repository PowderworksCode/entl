//! Zig container fields, as written, with spans.
//!
//! Every field a Zig container declares: the name it is given, the type
//! expression it is annotated with, the container that declares it, and where
//! in the file that happened. It is one walk over a parsed tree and it decides
//! nothing.
//!
//! It exists because Zig spells every pointer `*Foo` whether the container owns
//! the memory or borrows it, and a consumer deciding which is which needs the
//! declaration's own context to do it: whether the container is `extern` (so
//! the field is laid out to match a C declaration rather than by Zig), which
//! container in a nest of them declares the field, and the type exactly as the
//! author wrote it. Those are observations. Which ownership class they imply is
//! not, and does not belong here.
//!
//! ## Two things the grammar does that a consumer should not have to know
//!
//! `*jsc.VirtualMachine` does not parse as a pointer to a qualified name. It
//! parses as a `field_expression` whose left side is `pointer_type [*jsc]`, so
//! reading the tree structurally gives a pointer to `jsc`. [`ContainerField`]
//! therefore carries `zig_type` as the **source text of the type node**, which
//! is what the author wrote and is unaffected by how the grammar grouped it.
//!
//! Container declarations nest, and the same field name recurs across the
//! containers in one file — Bun has files where a name like `ctx` is declared
//! by three different structs with three different ownerships. So
//! [`ContainerField::container`] is the dotted path of enclosing declarations
//! rather than the innermost name, and `(path, container, name)` is a key where
//! `(file stem, name)` is not.

use std::path::{Path, PathBuf};

use entl_tree_sitter::ParsedFile;
use tree_sitter::Node;

/// Where an observation was made, in bytes and in lines.
///
/// Both, because a byte range is what a later pass slices with and a line is
/// what a person checks against the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    /// One-based, to match what an editor and `file:line` evidence both mean.
    pub start_line: usize,
    pub end_line: usize,
}

impl Span {
    fn of(node: Node<'_>) -> Self {
        Span {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
        }
    }
}

/// What kind of container declared a field.
///
/// `extern` and `packed` are kept apart from plain `struct` because they are
/// the layout-bearing cases: an `extern struct` mirrors a declaration from
/// outside Zig, which tells a consumer something about the field that the
/// field's own type does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContainerKind {
    Struct,
    ExternStruct,
    PackedStruct,
    Union,
    ExternUnion,
    PackedUnion,
    Enum,
    Opaque,
}

impl ContainerKind {
    /// The declaration was laid out to match something outside Zig.
    pub fn is_extern(self) -> bool {
        matches!(
            self,
            ContainerKind::ExternStruct | ContainerKind::ExternUnion
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            ContainerKind::Struct => "struct",
            ContainerKind::ExternStruct => "extern struct",
            ContainerKind::PackedStruct => "packed struct",
            ContainerKind::Union => "union",
            ContainerKind::ExternUnion => "extern union",
            ContainerKind::PackedUnion => "packed union",
            ContainerKind::Enum => "enum",
            ContainerKind::Opaque => "opaque",
        }
    }

    fn of(node: Node<'_>) -> Option<Self> {
        let mut cursor = node.walk();
        let modifier = node
            .children(&mut cursor)
            .find_map(|child| match child.kind() {
                "extern" => Some("extern"),
                "packed" => Some("packed"),
                _ => None,
            });
        Some(match (node.kind(), modifier) {
            ("struct_declaration", Some("extern")) => ContainerKind::ExternStruct,
            ("struct_declaration", Some("packed")) => ContainerKind::PackedStruct,
            ("struct_declaration", _) => ContainerKind::Struct,
            ("union_declaration", Some("extern")) => ContainerKind::ExternUnion,
            ("union_declaration", Some("packed")) => ContainerKind::PackedUnion,
            ("union_declaration", _) => ContainerKind::Union,
            ("enum_declaration", _) => ContainerKind::Enum,
            ("opaque_declaration", _) => ContainerKind::Opaque,
            _ => return None,
        })
    }
}

/// Is this node a container declaration?
fn is_container(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "struct_declaration" | "union_declaration" | "enum_declaration" | "opaque_declaration"
    )
}

/// One field, as the source declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerField {
    /// The file the declaration was read from, as the parse reported it.
    pub path: PathBuf,
    /// Dotted path of enclosing declarations, outermost first: `Foo.Inner`.
    /// Empty when every enclosing container is anonymous.
    pub container: String,
    pub container_kind: ContainerKind,
    pub name: String,
    /// The type expression as written. See the note on the grammar above.
    pub zig_type: String,
    /// The whole `name: Type` declaration.
    pub span: Span,
    /// Just the type expression, for a consumer that wants to point at it.
    pub type_span: Span,
    /// The field is declared `comptime`.
    pub comptime: bool,
}

/// Every field declared anywhere in one parsed Zig file.
///
/// Order is the order of declaration, which is stable for a given source and is
/// what a reader diffing against the file expects.
///
/// **A Zig file is itself a struct.** Fields written at the top level of a file
/// belong to it, and Bun uses that constantly — `HotReloadEvent.zig` opens with
/// `pub const HotReloadEvent = @This();` and then declares `owner: *DevServer,`
/// with no `struct` keyword anywhere. Those are reported, under the name the
/// file gives itself via `@This()` or, failing that, the file stem.
pub fn fields(file: &ParsedFile) -> Vec<ContainerField> {
    let root = file.tree.root_node();
    let source = &file.source;
    let path = file.path.as_path();
    let mut out = Vec::new();

    // The file's own fields are named by the file: `pub const HotReloadEvent =
    // @This();` then `owner: *DevServer,` is `HotReloadEvent.owner`, and the
    // stem stands in when a file makes no such claim.
    let name = declared_self(root, source).unwrap_or_else(|| {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    emit_fields(
        root,
        ContainerKind::Struct,
        source,
        path,
        std::slice::from_ref(&name),
        &mut out,
    );

    // What the file *declares* names itself. `const FilePoll = struct` is
    // `FilePoll`, not `posix_event_loop.FilePoll`, which is both how the source
    // refers to it and how Bun's own classification of 2,252 fields records it
    // — unprefixed in 700 rows against 77 prefixed. A Zig file is technically a
    // struct and so the prefixed path is arguably the truer one; it is not the
    // one anybody keys on.
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "variable_declaration"
            && let Some((declared, container)) = bound_container(child, source)
        {
            let mut scope = vec![declared];
            emit(container, source, path, &scope, &mut out);
            walk(container, source, path, &mut scope, &mut out);
            continue;
        }
        visit(child, source, path, &mut vec![name.clone()], &mut out);
    }
    out
}

/// The name a file gives itself with `pub const Foo = @This();`.
///
/// Its presence is the signal that the file is a type rather than a namespace,
/// so the answer is an `Option` and the absence is meaningful.
fn declared_self(root: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() != "variable_declaration" {
            continue;
        }
        let Some(text) = child.utf8_text(source).ok() else {
            continue;
        };
        if !text.contains("@This()") {
            continue;
        }
        let mut inner = child.walk();
        let children: Vec<Node<'_>> = child.children(&mut inner).collect();
        let Some(assign) = children.iter().position(|part| part.kind() == "=") else {
            continue;
        };
        if let Some(name) = children
            .iter()
            .take(assign)
            .rev()
            .find(|part| part.kind() == "identifier")
            .and_then(|part| part.utf8_text(source).ok())
        {
            return Some(name.to_string());
        }
    }
    None
}

fn text(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(str::to_string)
}

/// The name and container a `variable_declaration` binds, when it binds one.
fn bound_container<'tree>(node: Node<'tree>, source: &[u8]) -> Option<(String, Node<'tree>)> {
    let mut cursor = node.walk();
    let children: Vec<Node<'tree>> = node.children(&mut cursor).collect();
    let container = children
        .iter()
        .copied()
        .find(|child| is_container(*child))?;
    // The identifier the declaration binds is the last one before the `=`.
    let assign = children.iter().position(|child| child.kind() == "=")?;
    let name = children
        .iter()
        .take(assign)
        .rev()
        .find(|child| child.kind() == "identifier")?;
    Some((text(*name, source)?, container))
}

fn walk(
    node: Node<'_>,
    source: &[u8],
    path: &Path,
    scope: &mut Vec<String>,
    out: &mut Vec<ContainerField>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit(child, source, path, scope, out);
    }
}

/// What to do with one node, wherever it was found.
///
/// Shared by the file root and by every level below it, so a field written at
/// the top of a file is treated exactly like the same field written inside a
/// `struct`.
fn visit(
    node: Node<'_>,
    source: &[u8],
    path: &Path,
    scope: &mut Vec<String>,
    out: &mut Vec<ContainerField>,
) {
    match node.kind() {
        "variable_declaration" => match bound_container(node, source) {
            Some((name, container)) => {
                scope.push(name);
                emit(container, source, path, scope, out);
                walk(container, source, path, scope, out);
                scope.pop();
            }
            // Not a container binding, but a container can still be written
            // inside the initialiser, so keep descending.
            None => walk(node, source, path, scope, out),
        },
        // A container declared inside a function body belongs to that function:
        // Bun writes a `Closure` or `Context` struct per callback and several
        // functions in one file each declare their own.
        "function_declaration" => match declared_name(node, source) {
            Some(name) => {
                scope.push(name);
                walk(node, source, path, scope, out);
                scope.pop();
            }
            None => walk(node, source, path, scope, out),
        },
        // A field whose type is written inline. The anonymous container is
        // named by the field that holds it, which is the only name it has.
        "container_field" => match field_with_inline_container(node, source) {
            Some((name, container)) => {
                scope.push(name);
                emit(container, source, path, scope, out);
                walk(container, source, path, scope, out);
                scope.pop();
            }
            None => walk(node, source, path, scope, out),
        },
        _ if is_container(node) => {
            // Anonymous and not a field's type: a return type, a parameter.
            emit(node, source, path, scope, out);
            walk(node, source, path, scope, out);
        }
        _ => walk(node, source, path, scope, out),
    }
}

/// The identifier a `fn` declaration names.
fn declared_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == "identifier")
        .and_then(|child| text(child, source))
}

/// A `container_field` whose type is an inline container, and that field's name.
fn field_with_inline_container<'tree>(
    node: Node<'tree>,
    source: &[u8],
) -> Option<(String, Node<'tree>)> {
    let mut cursor = node.walk();
    let parts: Vec<Node<'tree>> = node.children(&mut cursor).collect();
    let colon = parts.iter().position(|part| part.kind() == ":")?;
    // The container can sit under a wrapper: `?struct { .. }` puts it inside a
    // `nullable_type`, and `[]const struct { .. }` deeper still.
    let container = parts
        .iter()
        .skip(colon)
        .find_map(|part| first_container(*part))?;
    let name = parts
        .iter()
        .take(colon)
        .rev()
        .find(|part| part.kind() == "identifier")?;
    Some((text(*name, source)?, container))
}

/// The outermost container at or under this node, without crossing into a
/// field of one — a nested field's own inline container is that field's, not
/// this one's.
fn first_container<'tree>(node: Node<'tree>) -> Option<Node<'tree>> {
    if is_container(node) {
        return Some(node);
    }
    if node.kind() == "container_field" {
        return None;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).find_map(first_container)
}

/// Fields declared directly by this container, not by one nested inside it.
fn emit(
    container: Node<'_>,
    source: &[u8],
    path: &Path,
    scope: &[String],
    out: &mut Vec<ContainerField>,
) {
    let Some(kind) = ContainerKind::of(container) else {
        return;
    };
    emit_fields(container, kind, source, path, scope, out);
}

/// As [`emit`], for a container whose kind the caller already knows.
///
/// Separate because the file node carries no `struct` keyword to read a kind
/// from: a Zig file is a plain struct by definition, not by declaration.
fn emit_fields(
    container: Node<'_>,
    kind: ContainerKind,
    source: &[u8],
    path: &Path,
    scope: &[String],
    out: &mut Vec<ContainerField>,
) {
    let mut cursor = container.walk();
    for child in container.children(&mut cursor) {
        if child.kind() != "container_field" {
            continue;
        }
        let mut inner = child.walk();
        let parts: Vec<Node<'_>> = child.children(&mut inner).collect();
        let comptime = parts.iter().any(|part| part.kind() == "comptime");
        let Some(colon) = parts.iter().position(|part| part.kind() == ":") else {
            // `enum` members and untyped fields carry no annotation. There is
            // nothing to observe about a type that was not written.
            continue;
        };
        let Some(name) = parts
            .iter()
            .take(colon)
            .rev()
            .find(|part| part.kind() == "identifier")
            .and_then(|part| text(*part, source))
        else {
            continue;
        };
        let Some(type_node) = parts.get(colon + 1) else {
            continue;
        };
        let Some(zig_type) = text(*type_node, source) else {
            continue;
        };
        out.push(ContainerField {
            path: path.to_path_buf(),
            container: scope.join("."),
            container_kind: kind,
            name,
            zig_type,
            span: Span::of(child),
            type_span: Span::of(*type_node),
            comptime,
        });
    }
}
