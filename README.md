# piyoparse

`piyoparse` parses Japanese PiyoLog export text into structured data.

- Rust crate: [piyoparse on crates.io](https://crates.io/crates/piyoparse)
- TypeScript/WebAssembly package: [`@necocen/piyoparse` on npm](https://www.npmjs.com/package/@necocen/piyoparse)
- Supports Japanese iOS and Android day/month exports
- Preserves unknown future record types and renamed custom records as `other`

This is an unofficial tool and is not affiliated with PiyoLog or its operating company. Do not contact PiyoLog support or the operating company about this library.  
本ツールは非公式であり、ぴよログおよびその運営会社とは一切関係ありません。本ライブラリについて、ぴよログのサポート窓口や運営会社へのお問い合わせはお控えください。

## Installation

### Rust

```sh
cargo add piyoparse
```

Or add it to `Cargo.toml`:

```toml
[dependencies]
piyoparse = "0.1"
```

### TypeScript

```sh
npm install @necocen/piyoparse
```

## Rust Usage

```rust
use piyoparse::{parse, RecordData};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string("piyolog.txt")?;
    let parsed = parse(&input)?;

    for day in &parsed.days {
        println!("{}: {} records", day.date, day.records.len());

        for record in &day.records {
            if let RecordData::Formula { amount_ml, .. } = &record.data {
                println!("formula at {}: {:?} ml", record.time, amount_ml);
            }
        }
    }

    Ok(())
}
```

## TypeScript Usage

```ts
import { parsePiyolog, type ParsedExport } from "@necocen/piyoparse";

const parsed: ParsedExport = parsePiyolog(exportText);

for (const record of parsed.days[0]?.records ?? []) {
  if (record.data.kind === "formula") {
    console.log(record.time, record.data.amount_ml);
  }
}
```

`parsePiyolog` returns typed objects directly. `parsePiyologJson` is also available when a JSON string is easier to pass through an application boundary.

## Data Model

The parser returns a `ParsedExport`:

- `ParsedExport.days`: parsed day blocks from a day or month export
- `Day.records`: timestamped records for that day
- `Day.summary`: summary totals printed by PiyoLog
- `Record.memo`: free-text memo attached to a record, when present
- `Record.data`: tagged record data

Known PiyoLog record types are represented as `RecordData` variants in Rust and as discriminated objects in TypeScript. For example, `formula` records expose `amount_ml`, `wake_up` records expose `duration_minutes`, and `breastfeeding` records expose left/right minutes, order, and amount when they can be parsed.

Unknown future record types and renamed custom items are parsed as `other` with the original `type_name` and raw `detail`, so the export can still be read even when the parser does not know the record type yet.

## Supported Input

`piyoparse` uses one tolerant parser for both iOS and Android export layouts. Callers do not need to detect the platform first.

Currently supported:

- Japanese PiyoLog exports
- iOS and Android export text
- Day exports and month exports
- Typed parsing for common records such as breastfeeding, formula, expressed breast milk, drink, wake-up, walks, and pumping
- Summary totals for breastfeeding, formula, expressed breast milk, sleep, pee, and poop

## Known Limitations

- Only Japanese PiyoLog exports are supported. Exports from other app languages/locales are not supported because record type names, summary labels, and detail text are locale-specific.
- Typed amount parsing supports only milliliters (`ml`). Other volume units, such as ounces, are not parsed into `amount_ml`.
- This crate parses the text export format. It is not a complete model of every field in the PiyoLog app.

## Development

Run the Rust tests:

```sh
cargo test
cargo test --features wasm
```

Run the TypeScript runtime test:

```sh
npm install
npm run test:ts
```

Build the WebAssembly npm package for local verification or publishing:

```sh
wasm-pack build --release --target bundler --scope necocen --features wasm
```

This step is for maintainers. Application code should install the published package with `npm install @necocen/piyoparse` instead of building it locally.

PiyoLog export files can contain meaningful trailing spaces. Fixture `.txt` files under `tests/fixtures/` intentionally keep those spaces, so avoid editing them with tools that automatically trim trailing whitespace.
