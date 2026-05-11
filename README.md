# piyoparse

Rust parser for PiyoLog export files. The crate can parse both iOS and Android export layouts, and can also be built for WebAssembly so TypeScript code can call it.

This is an unofficial tool and is not affiliated with PiyoLog or its operating company. Do not contact PiyoLog support or the operating company about this library.  
本ツールは非公式であり、ぴよログおよびその運営会社とは一切関係ありません。本ライブラリについて、ぴよログのサポート窓口や運営会社へのお問い合わせはお控えください。

## Known limitations

Only Japanese PiyoLog exports are currently supported and tested. This applies to both iOS and Android export layouts. Exports from other app languages/locales are not supported because record type names, summary labels, and detail text are locale-specific.

Typed amount parsing currently supports only milliliters (`ml`). Other volume units, such as ounces, are not supported.

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

Record-specific parsed values are exposed through `Record.data`, a tagged enum. For WebAssembly/TypeScript callers it serializes as a discriminated object. The generated npm package includes TypeScript definitions for these shapes.

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

## WebAssembly and npm

Build the npm package with `wasm-pack` and enable the `wasm` feature:

```sh
wasm-pack build --target bundler --features wasm
```

The generated `pkg/` directory is the npm package. Review it, then publish from that directory:

```sh
cd pkg
npm publish
```

Then call it from TypeScript. `parsePiyolog` returns a typed `ParsedExport`; `parsePiyologJson` returns the JSON string form. The generated definitions also export supporting types such as `Day`, `PiyologRecord`, `RecordData`, and `DaySummary`.

```ts
import { parsePiyolog, parsePiyologJson, type ParsedExport } from "piyoparse";

const parsed: ParsedExport = parsePiyolog(exportText);
const json = parsePiyologJson(exportText);
```

The repository also includes a TypeScript runtime test that type-checks against the generated `.d.ts`, runs the generated Node.js wasm package, and parses a real fixture:

```sh
npm install
npm run test:ts
```
