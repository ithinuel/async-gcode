# GCode Parser

This crate aims at providing a gcode parser to the rusty printer project (and other if it can fit).

## Features

- `parse-expressions` : enables parsing of expressions
- `parse-comments` : generates an event when a comment (or a message) is parsed.
- `parse-parameters` : enables parsing of parameters
- `no_std` : Adds appropriate reference to `core` & `alloc`
- `extended` : enables semi-colon comments and removes word letter restriction.

By default all three `parse-*` features are enabled.
