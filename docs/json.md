# JSON

This page covers the parser and writer at the edge of every call: the grammar
and limits the parser accepts, how numbers are written back, and how canonical
serialization orders keys. Read it when key order, number spelling or hashing
matters to you.

The parser and writer are hand-written, and that matters: the core parses JSON
itself, to keep number spelling and key order intact. That is why a number you
supplied can come back spelled exactly as you wrote it, and why the hashing
payload can depend on document key order.

## JSON parsing and limits

The core parses JSON itself, to keep number spelling and key order intact.

| Behaviour               | Detail                                                                                                                        |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Duplicate object keys   | Last value wins, first position is kept; no error                                                                             |
| Trailing content        | Rejected: `unexpected trailing content at byte offset N.`                                                                     |
| Nesting                 | `MAX_DEPTH` 64 — at most 65 nested containers                                                                                 |
| Numbers                 | Strict grammar; the literal source text is preserved. No leading `+`, no leading zeros, no `.5`, no `1.`, no `NaN`/`Infinity` |
| Integers                | A literal counts as an integer only without `.`, `e` or `E`, and only if it fits `i64`                                        |
| Strings                 | Escapes `\" \\ \/ \b \f \n \r \t \uXXXX`; raw control characters below U+0020 are rejected; lone surrogates are rejected      |
| Comments, single quotes | Not supported                                                                                                                 |
| Errors                  | Always carry a byte offset                                                                                                    |

### Number output

Numbers that came from your input are written back with their original spelling.
Numbers the core **computes** use shortest round-trip digits, switching to
exponent form outside `[-4, 16]`:

| Value      | Written as          |
| ---------- | ------------------- |
| `3.0`      | `3`                 |
| `-0.0`     | `-0`                |
| `22.86`    | `22.86`             |
| `1e16`     | `10000000000000000` |
| `1e17`     | `1E+17`             |
| `1e-4`     | `0.0001`            |
| `1e-5`     | `1E-05`             |
| non-finite | `null`              |

Canonical serialization sorts keys by **UTF-8 byte order**, which differs from
UTF-16 code-unit order for some non-BMP and U+E000–U+FFFF mixes.

The exact payload that `colander_content_hash` hashes — and the fact that it is
_not_ the key-sorted canonical form — is documented with the entry point in
[entry-points.md](entry-points.md#colander_content_hash).

## Next

- Behaviour worth knowing: [gotchas.md](gotchas.md)
- The full hashing entry point: [entry-points.md](entry-points.md#colander_content_hash)
- The JSON Schema subset built on this parser: [json-schema.md](json-schema.md)
