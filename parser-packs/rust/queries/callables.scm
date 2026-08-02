; The structural scaffolding `walk` builds today, as data: callables, the impl
; they sit in, call edges, test markers, and modules.
;
; This is the load-bearing question for multi-language. If this is expressible,
; the ~385 per-language lines of infact-rust-errors collapse to a pack; if it is
; not, every new language needs its own walk in Rust regardless of where the
; discard recognizers live.
;
; Everything the caller does with these is POSITIONAL -- byte-range containment
; decides which callable holds a discard, which impl holds a callable. Nothing
; downstream names a Rust node kind.

(function_item
  name: (identifier) @callable.name
  body: (block) @callable.body) @callable.item

(function_item
  return_type: (_) @callable.return
  body: (block)) @callable.with-return

(impl_item
  type: (_) @impl.type
  body: (declaration_list)) @impl.item

(mod_item
  name: (identifier) @mod.name) @mod.item

(call_expression
  function: [
    (identifier) @call.callee
    (scoped_identifier name: (identifier) @call.callee)
    (field_expression field: (field_identifier) @call.callee)
  ]) @call.site

(attribute_item (attribute) @attribute.text) @attribute.item
