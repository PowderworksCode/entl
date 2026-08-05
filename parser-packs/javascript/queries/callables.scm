; The scaffolding the discard analyzer joins against: callables, the class they
; sit in, and call edges. Nothing here is shared with the Rust pack -- the node
; kinds differ entirely -- which is the point: the consumer reads captures, not
; grammar.
;
; The typescript pack's file with every `@callable.with-return` pattern removed,
; because `tree-sitter-javascript` has no `type_annotation` and a query naming a
; kind its grammar lacks fails the pack load outright. That is the difference
; between the two languages and it is the whole difference: JavaScript declares
; nothing about failure in a signature, which is exactly what
; `propagation = "unchecked"` already says.
;
; `class_declaration` names an `identifier` here where TypeScript names a
; `type_identifier`, so this pattern is edited rather than copied too.

(function_declaration
  name: (identifier) @callable.name
  body: (statement_block) @callable.body) @callable.item

(method_definition
  name: (property_identifier) @callable.name
  body: (statement_block) @callable.body) @callable.item

; `const handler = async () => { .. }` -- the name is on the declarator, so the
; arrow itself carries the body and the declarator carries the name.
(variable_declarator
  name: (identifier) @callable.name
  value: [
    (arrow_function body: (statement_block) @callable.body)
    (function_expression body: (statement_block) @callable.body)
  ]) @callable.item

; A class is what an impl block is: the container a callable's path names.
(class_declaration
  name: (identifier) @impl.type
  body: (class_body)) @impl.item

(call_expression
  function: [
    (identifier) @call.callee
    (member_expression property: (property_identifier) @call.callee)
  ]) @call.site
