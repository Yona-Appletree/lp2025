# Phase 5: Fix Remaining Tests

## Description

Fix the remaining 4 tests that don't fit into the previous categories.

## Tests to Fix

- `backend::transform::fixed32::transform::tests::test_do_while`
- `exec::emu::tests::test_emu_builtin_sqrt_linked`
- `exec::emu::tests::test_emu_float_addition_fixed32`
- `exec::emu::tests::test_emu_float_constant_fixed32`
- `exec::emu::tests::test_emu_float_multiplication_fixed32`
- `exec::emu::tests::test_emu_user_fn_fixed32`

## Implementation

1. Run each test individually to see specific failure
2. Investigate root cause for each
3. Fix issues one by one

## Success Criteria

- All remaining tests pass
- No regressions
