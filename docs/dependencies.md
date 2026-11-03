# Dependencies and hand-written code

What colander depends on, and what it keeps hand-written. Read it when you wonder
why the crate has so few dependencies, or whether `cargo build` needs anything
beyond crates.io.

## Runtime dependencies

`Cargo.toml` declares exactly three direct runtime dependencies, which resolve to
sixteen transitive crates — nineteen in the normal graph. There is no path or
development dependency.

| Crate         | Version  | What it does                                                                                                                                                                                                                                                |
| ------------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fancy-regex` | `0.19.2` | Backs the `pattern` constraint in response validation and in the JSON Schema subset. It follows ECMA-262 as far as the engine supports, including look-around and backreferences; an uncompilable pattern is an error, never a silent non-match (SPEC S-6). |
| `indexmap`    | `2`      | Keeps JSON object members and the maps derived from them in insertion order. A document round-trips in its original key order, and duplicate keys keep their first position while the last value wins.                                                      |
| `sha2`        | `0.11.0` | Produces the lowercase-hex SHA-256 returned as `contentHash` by `colander_content_hash` and `colander_compile`.                                                                                                                                             |

`regex` is no longer a dependency, direct or transitive: `fancy-regex` builds on
`regex-automata`, `regex-syntax`, `bit-set` and `bit-vec` directly, never on the
`regex` facade.

Canonical serialization is a separate, hand-written writer. It sorts keys by
UTF-8 byte order, so its output does not depend on the insertion order
`indexmap` preserves.

## Hand-written modules

| Module                                         | Role                                                                          | Why it is not a dependency                                                                                                                                                                |
| ---------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/json.rs` with `src/json/{parse,write}.rs` | The JSON parser and writer at every boundary.                                 | The core parses JSON itself "to keep number spelling and key order intact". Numbers from your input are written back with their original spelling. See [json.md](json.md).                |
| `src/semver.rs`                                | Parses the version strings `colander_next_version` accepts.                   | The syntax is colander's own contract: a strict three-integer triplet. The accepted and rejected forms are in [entry-points-payload.md](entry-points-payload.md#accepted-version-syntax). |
| `src/{compile,rules,validate,schema}.rs`       | Compilation, the rule engine, response validation and the JSON Schema subset. | This is the crate's reason to exist. It holds no schema and no domain vocabulary; documents and the JSON Schemas describing them travel with each request.                                |

## Flat modules, one deliberate trait

The crate is deliberately flat: one file per domain, and inside each domain one
file per concern, so no file has to be read whole to be understood. The single
deliberate trait is the `Codec` seam in `src/codec.rs`, which the envelope
boundary is generic over so the wire format is injectable. There is no plugin
layer beyond that. As the repository says, "a second use is what would justify
extracting anything."

## Self-contained build

`Cargo.toml` declares three direct runtime dependencies and no dev-dependency. A
clean clone builds and tests with nothing but crates.io.

## See also

- Back to the repository landing page: [../README.md](../README.md)
- Full entry-point reference: [entry-points.md](entry-points.md)
- What the golden vectors assert: [vectors.md](vectors.md)
