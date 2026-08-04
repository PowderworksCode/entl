#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Functions, parameters, call arguments, and returns.

use std::path::PathBuf;
use std::sync::Arc;

use entl_tree_sitter::{LoadedParser, ParsedFile, ParserPack, ParserRuntime};
use entl_zig_observe::{call_sites, functions, returns};

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
fn a_parameter_keeps_its_name_type_and_position() {
    let parser = parser();
    let file = parse(
        &parser,
        "const Foo = struct {\n    pub fn init(alloc: std.mem.Allocator, dev: *DevServer) !*Foo {}\n};\n",
    );
    let observed = functions(&file);
    assert_eq!(observed.len(), 1);
    let function = &observed[0];
    assert_eq!(function.container, "Foo");
    assert_eq!(function.name, "init");
    assert_eq!(function.qualified(), "Foo.init");
    assert!(function.public);
    assert_eq!(function.zig_return, "!*Foo");
    let parameters: Vec<(&str, &str, usize)> = function
        .parameters
        .iter()
        .map(|p| (p.name.as_str(), p.zig_type.as_str(), p.index))
        .collect();
    assert_eq!(
        parameters,
        vec![("alloc", "std.mem.Allocator", 0), ("dev", "*DevServer", 1),]
    );
    assert!(function.parameters[1].is_pointer());
    assert!(!function.parameters[0].is_pointer());
}

/// A receiver is decided by type, not by name. Bun names it `this` four times
/// as often as `self`, so a name test finds one method in five.
#[test]
fn a_receiver_is_the_first_parameter_of_the_containers_own_type() {
    let parser = parser();
    let file = parse(
        &parser,
        "const Foo = struct {\n    fn deinit(this: *Foo) void {}\n    fn peek(self: *const Foo) void {}\n    fn own(ptr: *@This()) void {}\n    fn make(alloc: Allocator) void {}\n};\n",
    );
    let observed = functions(&file);
    let named = |name: &str| {
        observed
            .iter()
            .find(|function| function.name == name)
            .unwrap()
            .clone()
    };
    assert_eq!(
        named("deinit").receiver().map(|p| p.name.clone()),
        Some("this".to_owned())
    );
    assert_eq!(
        named("peek").receiver().map(|p| p.name.clone()),
        Some("self".to_owned())
    );
    assert_eq!(
        named("own").receiver().map(|p| p.name.clone()),
        Some("ptr".to_owned())
    );
    assert_eq!(named("make").receiver(), None);

    assert!(named("deinit").is_deinit());
    assert!(!named("peek").is_deinit());
}

/// `anytype` and `comptime T: type` declare no type this pass can read, and an
/// empty string is truer than a guess.
#[test]
fn a_parameter_with_no_written_type_reports_none() {
    let parser = parser();
    let file = parse(
        &parser,
        "fn log(comptime fmt: []const u8, args: anytype) void {}\n",
    );
    let function = &functions(&file)[0];
    assert_eq!(function.parameters[0].zig_type, "[]const u8");
    assert_eq!(function.parameters[1].name, "args");
}

#[test]
fn call_arguments_keep_their_position() {
    let parser = parser();
    let file = parse(
        &parser,
        "const Foo = struct {\n    fn init(dev: *DevServer) void {\n        register(dev, 3, .{});\n    }\n};\n",
    );
    let call = call_sites(&file)
        .into_iter()
        .find(|call| call.callee == "register")
        .expect("the register call");
    assert_eq!(call.enclosing, "Foo.init");
    let arguments: Vec<(usize, &str)> = call
        .arguments
        .iter()
        .map(|argument| (argument.index, argument.text.as_str()))
        .collect();
    assert_eq!(arguments, vec![(0, "dev"), (1, "3"), (2, ".{}")]);
}

/// The allocation idiom the OWNED rule turns on, seen as a call.
#[test]
fn an_allocator_call_names_its_receiver_and_type() {
    let parser = parser();
    let file = parse(
        &parser,
        "fn make(alloc: std.mem.Allocator) !*Foo {\n    return try alloc.create(Foo);\n}\n",
    );
    let call = call_sites(&file)
        .into_iter()
        .find(|call| call.callee.ends_with("create"))
        .expect("the create call");
    assert_eq!(call.callee, "alloc.create");
    assert_eq!(call.arguments[0].text, "Foo");
}

#[test]
fn a_return_names_the_function_it_leaves() {
    let parser = parser();
    let file = parse(
        &parser,
        "const Foo = struct {\n    fn init() *Foo {\n        return self;\n    }\n};\n",
    );
    let observed = returns(&file);
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].function, "Foo.init");
    assert_eq!(observed[0].value, "self");
}

/// A container declared inside a callback is not a sibling of the type that
/// holds it, so a function inside it must not be attributed to the outer type.
#[test]
fn a_function_inside_a_nested_container_carries_the_whole_path() {
    let parser = parser();
    let file = parse(
        &parser,
        "const Outer = struct {\n    fn run() void {\n        const Ctx = struct {\n            fn go(self: *Ctx) void {}\n        };\n    }\n};\n",
    );
    let go = functions(&file)
        .into_iter()
        .find(|function| function.name == "go")
        .expect("the nested function");
    assert_eq!(go.container, "Outer.run.Ctx");
    assert_eq!(go.qualified(), "Outer.run.Ctx.go");
}

/// A file is a struct, so a function written at its top level belongs to it.
#[test]
fn a_top_level_function_belongs_to_the_file() {
    let parser = parser();
    let file = parse(
        &parser,
        "pub const Watcher = @This();\n\nfn deinit(self: *Watcher) void {}\n",
    );
    let function = &functions(&file)[0];
    assert_eq!(function.container, "Watcher");
    assert_eq!(function.qualified(), "Watcher.deinit");
}
