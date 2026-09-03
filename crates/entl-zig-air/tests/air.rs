// Tests for `src/air.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use entl_zig_air::air::*;

const DUMP: &str = "\
# Begin Function AIR: sub.holder.Holder.init:
# Total AIR+Liveness bytes: 624B
# AIR Instructions:         32 (288B)
  %0 = arg(mem.Allocator, 0)
  %6!= dbg_stmt(2:35)
  %8 = struct_field_ptr_index_0(**sub.holder.Inner, %7!)
  %17 = try(%10, {
    %11 = unwrap_errunion_err(error{OutOfMemory}, %10!)
  } %10!)
  %21!= store_safe(%8!, %20!)
# End Function AIR: sub.holder.Holder.init
";

fn read(dump: &str) -> (Vec<Function>, Tally) {
    let mut reader = Reader::new();
    let mut out = Vec::new();
    for line in dump.lines() {
        if let Some(function) = reader.push(line) {
            out.push(function);
        }
    }
    if let Some(function) = reader.finish() {
        out.push(function);
    }
    (out, *reader.tally())
}

#[test]
fn a_function_is_read_with_its_header() {
    let (functions, _) = read(DUMP);
    assert_eq!(functions.len(), 1);
    assert_eq!(functions[0].mangled, "sub.holder.Holder.init");
    assert_eq!(functions[0].declared_instructions, 32);
    assert_eq!(functions[0].air_bytes, 624);
}

/// The resolved type is the whole reason for reading AIR at all.
#[test]
fn a_field_pointer_carries_its_resolved_type() {
    let (functions, _) = read(DUMP);
    let field = functions[0]
        .instructions
        .iter()
        .find(|instruction| instruction.op == "struct_field_ptr_index_0")
        .expect("the field pointer");
    assert_eq!(field.result_type, "**sub.holder.Inner");
    assert_eq!(field.operands, vec![7]);
}

#[test]
fn a_dead_result_is_marked_and_still_read() {
    let (functions, _) = read(DUMP);
    let store = functions[0]
        .instructions
        .iter()
        .find(|instruction| instruction.op == "store_safe")
        .expect("the store");
    assert!(store.dead);
    assert_eq!(store.operands, vec![8, 20]);
    assert!(store.result_type.is_empty(), "a store names no type");
}

/// `dbg_stmt` positions the instructions after it, not itself.
#[test]
fn a_position_applies_to_what_follows_it() {
    let (functions, _) = read(DUMP);
    let by_op = |op: &str| {
        functions[0]
            .instructions
            .iter()
            .find(|instruction| instruction.op == op)
            .cloned()
            .unwrap()
    };
    assert_eq!(by_op("arg").line, 0, "nothing positions the first argument");
    assert_eq!(by_op("struct_field_ptr_index_0").line, 2);
    assert_eq!(by_op("struct_field_ptr_index_0").column, 35);
}

#[test]
fn an_instruction_inside_a_block_is_read_at_depth() {
    let (functions, _) = read(DUMP);
    let nested = functions[0]
        .instructions
        .iter()
        .find(|instruction| instruction.op == "unwrap_errunion_err")
        .expect("the nested instruction");
    assert_eq!(nested.depth, 1);
    assert_eq!(nested.index, 11);
}

/// Every line lands in exactly one bucket, and nothing that looked like an
/// instruction failed to parse.
#[test]
fn every_line_is_accounted_for() {
    let (_, tally) = read(DUMP);
    assert!(tally.balances(), "{tally:?}");
    assert_eq!(tally.unparsed, 0, "{tally:?}");
    // arg, dbg_stmt, struct_field_ptr_index_0, try, the nested
    // unwrap_errunion_err, store_safe. The `} %10!)` is the delimiter.
    assert_eq!(tally.instructions, 6);
    assert_eq!(tally.delimiters, 1);
}

/// Liveness lists sit exactly where an instruction would.
#[test]
fn a_death_list_is_not_an_instruction() {
    assert!(is_death_list("%1!"));
    assert!(is_death_list("%1! %2! %3!"));
    assert!(!is_death_list("%1 = arg(u8, 0)"));
    assert!(!is_death_list("%1"));
    assert!(!is_death_list(""));

    let dump = "# Begin Function AIR: f:\n  %0 = arg(u8, 0)\n  %3 = block(void, {\n    %1! %2!\n  } %0!)\n# End Function AIR: f\n";
    let (functions, tally) = read(dump);
    assert_eq!(tally.deaths, 1);
    assert_eq!(tally.unparsed, 0);
    assert!(tally.balances(), "{tally:?}");
    assert_eq!(functions[0].instructions.len(), 2);
}

#[test]
fn a_comma_inside_a_type_does_not_split_the_arguments() {
    let arguments = "<fn (mem.Allocator, *air.Child) void, (function 'destroy')>, [%7!, %11!]";
    assert_eq!(split_top_level(arguments).len(), 2);
    assert_eq!(operands_of(arguments), vec![7, 11]);
}

/// A store's operands are (destination, value) and sorting them swaps the
/// two. This is the difference between "the field holds an allocation" and
/// "an allocation holds the field".
#[test]
fn operand_order_is_preserved() {
    assert_eq!(operands_of("%4!, %0!"), vec![4, 0]);
    assert_eq!(operands_of("%0!, %4!"), vec![0, 4]);
    // repeats are meaning too: `%3` used twice is two reads
    assert_eq!(operands_of("%3!, %3!"), vec![3, 3]);
}
