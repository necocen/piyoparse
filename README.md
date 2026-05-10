# piyoparse

Rust parser for PiyoLog export files. The crate can parse both iOS and Android export layouts, and can also be built for WebAssembly so TypeScript code can call it.

This is an unofficial tool and is not affiliated with PiyoLog or its operating company. Do not contact PiyoLog support or the operating company about this library.

## Rust

```rust
let input = std::fs::read_to_string("piyolog.txt")?;
let parsed = piyoparse::parse(&input)?;

for day in parsed.days {
    println!("{}: {} records", day.date, day.records.len());
}
```

The parser uses one tolerant parsing path for both iOS and Android exports. It does not try to classify the platform; if the export can be read, it is parsed.

Each `Record` has `date`, `time`, `data`, and the free-text `memo` field from the export. The record type and raw detail text are represented inside `Record.data`.

Record-specific parsed values are exposed through `Record.data`, a tagged enum. For WebAssembly/TypeScript callers it serializes as a discriminated object:

```ts
type BreastMilkOrder = "unspecified" | "left_then_right" | "right_then_left";
type WithDetail = { detail?: string };

type RecordData =
  | ({
      kind: "breastfeeding";
      left_minutes?: number;
      right_minutes?: number;
      order: BreastMilkOrder;
      amount_ml?: number;
    } & WithDetail)
  | ({ kind: "formula"; amount_ml?: number } & WithDetail)
  | ({ kind: "expressed_breast_milk"; amount_ml?: number } & WithDetail)
  | ({ kind: "drink"; amount_ml?: number } & WithDetail)
  | ({ kind: "wake_up"; duration_minutes?: number } & WithDetail)
  | ({ kind: "walks"; duration_minutes?: number } & WithDetail)
  | ({ kind: "pumping"; amount_ml?: number } & WithDetail)
  | ({ kind: "other"; type_name: string } & WithDetail)
  | ({ kind:
        | "poop"
        | "body_temp" | "height" | "weight" | "head_size" | "chest_size"
    } & WithDetail)
  | {
      kind:
        | "baths" | "sleep" | "pee"
        | "solid_food" | "snack" | "meal" | "cough" | "vomit"
        | "rash" | "injury" | "medicine" | "hospital" | "vaccine"
        | "milestone" | "others" | "notes";
    };
```

Known PiyoLog record types get dedicated variants. Unknown future types and custom items are parsed as `other` with `type_name`, so parser updates are not required just to read a newer export.

## Test fixtures

PiyoLog export files can contain meaningful trailing spaces. Fixture `.txt` files under `tests/fixtures/` intentionally keep those spaces, so avoid editing them with tools that automatically trim trailing whitespace.

## WebAssembly

Build with `wasm-pack` and enable the `wasm` feature:

```sh
wasm-pack build --target bundler --features wasm
```

Then call it from TypeScript:

```ts
import init, { parsePiyolog, parsePiyologJson } from "./pkg/piyoparse";

await init();

const parsed = parsePiyolog(exportText);
const json = parsePiyologJson(exportText);
```
