; Rust tags.scm - Symbol extraction for ctags-style navigation

; Function definitions (all types)
(function_item
    name: (identifier) @name.definition.function) @definition.function

; Methods in impl blocks
(impl_item
    body: (declaration_list
        (function_item
            name: (identifier) @name.definition.method))) @definition.method_container

; Struct definitions (all types - standard, tuple, unit)
(struct_item
    name: (type_identifier) @name.definition.struct) @definition.struct

; Enum definitions with variants
(enum_item
    name: (type_identifier) @name.definition.enum) @definition.enum

; Enum variants
(enum_variant
    name: (identifier) @name) @definition.enumerator

; Trait definitions
(trait_item
    name: (type_identifier) @name.definition.trait) @definition.trait

; Impl blocks (inherent implementation)
(impl_item
    type: (type_identifier) @name.definition.impl) @definition.impl

; Trait implementations
(impl_item
    trait: (type_identifier) @name.definition.impl_trait
    type: (type_identifier) @name.definition.impl_for) @definition.impl_trait

; Type aliases
(type_item
    name: (type_identifier) @name.definition.type_alias) @definition.type_alias

; Constants
(const_item
    name: (identifier) @name.definition.constant) @definition.constant

; Static items
(static_item
    name: (identifier) @name.definition.static) @definition.static

; Modules
(mod_item
    name: (identifier) @name.definition.module) @definition.module

; Macros
(macro_definition
  name: (identifier) @name) @definition.macro
