# Code review process

Format the code with `cd lightplayer/ && cargo +nightly fmt` (only format lightplayer code)
Build the code and the tests.
Plan any needed changes to the code. Be specific about the changes.

## Code style

- Fix most warnings.
- Unused code warnings for new code that will be used in the future can be ignored.
- Long files (more than about 200 lines) should be split into modules and separate files.
- Most general function / type should be at the top. Utilities at the bottom.
- Functions have short, desrcpitive doc comments.
- There should be no unnecessary train-of-thought comments.
- Comments should be used to explain the code, not to think out loud.

## Plan contents

The generated plan should follow this pattern:

- Commit current work
- Perform needed changes
- Run 'cd lightplayer/ && cargo +nightly fmt'
- Run tests for current module (implies a build)
- Fix any problems

## Testing

Unit tests should be fairly few and cover utilites and smoke testing.
Full correctness testing is provided by filetests.
