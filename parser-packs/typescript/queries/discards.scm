; Discarded-error recognition for TypeScript.
;
; The forms are language-neutral -- `DiscardForm` names no Rust concept -- but
; the spellings are not. Where Rust writes `Err(_)`, TypeScript writes a `catch`
; with nothing bound; where Rust writes `.ok()`, TypeScript writes
; `.catch(() => null)`.
;
; The same trick carries both: a QUANTIFIED capture that is simply absent when
; nothing is bound. Queries have no negation, and absence is the claim.

; --- `catch { }` -- a handler that binds nothing ----------------------------
; `catch (error) { .. }` produces @bind and is not a discard, exactly as
; `Err(error)` is not. A bound-but-unused parameter needs an emptiness test the
; Rust analyzer does not make either, so it is not reported here.
(catch_clause
  parameter: (_)? @discard.err-arm.bind
  body: (statement_block)) @discard.err-arm.expression @discard.err-arm

; --- `.catch(() => null)` -- a cause turned into an absence -----------------
; `.catch((error) => ..)` still sees the cause, so its parameter produces @bind.
(call_expression
  function: (member_expression
    object: (_) @discard.ok-discard.expression
    property: (property_identifier) @discard.ok-discard.method)
  arguments: (arguments
    [
      (arrow_function parameters: (formal_parameters (_)? @discard.ok-discard.bind))
      (function_expression parameters: (formal_parameters (_)? @discard.ok-discard.bind))
    ])
  (#eq? @discard.ok-discard.method "catch")) @discard.ok-discard

; --- `void risky()` -- a value discarded on purpose -------------------------
; The nearest thing TypeScript has to `let _ =`: an explicit statement that the
; result, and any rejection it carries, is being thrown away.
(unary_expression
  operator: "void"
  argument: (call_expression) @discard.let-underscore.expression) @discard.let-underscore
