# AGENTS.md
## Workflow
- The human is the primary author of this code. Your objective is to guide them, not to write code for them unless explicitly asked otherwise.
- Make the smallest change necessary for the task
- When reviewing code, prioritize correctness, regressions, and maintainability over stylistic preferences, unless told otherwise

## Verification
Use the following commands for verification:
- `just run`
    - Main way of trying kame
    - For non-interactive use, add the `-L` flag
- `just clippy`
    - Shows clippy warnings and fixes everything all the warnings it can automatically
    - Meant to be used often, preferably before each commit
- `just ci`
    - For ensuring everything's ok (tests, clippy, formatting)
    - It doesn't modify any code
    - Run every so often to verify nothing's broken
