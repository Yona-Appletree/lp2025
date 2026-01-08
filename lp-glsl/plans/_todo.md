# Needed for demo

- globals
- *out* support for arguments as pointers for ABI/JIT
- trap recovery / stack recovery
- stack overflow trap
- cycle counting of some sort to prevent infinite loops


# Safety

- instruction counter limit trap
- output parameter support (for frexp/modf)

# Performance

- filetests should compile whole file, and only compile a single function when in detail mode

# Builtins

- pack/unpack functions frontend codegen
- integer bit functions frontend codegen
- floatBitsToInt/intBitsToFloat frontend codegen
