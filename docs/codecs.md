# Codecs: the seam between the document model and its bytes

The core stores every document as `Json` and never hardcodes how those bytes
travel. A `Codec` maps bytes to `Json` and back, and the envelope boundary is
generic over it. JSON is the default and stays byte-identical; MessagePack is a
second, handwritten implementation that proves the seam is real.

## Quick path

1. Read the trait in `src/codec.rs`: `decode`, `encode_canonical`,
   `encode_ordered`, and an associated `Encoded` type.
2. Use `JsonCodec` for the published behavior; it is the only codec the C ABI
   ever injects.
3. Use `ffi::envelope::run` to drive any codec end to end from Rust.

## Why injection is static

`ffi::envelope::dispatch` is generic over `C: Codec`, and each export passes
`&JsonCodec`. The compiler monomorphizes the call site, so there is no vtable, no
dynamic dispatch, and no per-node indirection. The codec choice is a compile-time
constant in the generated code.

This also keeps the JSON path allocation-free. The associated `Encoded` type is
`String` for JSON, so `CString::new` takes ownership of the writer's buffer with
zero copies. A `Vec<u8>`-only trait surface would add one copy and one `String`
round trip per response.

## What each implementation does

| Codec              | `Encoded` | Backed by                                         |
| ------------------ | --------- | ------------------------------------------------- |
| `JsonCodec`        | `String`  | `json::parse`, `json::canonical`, `json::ordered` |
| `MessagePackCodec` | `Vec<u8>` | Handwritten encoder and decoder                   |

The two codecs share the request boundary: `read_request` returns borrowed bytes
with no UTF-8 validation, and the codec performs the one validation it needs.
On the JSON path that removes one `String` allocation and one copy per request.

## Tradeoffs this proof of concept surfaces

1. **Number spelling is lossy.** `Json::Number` stores the literal source text,
   while MessagePack has integers and floats, not literals. `1.50` returns as
   `1.5`, and `1e3` returns as `1000`. Nothing is corrupted; the spelling is
   simply gone. A real wire swap would change `contentHash`.
2. **Canonicality becomes codec-imposed.** MessagePack has no canonical form, so
   sorting keys before encoding is the codec's job, not the format's.
   `encode_canonical` sorts by UTF-8 byte order; the format itself will not.
3. **Key order survives.** `JsonMap` is an `IndexMap`; encoding in iteration
   order and decoding in stream order preserves document order. Only canonical
   encoding sorts.
4. **Binary cannot cross the `char *` ABI.** The request boundary is a
   NUL-terminated UTF-8 C string, so a MessagePack payload containing `0x00`
   cannot be delivered through an export. That is why the seam is exercised from
   Rust, through `ffi::envelope::run`.

## When this would cross the ABI

It does not today, and this page does not propose that it should. A binary wire
format would need a length-prefixed request boundary instead of `char *`, plus a
new set of exports or a negotiated format selector. Both are contract changes:
they touch `include/colander.h`, the export list, and `ABI_VERSION`. The seam
exists so that, when such a decision is made, the plumbing does not have to be
rewritten in place.

## Next

- The parser and writers the JSON codec delegates to: [json.md](json.md)
- The boundary rules: [abi.md](abi.md)
- What stays hand-written, and why: [dependencies.md](dependencies.md)
