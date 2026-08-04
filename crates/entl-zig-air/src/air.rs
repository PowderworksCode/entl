//! Reading the Zig compiler's AIR dump.
//!
//! AIR is Zig's analysed intermediate representation: the form the source takes
//! *after* semantic analysis, so every type in it is resolved and every
//! comptime generic is already instantiated. A compiler built with
//! `-Ddebug-extensions=true` writes it on stderr for `--verbose-air`.
//!
//! The dump is a text stream, one function at a time:
//!
//! ```text
//! # Begin Function AIR: sub.holder.Holder.init:
//! # Total AIR+Liveness bytes: 624B
//! # AIR Instructions:         32 (288B)
//!   %0 = arg(mem.Allocator, 0)
//!   %8 = struct_field_ptr_index_0(**sub.holder.Inner, %7!)
//!   %17 = try(%10, {
//!     %11 = unwrap_errunion_err(error{OutOfMemory}, %10!)
//!   } %10!)
//!   %21!= store_safe(%8!, %20!)
//! # End Function AIR: sub.holder.Holder.init
//! ```
//!
//! Three things make it more than a line scan. Instructions nest inside `{ }`
//! blocks, so a line's indentation is its depth and the closing `} %10!)` is
//! not an instruction. A result may be written `%21!=` rather than `%21 =`,
//! where the `!` marks a value nothing reads afterwards. And positions arrive
//! out of band as `dbg_stmt(line:col)`, which applies to the instructions that
//! follow it rather than to itself.
//!
//! This reads; it decides nothing. Whether a `store_safe` into
//! `struct_field_ptr_index_0` means the field is owned is a question for a
//! consumer.

/// One instruction, as the dump wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// The `%N` this instruction binds.
    pub index: u32,
    /// The operation: `arg`, `store_safe`, `struct_field_ptr_index_0`.
    pub op: String,
    /// The result type as the compiler renders it, resolved.
    ///
    /// Conventionally the first argument of the operations that have one. Empty
    /// when the operation's arguments do not begin with a type.
    pub result_type: String,
    /// The argument list verbatim, for anything not decomposed here.
    pub arguments: String,
    /// `%N` values this instruction reads, **in written order**.
    ///
    /// Order is meaning, not presentation: `store_safe(%4!, %0!)` stores `%0`
    /// into the pointer `%4`, and sorting those would silently swap a
    /// destination for a value. Repeats are kept for the same reason.
    pub operands: Vec<u32>,
    /// Nothing reads this result: the dump wrote `%N!=`.
    pub dead: bool,
    /// How deep inside `{ }` blocks the instruction sits.
    pub depth: u8,
    /// The most recent `dbg_stmt` before it, one-based; zero when unknown.
    pub line: u32,
    pub column: u32,
}

/// One function's worth of the dump.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Function {
    /// The compiler's own name: `sub.holder.Holder.init`.
    ///
    /// It is the import path with separators flattened to `.`, then the
    /// container and the function. A consumer wanting a file has to split it
    /// against the paths it already knows, because `a.b.C.f` is ambiguous
    /// between `a/b.zig`'s `C.f` and `a.zig`'s `b.C.f`.
    pub mangled: String,
    /// Instructions as the header counted them, for checking against what was
    /// actually read.
    pub declared_instructions: u32,
    pub air_bytes: u32,
    pub instructions: Vec<Instruction>,
}

/// What the reader did with every line it saw.
///
/// The point is the invariant: a line is a header, an instruction, a block
/// delimiter, or unrecognized, and nothing else. A reader that silently drops
/// syntax it does not know reports a clean run over a corpus it half read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub lines: usize,
    pub headers: usize,
    pub instructions: usize,
    pub delimiters: usize,
    pub blank: usize,
    /// Looked like an instruction and did not parse. Should be zero.
    pub unparsed: usize,
    /// Liveness death annotations: a line of bare `%N!` tokens naming values
    /// that die entering a block. Real information, and not instructions.
    pub deaths: usize,
    /// Not an instruction and not a header: compiler chatter.
    pub other: usize,
    /// Functions whose read instruction count disagreed with their header.
    pub miscounted: usize,
}

impl Tally {
    pub fn accounted(&self) -> usize {
        self.headers
            + self.instructions
            + self.delimiters
            + self.blank
            + self.deaths
            + self.unparsed
            + self.other
    }

    pub fn balances(&self) -> bool {
        self.lines == self.accounted()
    }
}

/// Streams functions out of an AIR dump, one at a time.
///
/// Holds at most one function, because Bun's dump is hundreds of megabytes and
/// a reader that collects it has already lost.
#[derive(Debug, Default)]
pub struct Reader {
    current: Option<Function>,
    line: u32,
    column: u32,
    tally: Tally,
}

impl Reader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tally(&self) -> &Tally {
        &self.tally
    }

    /// Feed one line. Returns a function when that line completed one.
    pub fn push(&mut self, raw: &str) -> Option<Function> {
        self.tally.lines += 1;
        let trimmed = raw.trim_end();
        let body = trimmed.trim_start();

        if body.is_empty() {
            self.tally.blank += 1;
            return None;
        }

        if let Some(rest) = body.strip_prefix("# Begin Function AIR: ") {
            self.tally.headers += 1;
            self.current = Some(Function {
                mangled: rest.trim_end_matches(':').to_owned(),
                ..Function::default()
            });
            self.line = 0;
            self.column = 0;
            return None;
        }

        if body.starts_with("# End Function AIR:") {
            self.tally.headers += 1;
            let finished = self.current.take();
            if let Some(function) = &finished
                && function.declared_instructions as usize != function.instructions.len()
            {
                // Not necessarily wrong — the header counts AIR slots and a
                // block's closing line is not one — but it should be recorded
                // rather than assumed away.
                self.tally.miscounted += 1;
            }
            return finished;
        }

        if body.starts_with('#') {
            self.tally.headers += 1;
            if let Some(function) = self.current.as_mut() {
                if let Some(rest) = body.strip_prefix("# AIR Instructions:") {
                    function.declared_instructions = leading_number(rest);
                } else if let Some(rest) = body.strip_prefix("# Total AIR+Liveness bytes:") {
                    function.air_bytes = leading_number(rest);
                }
            }
            return None;
        }

        if self.current.is_none() {
            self.tally.other += 1;
            return None;
        }

        // A block's closing line: `} %10!)` or `}, unlikely {`.
        if body.starts_with('}') {
            self.tally.delimiters += 1;
            return None;
        }

        // Liveness: `%1! %2! %3!`, the values that die entering this block.
        // These sit where an instruction would and are not one, so they get
        // their own bucket rather than counting as unparsed.
        if is_death_list(body) {
            self.tally.deaths += 1;
            return None;
        }

        match parse_instruction(body, depth_of(trimmed), self.line, self.column) {
            Some(instruction) => {
                // `dbg_stmt` is a position for what follows, not an
                // instruction with a position of its own.
                if instruction.op == "dbg_stmt" {
                    if let Some((line, column)) = parse_position(&instruction.arguments) {
                        self.line = line;
                        self.column = column;
                    }
                }
                self.tally.instructions += 1;
                if let Some(function) = self.current.as_mut() {
                    function.instructions.push(instruction);
                }
            }
            None if body.starts_with('%') => self.tally.unparsed += 1,
            None => self.tally.other += 1,
        }
        None
    }

    /// A dump that ends mid-function still has that function to give.
    pub fn finish(&mut self) -> Option<Function> {
        self.current.take()
    }
}

/// Whether a line is only `%N!` tokens: a liveness death list.
fn is_death_list(body: &str) -> bool {
    let mut any = false;
    for token in body.split_whitespace() {
        let Some(number) = token
            .strip_prefix('%')
            .and_then(|rest| rest.strip_suffix('!'))
        else {
            return false;
        };
        if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
        any = true;
    }
    any
}

/// Indentation is nesting: two spaces per level, one level for the function.
fn depth_of(line: &str) -> u8 {
    let spaces = line.len() - line.trim_start().len();
    u8::try_from(spaces / 2)
        .unwrap_or(u8::MAX)
        .saturating_sub(1)
}

fn leading_number(rest: &str) -> u32 {
    rest.trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

/// `2:35` from a `dbg_stmt`.
fn parse_position(arguments: &str) -> Option<(u32, u32)> {
    let (line, column) = arguments.trim().split_once(':')?;
    Some((line.trim().parse().ok()?, column.trim().parse().ok()?))
}

fn parse_instruction(body: &str, depth: u8, line: u32, column: u32) -> Option<Instruction> {
    let rest = body.strip_prefix('%')?;
    let assign = rest.find('=')?;
    let (target, after) = rest.split_at(assign);
    let dead = target.ends_with('!');
    let index: u32 = target.trim_end_matches('!').trim().parse().ok()?;

    let call = after.strip_prefix('=')?.trim();
    let (op, arguments) = match call.split_once('(') {
        Some((op, arguments)) => (op.trim(), arguments.trim_end_matches(')')),
        // Some operations take no arguments at all.
        None => (call, ""),
    };
    if op.is_empty() {
        return None;
    }

    Some(Instruction {
        index,
        op: op.to_owned(),
        result_type: leading_type(arguments),
        arguments: arguments.to_owned(),
        operands: operands_of(arguments),
        dead,
        depth,
        line,
        column,
    })
}

/// The first argument, when it is a type rather than a value reference.
///
/// Types are how AIR carries the answer we are here for, and they are written
/// first: `struct_field_ptr_index_0(**sub.holder.Inner, %7!)`. A leading `%`
/// means the operation takes a value first and names no type.
fn leading_type(arguments: &str) -> String {
    let first = split_top_level(arguments).into_iter().next().unwrap_or("");
    let first = first.trim();
    if first.is_empty() || first.starts_with('%') || first.starts_with('@') {
        return String::new();
    }
    first.to_owned()
}

/// Split on commas that are not inside brackets of any kind.
fn split_top_level(arguments: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (offset, character) in arguments.char_indices() {
        match character {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&arguments[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    parts.push(&arguments[start..]);
    parts
}

/// Every `%N` the arguments read, in the order written.
///
/// Deliberately neither sorted nor deduplicated. See [`Instruction::operands`].
fn operands_of(arguments: &str) -> Vec<u32> {
    let mut found = Vec::new();
    let bytes = arguments.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let start = at + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start
                && let Ok(index) = arguments[start..end].parse::<u32>()
            {
                found.push(index);
            }
            at = end.max(start);
        } else {
            at += 1;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
