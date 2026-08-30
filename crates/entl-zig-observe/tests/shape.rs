// Tests for `src/shape.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_zig_observe::*;

#[test]
fn a_written_type_is_read_for_shape_not_for_stars() {
    assert_eq!(TypeShape::of("*Foo").pointer, PointerShape::Single);
    assert_eq!(
        TypeShape::of("*const Foo").pointer,
        PointerShape::SingleConst
    );
    assert_eq!(TypeShape::of("[*]u8").pointer, PointerShape::Many);
    assert_eq!(TypeShape::of("[*c]u8").pointer, PointerShape::Many);
    assert_eq!(TypeShape::of("[]const u8").pointer, PointerShape::Slice);
    assert_eq!(TypeShape::of("Foo").pointer, PointerShape::None);
    assert_eq!(TypeShape::of("u32").pointer, PointerShape::None);
}

#[test]
fn an_optional_pointer_is_both() {
    let shape = TypeShape::of("?*Thing");
    assert!(shape.optional);
    assert_eq!(shape.pointer, PointerShape::Single);
    assert!(shape.is_pointer());
}

/// The distinction the substring test throws away.
#[test]
fn a_const_pointer_cannot_be_freed_through() {
    assert!(TypeShape::of("*Foo").pointer.can_free_through());
    assert!(!TypeShape::of("*const Foo").pointer.can_free_through());
    assert!(!TypeShape::of("Foo").pointer.can_free_through());
}

#[test]
fn the_pointee_survives_every_marker() {
    assert_eq!(
        TypeShape::pointee("?*const jsc.JSGlobalObject"),
        "jsc.JSGlobalObject"
    );
    assert_eq!(TypeShape::pointee("[*]u8"), "u8");
    assert_eq!(TypeShape::pointee("Foo"), "Foo");
}
