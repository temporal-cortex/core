# Temporal Cortex Core

Deterministic calendar tools for AI agents. Reduce token costs, eliminate date hallucinations, and detect scheduling conflicts — all without LLM inference.

## Why?

LLMs struggle with calendar operations:

- **Date hallucination** — "3rd Tuesday of each month at 2pm Pacific" requires deterministic math that LLMs get wrong, especially across DST transitions
- **Token waste** — a single Google Calendar API response can consume 4,000+ tokens of context window
- **No conflict detection** — LLMs can't reliably determine if two schedules overlap

Temporal Cortex Core solves these with two libraries:

| Library | What it does |
|---------|-------------|
| **TOON** | Token-Oriented Object Notation — compact format achieving 50%+ token reduction vs JSON |
| **Truth Engine** | Deterministic RRULE expansion, conflict detection, and free/busy computation |

## Packages

| Package | Install | Language |
|---------|---------|----------|
| `toon-core` | `cargo add toon-core` | Rust |
| `toon-cli` | `cargo install toon-cli` | CLI |
| `@temporal-cortex/toon` | `npm i @temporal-cortex/toon` | JavaScript (WASM) |
| `toon-format` | `pip install toon-format` | Python |
| `truth-engine` | `cargo add truth-engine` | Rust |
| `@temporal-cortex/truth-engine` | `npm i @temporal-cortex/truth-engine` | JavaScript (WASM) |

## Quick Start

### Rust

```rust
use toon_core::{encode, decode};

// Compress JSON to TOON (50%+ token reduction)
let json = r#"{"name":"Alice","scores":[95,87,92]}"#;
let toon = encode(json).unwrap();
assert_eq!(toon, "name: Alice\nscores[3]: 95,87,92");

// Perfect roundtrip
let back = decode(&toon).unwrap();
assert_eq!(back, json);
```

```rust
use truth_engine::{expand_rrule, find_conflicts, find_free_slots};

// Deterministic RRULE expansion with DST handling
let events = expand_rrule(
    "FREQ=MONTHLY;BYDAY=TU;BYSETPOS=3",  // 3rd Tuesday of each month
    "2026-02-17T14:00:00",                 // local time
    60,                                     // 60-minute duration
    "America/Los_Angeles",                  // IANA timezone
    Some("2026-12-31T23:59:59"),            // expand until
    None,                                   // no count limit
).unwrap();

// Detect scheduling conflicts
let conflicts = find_conflicts(&calendar_a, &calendar_b);

// Find available meeting slots
let free_slots = find_free_slots(&busy_events, window_start, window_end);
```

### JavaScript

```javascript
import { encode, decode } from '@temporal-cortex/toon';

const toon = encode('{"name":"Alice","age":30}');
// "name: Alice\nage: 30"

const json = decode(toon);
// '{"name":"Alice","age":30}'
```

### Python

```python
from toon_format import encode, decode, expand_rrule

toon = encode('{"name":"Alice","scores":[95,87,92]}')
# "name: Alice\nscores[3]: 95,87,92"

json_str = decode(toon)
# '{"name":"Alice","scores":[95,87,92]}'

events_json = expand_rrule(
    "FREQ=WEEKLY;BYDAY=TU,TH",
    "2026-02-17T14:00:00",
    60,
    "America/Los_Angeles",
    "2026-06-30T23:59:59",
    None,
)
```

## TOON Format

TOON (Token-Oriented Object Notation) is a compact, human-readable format that minimizes token usage when feeding structured data to LLMs.

**JSON** (317 bytes):
```json
{
  "summary": "Team Standup",
  "start": {"dateTime": "2024-01-15T09:00:00-08:00", "timeZone": "America/Los_Angeles"},
  "attendees": [
    {"email": "alice@company.com", "responseStatus": "accepted"},
    {"email": "bob@company.com", "responseStatus": "tentative"}
  ]
}
```

**TOON** (196 bytes, 38% smaller):
```
summary: Team Standup
start:
  dateTime: 2024-01-15T09:00:00-08:00
  timeZone: America/Los_Angeles
attendees[2]{email,responseStatus}:
  alice@company.com,accepted
  bob@company.com,tentative
```

Key features: key folding (indentation replaces braces), tabular arrays (CSV-like rows for uniform objects), inline arrays, and context-dependent quoting.

## Truth Engine

Deterministic calendar math that LLMs cannot reliably perform:

- **RRULE expansion** — full RFC 5545 support (FREQ, BYDAY, BYSETPOS, COUNT, UNTIL, EXDATE)
- **DST-aware** — events at 14:00 Pacific stay at 14:00 Pacific across DST transitions
- **Conflict detection** — pairwise overlap detection with duration calculation
- **Free/busy computation** — merge busy periods and find available slots
- **Leap year handling** — `BYMONTHDAY=29` correctly skips non-leap years

## Development

This project follows strict **Test-Driven Development** (Red-Green-Refactor). No production code is written without a corresponding test first.

### Prerequisites

- Rust 1.88+ (1.93 recommended) with `wasm32-unknown-unknown` target
- Node.js 18+ with pnpm
- Python 3.12+ (for toon-python bindings)
- `wasm-bindgen-cli` (`cargo install wasm-bindgen-cli`)

### Running Tests

```bash
# All Rust tests
cargo test --workspace

# Individual crates
cargo test -p toon-core       # encoder + decoder + roundtrip + spec + proptest
cargo test -p toon-cli         # CLI integration tests
cargo test -p truth-engine     # expander + conflict + freebusy + RFC 5545 + proptest

# Code quality
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check

# WASM + JavaScript tests
cargo build -p toon-wasm -p truth-engine-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs target/wasm32-unknown-unknown/release/toon_wasm.wasm --out-dir packages/toon-js/wasm/
wasm-bindgen --target nodejs target/wasm32-unknown-unknown/release/truth_engine_wasm.wasm --out-dir packages/truth-engine-js/wasm/
pnpm install && pnpm test

# Python tests
cd crates/toon-python
python3 -m venv .venv && source .venv/bin/activate
pip install maturin pytest
maturin develop
pytest tests/ -v
```

### Repository Structure

```
temporal-cortex-core/
├── crates/
│   ├── toon-core/           # TOON encoder/decoder + semantic filtering
│   ├── toon-cli/            # CLI: toon encode | decode | stats | --filter
│   ├── toon-wasm/           # WASM bindings for toon-core
│   ├── toon-python/         # Python bindings via PyO3
│   ├── truth-engine/        # RRULE expansion + conflict + free/busy
│   └── truth-engine-wasm/   # WASM bindings for truth-engine
├── packages/
│   ├── toon-js/             # @temporal-cortex/toon (NPM)
│   └── truth-engine-js/     # @temporal-cortex/truth-engine (NPM)
└── scripts/poc/             # Test fixtures
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT OR Apache-2.0
