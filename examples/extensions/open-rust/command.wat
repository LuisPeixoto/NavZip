(module
  (import "navzip" "emit" (func $emit (param i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "{\22action\22:\22create_tab\22,\22url\22:\22https://www.rust-lang.org/\22}")
  (func (export "run") (result i32)
    i32.const 0
    i32.const 58
    call $emit))
