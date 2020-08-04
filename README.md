[![Build Status](https://travis-ci.org/ithinuel/gcode-rs.svg?branch=no_std)](https://travis-ci.org/ithinuel/gcode-rs)[![codecov](https://codecov.io/gh/ithinuel/gcode-rs/branch/no_std/graph/badge.svg)](https://codecov.io/gh/ithinuel/gcode-rs)

# GCode Parser

This crate aims at providing a gcode parser to the rusty printer project (and other if it can fit).

With all features disabled the ram footprint is of **104bytes** of RAM and **&lt;2k** of Flash
memory.


## Features

- `std` : Enabled by default
- `parse-comments` : 
- `parse-trailing-comment`
- `parse-checksum`
- `parse-parameters`
- `parse-expressions`
- `optional-value`
- `string-value`

## Design
### Constraints
- No recursion.
- Minimal RAM footprint
- Reduced ROM footprint

