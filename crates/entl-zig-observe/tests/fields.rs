#![allow(clippy::unwrap_used, clippy::expect_used)]
//! What the observer reports for Zig it is likely to meet.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{LoadedParser, ParsedFile, ParserPack, ParserRuntime};
use entl_zig_observe::{ContainerKind, fields};

fn parser() -> LoadedParser {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../parser-packs/zig");
    ParserRuntime::new()
        .unwrap()
        .load(Arc::new(ParserPack::load(path).unwrap()))
        .unwrap()
}

fn parse(parser: &LoadedParser, source: &str) -> ParsedFile {
    parser
        .parse("t.zig", Arc::<[u8]>::from(source.as_bytes()))
        .unwrap()
}

#[test]
fn reports_name_type_and_container_kind() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const Foo = extern struct {\n    next: ?*Foo,\n    data: [*c]u8,\n};\n",
    );
    let observed = fields(&file);
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0].name, "next");
    assert_eq!(observed[0].zig_type, "?*Foo");
    assert_eq!(observed[0].container, "Foo");
    assert_eq!(observed[0].container_kind, ContainerKind::ExternStruct);
    assert!(observed[0].container_kind.is_extern());
    assert_eq!(observed[1].zig_type, "[*c]u8");
}

/// The grammar parses `*jsc.VirtualMachine` as a field expression over
/// `pointer_type [*jsc]`. The observation must be what the author wrote.
#[test]
fn qualified_pointer_type_is_reported_as_written() {
    let parser = parser();
    let file = parse(
        &parser,
        "const S = struct {\n    vm: *jsc.VirtualMachine,\n    log: *const logger.Log = undefined,\n};\n",
    );
    let observed = fields(&file);
    assert_eq!(observed[0].zig_type, "*jsc.VirtualMachine");
    // A default value is not part of the type.
    assert_eq!(observed[1].zig_type, "*const logger.Log");
}

/// The reason `(file stem, field name)` is not a key.
#[test]
fn nested_containers_get_a_dotted_path() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const Outer = struct {\n    ctx: *Outer,\n    pub const Inner = struct {\n        ctx: *u8,\n    };\n};\n",
    );
    let observed = fields(&file);
    assert_eq!(observed.len(), 2);
    let mut seen: Vec<(&str, &str)> = observed
        .iter()
        .map(|field| (field.container.as_str(), field.name.as_str()))
        .collect();
    seen.sort_unstable();
    assert_eq!(seen, vec![("Outer", "ctx"), ("Outer.Inner", "ctx")]);
}

#[test]
fn spans_are_one_based_and_bracket_the_declaration() {
    let parser = parser();
    let source = "const S = struct {\n    ptr: *u8,\n};\n";
    let file = parse(&parser, source);
    let observed = fields(&file);
    let field = &observed[0];
    assert_eq!(field.span.start_line, 2);
    assert_eq!(
        &source[field.span.start_byte..field.span.end_byte],
        "ptr: *u8"
    );
    assert_eq!(
        &source[field.type_span.start_byte..field.type_span.end_byte],
        "*u8"
    );
}

/// Enum members carry no type annotation, so there is nothing to report.
#[test]
fn untyped_members_are_not_fields() {
    let parser = parser();
    let file = parse(&parser, "const E = enum { a, b, c };\n");
    assert!(fields(&file).is_empty());
}

#[test]
fn packed_and_union_containers_are_distinguished() {
    let parser = parser();
    let file = parse(
        &parser,
        "const A = packed struct { a: u1 };\nconst B = extern union { b: *u8 };\n",
    );
    let observed = fields(&file);
    assert_eq!(observed[0].container_kind, ContainerKind::PackedStruct);
    assert_eq!(observed[1].container_kind, ContainerKind::ExternUnion);
    assert!(observed[1].container_kind.is_extern());
}

#[test]
fn comptime_fields_are_flagged_not_dropped() {
    let parser = parser();
    let file = parse(&parser, "const S = struct { comptime n: u32 = 0 };\n");
    let observed = fields(&file);
    assert_eq!(observed.len(), 1);
    assert!(observed[0].comptime);
}

/// A Zig file is a struct. Bun relies on this constantly: `HotReloadEvent.zig`
/// declares `owner: *DevServer,` at the top level with no `struct` keyword.
#[test]
fn top_level_fields_belong_to_the_file() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const HotReloadEvent = @This();\n\nowner: *DevServer,\ncount: u32 = 0,\n",
    );
    let observed = fields(&file);
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0].name, "owner");
    assert_eq!(observed[0].zig_type, "*DevServer");
    // Named by `@This()`, not by the placeholder path the test parsed under.
    assert_eq!(observed[0].container, "HotReloadEvent");
}

/// With no `@This()` the file still has fields; it just falls back to the stem.
#[test]
fn a_file_without_this_falls_back_to_its_stem() {
    let parser = parser();
    let file = parse(&parser, "ptr: *u8,\n");
    let observed = fields(&file);
    assert_eq!(observed[0].container, "t");
}

/// A file with no `@This()` is a namespace, not a type. `const FilePoll =
/// struct` in `posix_event_loop.zig` is `FilePoll`, not
/// `posix_event_loop.FilePoll`, because nothing claims the file is a type.
#[test]
fn a_namespace_file_does_not_prefix_what_it_declares() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const FilePoll = struct { next: ?*FilePoll };\n",
    );
    let observed = fields(&file);
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].container, "FilePoll");
}

/// Even in a file that declares itself a struct, a nested declaration names
/// itself. The file's own fields take the file's name; `HTML` does not become
/// `RouteBundle.HTML`. See the note in `fields` for why, and what it costs.
#[test]
fn a_declaration_names_itself_even_inside_a_struct_file() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const RouteBundle = @This();\nself_field: *u8,\npub const HTML = struct { cached: *u8 };\n",
    );
    let observed = fields(&file);
    let mut seen: Vec<(&str, &str)> = observed
        .iter()
        .map(|f| (f.container.as_str(), f.name.as_str()))
        .collect();
    seen.sort_unstable();
    assert_eq!(
        seen,
        vec![("HTML", "cached"), ("RouteBundle", "self_field")]
    );
}

/// Several functions in one file each declare their own `Closure`.
#[test]
fn containers_declared_in_a_function_carry_the_function_name() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const Pattern = struct {\n    pub fn writeToString() void {\n        const Closure = struct { res: *u8 };\n        _ = Closure;\n    }\n};\n",
    );
    let observed = fields(&file);
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].container, "Pattern.writeToString.Closure");
}

/// An inline container is named by the field that holds it.
#[test]
fn an_inline_container_is_named_by_its_field() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const DevServer = @This();\ncurrent_bundle: ?struct { bv2: *BundleV2 },\n",
    );
    let observed = fields(&file);
    let inner = observed
        .iter()
        .find(|f| f.name == "bv2")
        .expect("the inline container's field");
    assert_eq!(inner.container, "DevServer.current_bundle");
}
