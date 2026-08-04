//! Functions, their parameters, the calls they make, and what they return.
//!
//! A field's declaration says what it points at and never says where the
//! pointer came from. That answer is spread across three places: the parameter
//! a constructor was handed, the argument a caller passed it, and the value a
//! function returned. Observing all three is what lets a later pass join them
//! into one edge — `dev` was a parameter of `init`, the caller passed
//! `self.dev`, so the field borrows something the caller already owned.
//!
//! Naming follows the convention [`crate::fields`] uses: a function is named by
//! the container it is written in, so `init` inside `const Foo = struct` is
//! `Foo.init`, and a container declared inside a function carries the
//! function's name in turn.

use std::path::{Path, PathBuf};

use entl_tree_sitter::ParsedFile;
use tree_sitter::Node;

use crate::{Span, bound_container, declared_name, file_scope, is_container, text};

/// One declared parameter, in the order it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    /// The type as written, `*DevServer` and not a resolution of it.
    pub zig_type: String,
    /// Position in the parameter list, counting from zero.
    pub index: usize,
    pub span: Span,
    pub type_span: Span,
}

impl Parameter {
    /// Whether this parameter is a pointer of any shape.
    pub fn is_pointer(&self) -> bool {
        self.zig_type.contains('*')
    }
}

/// A function, with everything about its signature a later pass needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub path: PathBuf,
    /// The container the function is written in, dotted.
    pub container: String,
    pub name: String,
    pub parameters: Vec<Parameter>,
    /// The return type as written, empty when the function declares none.
    pub zig_return: String,
    pub public: bool,
    pub span: Span,
}

impl Function {
    /// The dotted name a call site would use: `Foo.init`.
    pub fn qualified(&self) -> String {
        if self.container.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", self.container, self.name)
        }
    }

    /// The receiver, when the function is a method.
    ///
    /// Zig has no `self` keyword and no method declaration: `a.f(b)` is sugar
    /// for `f(a, b)`, so what makes a function a method is that its **first
    /// parameter has the enclosing container's type**. That is the test used
    /// here, and it is not the obvious one.
    ///
    /// The obvious one is to look for a parameter named `self`, which is what
    /// this did first. Across Bun's 1,292 Zig files the first parameter is
    /// named `this` 9,479 times and `self` 2,180 — so the name test found 7.7%
    /// of functions to be methods and missed roughly four in five. It also
    /// disagreed with the source about `deinit`, finding 154 where `fn deinit(`
    /// is written 783 times.
    ///
    /// Matching on the type instead reads `this: *Foo`, `self: *const Foo` and
    /// `ptr: *@This()` alike, and declines `alloc: Allocator` whatever it is
    /// called. It does accept a comparator like `fn order(a: *Foo, b: *Foo)`,
    /// which is not a mistake: `a.order(b)` is how Zig calls it.
    pub fn receiver(&self) -> Option<&Parameter> {
        let own = self.container.rsplit('.').next().unwrap_or_default();
        self.parameters.first().filter(|parameter| {
            let declared = parameter.zig_type.trim_start_matches(['?', '*', '[', ']']);
            let declared = declared.trim_start_matches("const ").trim();
            declared == "@This()" || (!own.is_empty() && declared == own)
        })
    }

    /// Whether this looks like the container's own destructor.
    ///
    /// The container's destructor is where a field it owns gets freed, so this
    /// is the question the `OWNED` rule turns on: an allocation reaching a
    /// field means nothing until something frees it here.
    pub fn is_deinit(&self) -> bool {
        matches!(
            self.name.as_str(),
            "deinit" | "deinitAndFree" | "destroy" | "finalize"
        ) && self.receiver().is_some()
    }
}

/// One argument at a call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    pub index: usize,
    /// The expression as written.
    pub text: String,
    pub span: Span,
}

/// A call, with its arguments positioned so they can be matched to parameters.
///
/// This is deliberately separate from [`crate::MethodCall`], which answers "was
/// anything called on this field" and needs no arguments. Joining an argument
/// to a parameter needs both ends and their positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    pub path: PathBuf,
    /// The callee as written: `alloc.create`, `Watcher.init`, `bun.new`.
    pub callee: String,
    pub arguments: Vec<Argument>,
    /// The function the call is written inside, dotted, or empty at file scope.
    pub enclosing: String,
    pub span: Span,
}

/// A local binding, with whatever it says about its own type.
///
/// Locals are how a value gets from where it was made to where it is stored:
/// `const c = try alloc.create(Child); self.child = c;` puts an allocation and
/// the field that keeps it in two different statements, and nothing links them
/// except the name. They are also how a field of *another* container is
/// reached — `ctx.dev = dev` says nothing until `ctx` has a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub path: PathBuf,
    /// The function the binding is written in, dotted.
    pub function: String,
    pub name: String,
    /// The declared type, when one is written: `var node: Node = ..`.
    pub zig_type: String,
    /// The initialising expression as written, empty when there is none.
    pub value: String,
    /// `var` rather than `const`.
    pub mutable: bool,
    pub span: Span,
}

impl Local {
    /// The container this local holds, when anything says so.
    ///
    /// The declared type first, then the type a named initialiser constructs:
    /// `const ctx = Ctx{ .. }` has no annotation and still says `Ctx`.
    pub fn container(&self) -> Option<&str> {
        if !self.zig_type.is_empty() {
            return Some(strip_markers(&self.zig_type));
        }
        let (head, _) = self.value.split_once('{')?;
        let head = head.trim();
        (!head.is_empty()
            && head
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '.'))
        .then_some(head)
    }
}

/// A type with its pointer, optional and const markers removed.
fn strip_markers(written: &str) -> &str {
    written
        .trim()
        .trim_start_matches(['?', '!'])
        .trim_start()
        .trim_start_matches("[*c]")
        .trim_start_matches("[*]")
        .trim_start_matches("[]")
        .trim_start_matches('*')
        .trim_start()
        .trim_start_matches("const ")
        .trim()
}

/// A value handed back to a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnSite {
    pub path: PathBuf,
    /// The function returning, dotted.
    pub function: String,
    /// The returned expression as written, empty for a bare `return`.
    pub value: String,
    pub span: Span,
}

/// Every function declared anywhere in one parsed Zig file.
pub fn functions(file: &ParsedFile) -> Vec<Function> {
    let mut out = Vec::new();
    walk(file, &mut |node, scope, source, path| {
        if node.kind() != "function_declaration" {
            return;
        }
        if let Some(function) = read_function(node, source, path, scope) {
            out.push(function);
        }
    });
    out
}

/// Every call written in one parsed Zig file, with its arguments.
pub fn call_sites(file: &ParsedFile) -> Vec<CallSite> {
    let mut out = Vec::new();
    walk(file, &mut |node, scope, source, path| {
        if node.kind() != "call_expression" {
            return;
        }
        let Some(callee) = node.child(0).and_then(|child| text(child, source)) else {
            return;
        };
        out.push(CallSite {
            path: path.to_path_buf(),
            callee: callee.split_whitespace().collect(),
            arguments: read_arguments(node, source),
            enclosing: scope.join("."),
            span: Span::of(node),
        });
    });
    out
}

/// Every local binding written inside a function in one parsed Zig file.
///
/// A `const Foo = struct { .. }` is spelled exactly like a local binding and is
/// a type declaration, so bindings whose value is a container are left out —
/// they are already reported by [`crate::fields`] as the containers they are.
pub fn locals(file: &ParsedFile) -> Vec<Local> {
    let mut out = Vec::new();
    walk(file, &mut |node, scope, source, path| {
        if node.kind() != "variable_declaration" {
            return;
        }
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
        let mutable = match children.first().map(Node::kind) {
            Some("var") => true,
            Some("const") => false,
            // `self.field = value` is a `variable_declaration` too, and binds
            // nothing.
            _ => return,
        };
        let Some(name) = children.get(1).and_then(|child| text(*child, source)) else {
            return;
        };
        let value = children
            .iter()
            .position(|child| child.kind() == "=")
            .and_then(|at| children.get(at + 1))
            .copied();
        if value.is_some_and(is_container) {
            return;
        }
        out.push(Local {
            path: path.to_path_buf(),
            function: scope.join("."),
            name,
            zig_type: node
                .child_by_field_name("type")
                .and_then(|child| text(child, source))
                .unwrap_or_default(),
            value: value
                .and_then(|child| text(child, source))
                .unwrap_or_default(),
            mutable,
            span: Span::of(node),
        });
    });
    out
}

/// Every `return` written in one parsed Zig file.
pub fn returns(file: &ParsedFile) -> Vec<ReturnSite> {
    let mut out = Vec::new();
    walk(file, &mut |node, scope, source, path| {
        if node.kind() != "return_expression" {
            return;
        }
        let value = node
            .child(1)
            .and_then(|child| text(child, source))
            .unwrap_or_default();
        out.push(ReturnSite {
            path: path.to_path_buf(),
            function: scope.join("."),
            value,
            span: Span::of(node),
        });
    });
    out
}

/// Read a `function_declaration` into a [`Function`].
fn read_function(node: Node<'_>, source: &[u8], path: &Path, scope: &[String]) -> Option<Function> {
    let name = node
        .child_by_field_name("name")
        .and_then(|child| text(child, source))?;
    // `parameters` is a node kind rather than a named field, unlike `name:` and
    // `type:` on the same declaration.
    let mut cursor = node.walk();
    let parameters = node
        .children(&mut cursor)
        .find(|child| child.kind() == "parameters")
        .map(|list| read_parameters(list, source))
        .unwrap_or_default();
    // The grammar puts the return type in the same `type:` field a parameter
    // uses, as the last one on the declaration itself.
    let zig_return = node
        .child_by_field_name("type")
        .and_then(|child| text(child, source))
        .unwrap_or_default();
    let public = node.child(0).is_some_and(|child| child.kind() == "pub");
    Some(Function {
        path: path.to_path_buf(),
        container: scope.join("."),
        name,
        parameters,
        zig_return,
        public,
        span: Span::of(node),
    })
}

fn read_parameters(list: Node<'_>, source: &[u8]) -> Vec<Parameter> {
    let mut cursor = list.walk();
    list.children(&mut cursor)
        .filter(|child| child.kind() == "parameter")
        .enumerate()
        .filter_map(|(index, child)| {
            let name = child
                .child_by_field_name("name")
                .and_then(|part| text(part, source))?;
            // `anytype` and `comptime T: type` parameters have no written type,
            // and reporting an empty one is truer than inventing a name for it.
            let declared = child.child_by_field_name("type");
            Some(Parameter {
                name,
                zig_type: declared
                    .and_then(|part| text(part, source))
                    .unwrap_or_default(),
                index,
                span: Span::of(child),
                type_span: Span::of(declared.unwrap_or(child)),
            })
        })
        .collect()
}

fn read_arguments(call: Node<'_>, source: &[u8]) -> Vec<Argument> {
    let Some(list) = call.child_by_field_name("arguments") else {
        return Vec::new();
    };
    let mut cursor = list.walk();
    list.children(&mut cursor)
        // the list holds its parentheses and commas too
        .filter(|child| !matches!(child.kind(), "(" | ")" | ","))
        .enumerate()
        .filter_map(|(index, child)| {
            Some(Argument {
                index,
                text: text(child, source)?,
                span: Span::of(child),
            })
        })
        .collect()
}

/// Walk a file, carrying the container scope, and hand each node to `visit`.
///
/// The scope rule is [`crate::fields`]': a declaration names itself, the file
/// names only what is written directly in it, and a function's body is scoped
/// to the function so a container declared in a callback is not mistaken for a
/// sibling of the type that holds it.
fn walk(file: &ParsedFile, visit: &mut impl FnMut(Node<'_>, &[String], &[u8], &Path)) {
    let root = file.tree.root_node();
    let source = &file.source;
    let path = file.path.as_path();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "variable_declaration"
            && let Some((declared, container)) = bound_container(child, source)
        {
            let mut scope = vec![declared];
            descend(container, source, path, &mut scope, visit);
            continue;
        }
        let mut scope = vec![file_scope(root, source, path)];
        descend(child, source, path, &mut scope, visit);
    }
}

fn descend(
    node: Node<'_>,
    source: &[u8],
    path: &Path,
    scope: &mut Vec<String>,
    visit: &mut impl FnMut(Node<'_>, &[String], &[u8], &Path),
) {
    visit(node, scope, source, path);
    // A name pushed here scopes everything below it. The visit above already
    // saw this node under its *enclosing* scope, which is where a function's
    // own container is the one that declares it rather than itself.
    let pushed = match node.kind() {
        "variable_declaration" => bound_container(node, source).map(|(name, _)| name),
        "function_declaration" => declared_name(node, source),
        _ => None,
    };
    let scoped = pushed.is_some();
    if let Some(name) = pushed {
        scope.push(name);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        descend(child, source, path, scope, visit);
    }
    if scoped {
        scope.pop();
    }
}
