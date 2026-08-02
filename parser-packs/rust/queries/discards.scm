; Discarded-error recognition. One capture per DiscardForm, named after the
; variant, so the caller maps a capture name to a form and names no node kind.
;
; What CANNOT live here, and stays as pack data in parser.toml:
;   * which forms are ambiguous (`ambiguous-forms`) -- `.unwrap_or_default()`
;     and `.unwrap()` read identically on a type that never failed, and no
;     query can see a type
;   * which callables return a non-failure `Err` (`non-failure-results`) --
;     `binary_search` answers "not present"; that is a fact about the stdlib
;
; A `#eq?` or `#match?` must sit INSIDE its pattern's parentheses. A predicate
; after the closing paren compiles cleanly and silently becomes its own pattern
; matching every node in the file.

; --- `let _ = fallible();` --------------------------------------------------
; `_` is an ANONYMOUS token, matched as a quoted node rather than a named one.
(let_declaration
  pattern: "_"
  value: [
    (call_expression)
    (try_expression)
  ] @discard.let-underscore.expression) @discard.let-underscore

; --- `let Ok(..) = fallible() else { .. }` ----------------------------------
; The `alternative` is what separates this from an ordinary destructuring let:
; the else arm runs on failure and never sees the cause.
(let_declaration
  pattern: (tuple_struct_pattern
    type: [
      (identifier) @discard.ok-binding.type
      (scoped_identifier name: (identifier) @discard.ok-binding.type)
    ])
  value: (_) @discard.ok-binding.expression
  alternative: (block)
  (#eq? @discard.ok-binding.type "Ok")) @discard.ok-binding

; --- `if let Ok(..) = fallible()` -------------------------------------------
(let_condition
  pattern: (tuple_struct_pattern
    type: [
      (identifier) @discard.ok-binding.type
      (scoped_identifier name: (identifier) @discard.ok-binding.type)
    ])
  value: (_) @discard.ok-binding.expression
  (#eq? @discard.ok-binding.type "Ok")) @discard.ok-binding

; --- `Err(_) => ..` ---------------------------------------------------------
; @bind is QUANTIFIED over ANY node kind: `Err(e)` and `Err((a, b))` both
; produce it, `Err(_)` does not, and its ABSENCE is how a query says "nothing
; is bound here". Restricting it to (identifier) misses the tuple case and
; reports a bound-and-used error as discarded.
(match_arm
  pattern: (match_pattern
    (tuple_struct_pattern
      type: [
        (identifier) @discard.err-arm.type
        (scoped_identifier name: (identifier) @discard.err-arm.type)
      ]
      (_)? @discard.err-arm.bind) @discard.err-arm.expression)
  (#eq? @discard.err-arm.type "Err")) @discard.err-arm

; --- `.ok()` ----------------------------------------------------------------
; `Result::ok` has no `Option` counterpart, so the receiver is a Result. The
; receiver's own method is captured so the caller can decline the pack's
; `non-failure-results`.
(call_expression
  function: (field_expression
    value: (_) @discard.ok-discard.expression
    field: (field_identifier) @discard.ok-discard.method)
  (#eq? @discard.ok-discard.method "ok")) @discard.ok-discard

; --- `.unwrap_or(..)`, `.unwrap_or_default()` -------------------------------
(call_expression
  function: (field_expression
    value: (_) @discard.unwrap-or.expression
    field: (field_identifier) @discard.unwrap-or.method)
  (#match? @discard.unwrap-or.method "^(unwrap_or|unwrap_or_default)$")) @discard.unwrap-or

; --- `.unwrap_or_else(|_| ..)` ----------------------------------------------
; Declined when the closure binds, because `|error| ..` still sees the cause.
; A call with no closure at all is still a discard.
(call_expression
  function: (field_expression
    value: (_) @discard.unwrap-or.expression
    field: (field_identifier) @discard.unwrap-or.method)
  arguments: (arguments
    (closure_expression
      parameters: (closure_parameters (_)? @discard.unwrap-or.bind))?)
  (#eq? @discard.unwrap-or.method "unwrap_or_else")) @discard.unwrap-or

; --- `.map_err(|_| ..)` -----------------------------------------------------
; Requires an EXPLICIT unbound closure. `map_err(SomeError::from)` forwards a
; function that may well read the cause, so it is not a discard.
(call_expression
  function: (field_expression
    value: (_) @discard.cause-erased.expression
    field: (field_identifier) @discard.cause-erased.method)
  arguments: (arguments
    (closure_expression
      parameters: (closure_parameters (_)? @discard.cause-erased.bind)) @discard.cause-erased.closure)
  (#eq? @discard.cause-erased.method "map_err")) @discard.cause-erased

; --- `.unwrap()`, `.expect(..)` ---------------------------------------------
(call_expression
  function: (field_expression
    value: (_) @discard.panic.expression
    field: (field_identifier) @discard.panic.method)
  (#match? @discard.panic.method "^(unwrap|expect)$")) @discard.panic

; --- `.filter_map(Result::ok)` ----------------------------------------------
; Drops failed items mid-iteration. The argument is matched by text because
; what makes it a drop is the `ok` it names, in either spelling.
(call_expression
  function: (field_expression
    value: (_) @discard.iterator-drop.expression
    field: (field_identifier) @discard.iterator-drop.method)
  arguments: (arguments) @discard.iterator-drop.arguments
  (#eq? @discard.iterator-drop.method "filter_map")
  (#match? @discard.iterator-drop.arguments "Result::ok|\\.ok\\(\\)\\s*\\)\\s*$")) @discard.iterator-drop
