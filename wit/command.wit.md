
# WASI Commands

WASI Commands are a placeholder, designed for legacy compatibility with
snapshot 1 commands.

## Imports
```wit
use { descriptor } from wasi-filesystem
```

## `command`
```wit
/// The entrypoint of a WASI command.
command: func(stdin: descriptor, stdout: descriptor)
```
