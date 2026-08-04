#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Where a field is assigned, and what is called on it.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{LoadedParser, ParsedFile, ParserPack, ParserRuntime};
use entl_zig_observe::{AssignmentForm, assignments, method_calls};

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

/// The grammar reads `self.field = value;` as a `variable_declaration`. A
/// consumer should not have to know that.
#[test]
fn a_statement_assignment_is_reported() {
    let parser = parser();
    let file = parse(
        &parser,
        "fn init(self: *Foo) void {\n    self.gradient = bun.create(gpa, Gradient, .{});\n}\n",
    );
    let observed = assignments(&file);
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].receiver, "self");
    assert_eq!(observed[0].field, "gradient");
    assert_eq!(observed[0].value, "bun.create(gpa, Gradient, .{})");
    assert_eq!(observed[0].form, AssignmentForm::Statement);
}

/// `.field = value` inside an initialiser names no receiver.
#[test]
fn an_initialiser_assignment_is_reported_with_no_receiver() {
    let parser = parser();
    let file = parse(
        &parser,
        "fn make() Foo {\n    return .{ .vm = vm, .next = null };\n}\n",
    );
    let observed = assignments(&file);
    let fields: Vec<(&str, &str)> = observed
        .iter()
        .map(|a| (a.field.as_str(), a.value.as_str()))
        .collect();
    assert_eq!(fields, vec![("vm", "vm"), ("next", "null")]);
    assert!(observed.iter().all(|a| a.receiver.is_empty()));
    assert_eq!(observed[0].form, AssignmentForm::Initializer);
}

/// The arena cue: a value that goes into a store freed in bulk.
#[test]
fn a_call_valued_assignment_keeps_the_whole_expression() {
    let parser = parser();
    let file = parse(
        &parser,
        "fn make() Foo {\n    return .{ .data = Data.Store.append(x) };\n}\n",
    );
    let observed = assignments(&file);
    assert_eq!(observed[0].field, "data");
    assert_eq!(observed[0].value, "Data.Store.append(x)");
}

#[test]
fn spans_bracket_the_assignment_and_its_value() {
    let parser = parser();
    let source = "fn f(self: *Foo) void {\n    self.a = b;\n}\n";
    let file = parse(&parser, source);
    let assignment = &assignments(&file)[0];
    assert_eq!(assignment.span.start_line, 2);
    assert_eq!(
        &source[assignment.value_span.start_byte..assignment.value_span.end_byte],
        "b"
    );
}

/// The refcount cue: `self.str.deref()` releases a reference `str` holds, and
/// nothing in `str`'s declaration says so.
#[test]
fn a_method_call_names_its_receiver_and_the_field_it_ends_in() {
    let parser = parser();
    let file = parse(
        &parser,
        "fn deinit(self: *Foo) void {\n    self.str.deref();\n}\n",
    );
    let calls = method_calls(&file);
    let deref = calls
        .iter()
        .find(|call| call.method == "deref")
        .expect("the deref call");
    assert_eq!(deref.receiver, "self.str");
    assert_eq!(deref.receiver_tail, "str");
}

/// A plain function call has no receiver and is not a method call on a field.
#[test]
fn a_bare_call_is_not_reported_as_a_method_call() {
    let parser = parser();
    let file = parse(&parser, "fn f() void {\n    doThing(1);\n}\n");
    assert!(method_calls(&file).is_empty());
}

#[test]
fn a_declaration_that_is_not_an_assignment_to_a_field_is_ignored() {
    let parser = parser();
    let file = parse(&parser, "fn f() void {\n    const x = 1;\n}\n");
    assert!(assignments(&file).is_empty());
}
