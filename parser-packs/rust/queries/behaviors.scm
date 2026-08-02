; Enum/macro behavior recognition, moved out of
; infact-rust-behaviors/src/lib.rs:426-793.
;
; Tree-sitter queries are existential and cannot count. Three constraints the
; Rust recognizers enforce are therefore NOT expressible here and stay in the
; caller, which is the honest cost of this move:
;
;   * arity      -- `only_expression` (a body holds exactly one named child)
;   * coverage   -- "every arm/variant matched", not "some arm/variant matched"
;   * ordering   -- `manual_variant_array` compares an element SEQUENCE
;
; Each pattern emits one match per repeated part (arm, variant, element), so
; the caller groups by the enclosing item before deciding anything.
;
; Every `#eq?` sits INSIDE its pattern's parentheses. A predicate placed after
; the closing paren compiles cleanly and silently becomes its own pattern that
; matches every node in the file.

; --- manual `as_str`: `match self { Self::V => "v", .. }` --------------------
; Equivalent of `manual_as_str` (lib.rs:632). One match per arm.
(impl_item
  type: (type_identifier) @as-str.type
  body: (declaration_list
    (function_item
      name: (identifier) @as-str.name
      body: (block
        (expression_statement
          (match_expression
            value: (self)
            body: (match_block
              (match_arm
                pattern: (match_pattern
                  [
                    (scoped_identifier name: (identifier) @as-str.variant)
                    (identifier) @as-str.variant
                  ])
                value: (string_literal
                  (string_content) @as-str.value)) @as-str.arm)) @as-str.match)) @as-str.body) @as-str.fn)
  (#eq? @as-str.name "as_str"))

; --- `Display` delegating to `as_str` ---------------------------------------
; Equivalent of `display_delegates_to_as_str` (lib.rs:683). One match per impl.
; The trait may be written `Display` or `std::fmt::Display`, so both spellings
; are admitted. `fmt`'s body holds the call DIRECTLY, unlike `as_str`, whose
; `match` is wrapped in an `expression_statement` -- a grammar detail verified
; by dumping the tree, not assumed.
(impl_item
  trait: [
    (type_identifier) @display.trait
    (scoped_type_identifier name: (type_identifier) @display.trait)
  ]
  type: (type_identifier) @display.type
  body: (declaration_list
    (function_item
      name: (identifier) @display.name
      body: (block
        (call_expression
          function: (field_expression
            value: (call_expression
              function: (field_expression
                value: (self)
                field: (field_identifier) @display.inner)
              arguments: (arguments) @display.inner-args)
            field: (field_identifier) @display.outer))) @display.body))
  (#eq? @display.name "fmt")
  (#eq? @display.inner "as_str")
  (#eq? @display.outer "fmt")
  (#eq? @display.trait "Display"))

; --- unit enum --------------------------------------------------------------
; Equivalent of `unit_enum` (lib.rs:548). One match per variant. A variant
; carrying a payload has extra named children, so the caller checks that every
; variant of @unit-enum.body appeared -- "all variants are unit" is a counting
; claim and cannot be stated here.
(enum_item
  name: (type_identifier) @unit-enum.name
  body: (enum_variant_list
    (enum_variant
      name: (identifier) @unit-enum.variant) @unit-enum.variant-node) @unit-enum.body)

; --- `serde(rename_all = "..")` on the enum ---------------------------------
; Equivalent of `enum_serde_case` (lib.rs:528). Attributes are PRECEDING
; SIBLINGS of the enum, not children of it, so this matches the attribute alone
; and the caller associates it by position, as `prev_named_sibling` does today.
(attribute_item
  (attribute
    (identifier) @serde-case.macro
    arguments: (token_tree
      (identifier) @serde-case.key
      (string_literal (string_content) @serde-case.value)))
  (#eq? @serde-case.macro "serde")
  (#eq? @serde-case.key "rename_all"))

; --- manual variant array ---------------------------------------------------
; Equivalent of `manual_variant_array` (lib.rs:568). One match per element.
; The Rust version requires the element sequence to EQUAL the variant list, in
; order; a query can neither order nor compare sequences, so the caller does it.
(impl_item
  type: (type_identifier) @variant-array.type
  body: (declaration_list
    (const_item
      name: (identifier) @variant-array.const
      value: (array_expression
        [
          (scoped_identifier name: (identifier) @variant-array.element)
          (identifier) @variant-array.element
        ]) @variant-array.array) @variant-array.item))
