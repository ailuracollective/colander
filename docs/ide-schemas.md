# IDE schemas

`schemas/` holds base JSON Schema (Draft 2020-12) templates for every JSON
document this crate consumes. Point your editor at them and authoring a form,
UI or rules document gives you autocomplete, inline documentation and
validation while you type.

| File                                                                | Describes                                                |
| ------------------------------------------------------------------- | -------------------------------------------------------- |
| [form.schema.json](../schemas/form.schema.json)                     | The form document: `schemaVersion`, `$schema`, `fields`  |
| [ui.schema.json](../schemas/ui.schema.json)                         | The UI document: `fields`, `layout`, versions            |
| [rules.schema.json](../schemas/rules.schema.json)                   | The rules document: `fields`, `validations`, expressions |
| [golden-vectors.schema.json](../schemas/golden-vectors.schema.json) | The seven `tests/golden/vectors/*.json` files            |

## Guarantee

The three document schemas use only keywords from the subset colander itself
implements ([json-schema.md](json-schema.md), SPEC S-1…S-7), with local
`#/$defs/…` references only. That means they also work when passed to
`colander_validate_schema`: every keyword in them is known to the core's own
classifier, and the recursive references (`items`, `args`) terminate because
instances are finite.

Strictness follows the contract, not the other way round: a key is
`required`, enumerated or closed (`additionalProperties: false`) only where
the core rejects or drops it — field `id`/`code`/`type`, layout `type`,
validation `code`/`assert`, the twelve field types (SPEC D-2), top-level keys
(SPEC P-7). Everywhere the core tolerates or preserves (unknown keys inside
form fields, opaque UI entries, null-or-expression rules), the schema stays
permissive and says so in its `description`.

## Wiring

This repository ships no editor configuration, so mapping is one step in
your IDE. Name your documents `*.form.json`, `*.ui.json` and `*.rules.json`
to match the patterns below.

VS Code (`settings.json`):

```json
"json.schemas": [
  { "fileMatch": ["*.form.json"], "url": "./schemas/form.schema.json" },
  { "fileMatch": ["*.ui.json"], "url": "./schemas/ui.schema.json" },
  { "fileMatch": ["*.rules.json"], "url": "./schemas/rules.schema.json" },
  {
    "fileMatch": ["tests/golden/vectors/compile.json"],
    "url": "./schemas/golden-vectors.schema.json#/$defs/compileFile"
  }
]
```

Map each vector file to its own `#/$defs/<group>File` fragment for precise
checking; the schema root accepts any of the seven files.

IntelliJ: Settings → Languages & Frameworks → Schemas and DTDs → JSON
Schema Mappings, add the three document schemas with the same file patterns,
and `schemas/golden-vectors.schema.json` for
`tests/golden/vectors/*.json`.

## Vectors

The golden vectors are frozen against accidental drift (SPEC F-1): the
vectors schema is reference-only for editing and validation. It never
loosens a fixture — all 249 recorded entries validate against it.

## Next

- The documents these schemas describe: [documents.md](documents.md)
- The rule operators: [rules.md](rules.md)
- The keyword subset the schemas stay inside: [json-schema.md](json-schema.md)
