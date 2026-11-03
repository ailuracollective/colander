# JSON

The parser and writer at the edge of every call: the grammar and limits the
parser accepts, how numbers are written back, and how the key-sorted form orders
keys. Read it when key order, number spelling or hashing matters.

The parser and writer are hand-written on purpose. The core parses JSON itself to
keep number spelling and key order intact, so a number you supplied can come back
spelled exactly as you wrote it, and the hashing payload can depend on document
key order.

## Parsing and limits

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

## Key-sorted form

The key-sorted form sorts object keys by **UTF-8 byte order**. The contract does
not call it "canonical": it is not RFC 8785, which orders by UTF-16 code units,
and interoperation with that standard is not offered (SPEC H-4). The two orders
differ for some non-BMP and U+E000–U+FFFF mixes.

The exact payload `colander_content_hash` hashes — and the fact that it is _not_
the key-sorted form — is documented with the entry point in
[entry-points-payload.md](entry-points-payload.md#colander_content_hash).

## See also

- Behaviour worth knowing: [gotchas.md](gotchas.md)
- The full hashing entry point: [entry-points-payload.md](entry-points-payload.md#colander_content_hash)
- The JSON Schema subset built on this parser: [json-schema.md](json-schema.md)
