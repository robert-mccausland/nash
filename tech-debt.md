1. Lexer matching logic still quite confusing to reason about
2. Better error messages / error handling logic in general
3. Backtrackable has to do a lot of peeking to make sure that errors are reported at the correct position
4. More unit tests?
5. Only implement Serialize in test builds
6. Test the commands, as they are used in the integration tests so bugs in them are a bit invisible
7. Cross platform compatibility with newlines and such needing to be consistent across environments but also would be good for them to match the environment they are in.
8. Command class just has a bunch of funky logic in it, probably needs to be split into more files & sorted out a bit.
9. In the command class always use Stdio for redirecting to other commands or files, only create pipe if we need to capture output in the program.
10. Refactor integration tests to read code from files instead of inline code.
11. What to do about errors that should never happen anymore due to the post-processor catching them?
12. Unit tests (lol)
13. Builtin functions are pretty messy at the moment
14. Function calls & Variables are considered the same expression and its quite confusing
