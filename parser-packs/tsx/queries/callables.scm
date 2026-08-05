; The typescript pack's callables.scm, copied. `.ts` and `.tsx` are one
; language read by two grammars, so this pack must describe the same forms or
; the same code gets a different answer depending on the extension it was
; written under. Verified: these patterns compile against the tsx grammar
; unchanged -- JSX adds node kinds, it removes none.
;
; The two files are copies rather than shared source because a pack manifest
; has no way to name another pack's queries, and inventing one for a single
; consumer is not worth it. `packs_for_one_language_describe_it_the_same_way`
; is what makes a divergence loud; drift in the PATTERNS is the residual risk,
; and it is the ordinary risk every query file already carries.

; The scaffolding the discard analyzer joins against: callables, the class they
; sit in, and call edges. Nothing here is shared with the Rust pack -- the node
; kinds differ entirely -- which is the point: the consumer reads captures, not
; grammar.

(function_declaration
  name: (identifier) @callable.name
  body: (statement_block) @callable.body) @callable.item

(function_declaration
  return_type: (type_annotation) @callable.return
  body: (statement_block)) @callable.with-return

(method_definition
  name: (property_identifier) @callable.name
  body: (statement_block) @callable.body) @callable.item

(method_definition
  return_type: (type_annotation) @callable.return
  body: (statement_block)) @callable.with-return

; `const handler = async () => { .. }` -- the name is on the declarator, so the
; arrow itself carries the body and the declarator carries the name.
(variable_declarator
  name: (identifier) @callable.name
  value: [
    (arrow_function body: (statement_block) @callable.body)
    (function_expression body: (statement_block) @callable.body)
  ]) @callable.item

(variable_declarator
  value: [
    (arrow_function return_type: (type_annotation) @callable.return)
    (function_expression return_type: (type_annotation) @callable.return)
  ]) @callable.with-return

; A class is what an impl block is: the container a callable's path names.
(class_declaration
  name: (type_identifier) @impl.type
  body: (class_body)) @impl.item

(call_expression
  function: [
    (identifier) @call.callee
    (member_expression property: (property_identifier) @call.callee)
  ]) @call.site
