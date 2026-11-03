# IDE schemas

`schemas/` holds base JSON Schema (Draft 2020-12) templates for the JSON
documents this crate consumes. Point your editor at them and authoring a form, UI
or rules document gives you autocomplete, inline documentation and validation
while you type.

| File                                                                | Describes                                                    |
| ------------------------------------------------------------------- | ------------------------------------------------------------ |
| [form.schema.json](../schemas/form.schema.json)                     | The form document: `schemaVersion`, `$schema`, `fields`      |
| [ui.schema.json](../schemas/ui.schema.json)                         | The UI document: `fields`, `layout`, versions                |
| [rules.schema.json](../schemas/rules.schema.json)                   | The rules document: `fields`, `validations`, expressions     |
| [golden-vectors.schema.json](../schemas/golden-vectors.schema.json) | Seven of the eight `tests/golden/vectors/*.json` group files |

## Guarantee

The three document schemas use only keywords from the subset colander implements
([json-schema.md](json-schema.md), SPEC S-1…S-13), with local `#/$defs/…`
references only. They therefore also work when passed to
`colander_validate_schema`: every keyword is known to the core's classifier, and
the recursive references (`items`, `args`) terminate because instances are
finite.

Strictness follows the contract: a key is `required`, enumerated or closed
(`additionalProperties: false`) only where the core rejects or drops it — field
`id`/`code`/`type`, layout `type`, validation `code`/`assert`, the twelve field
types (SPEC D-2), top-level keys (SPEC P-7). Where the core tolerates or preserves
— unknown keys inside form fields, opaque UI entries, null-or-expression rules —
the schema stays permissive and says so in its `description`.

## Wiring

This repository ships no editor configuration, so mapping is one step in your
IDE. Name your documents `*.form.json`, `*.ui.json` and `*.rules.json` to match
the patterns below.

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
checking. The schema root accepts the seven group files it defines.

IntelliJ: Settings → Languages & Frameworks → Schemas and DTDs → JSON Schema
Mappings, add the three document schemas with the same file patterns, and
`schemas/golden-vectors.schema.json` for `tests/golden/vectors/*.json`.

## Vectors

The golden vectors are frozen against accidental drift (SPEC F-1): the vectors
schema is reference-only for editing and validation, and never loosens a fixture.
The `describe` group was added after the template and has no fragment yet, so it
is not covered by the schema.

## See also

- The documents these schemas describe: [documents.md](documents.md)
- The rule operators: [rules.md](rules.md)
- The keyword subset the schemas stay inside: [json-schema.md](json-schema.md)
