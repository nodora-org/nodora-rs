# nodora-rs

Rust bindings for the [Nodora](https://nodora.org) rule engine.

### Install

Run the following cargo command in your project directory:

```bash
cargo add nodora
```

### Prebuilt targets

| Target                      |     |
| --------------------------- | --- |
| `x86_64-unknown-linux-gnu`  | ✅  |
| `aarch64-unknown-linux-gnu` | ✅  |
| `x86_64-apple-darwin`       | ✅  |
| `aarch64-apple-darwin`      | ✅  |

Other targets build from source (needs Go 1.24+ with cgo).

## Usage

```rust
use serde_json::json;

let program = nodora::compile(r#"
    rule AdultCheck {
        out is_adult = input.age >= 18
    }
"#)?;

let evaluator = program.evaluator()?;
let result = evaluator.evaluate("AdultCheck", &json!({ "age": 21 }))?;

assert_eq!(result.outputs["is_adult"], json!(true));
# Ok::<(), nodora::Error>(())
```

Signals emitted by a rule are returned in the result:

```rust
# let result: nodora::EvaluationResult = Default::default();
for signal in &result.emitted_signals {
    println!("{}({:?})", signal.name, signal.args);
}
```

A precompiled program (e.g. the output of `nodora compile`) can be loaded
without recompiling the source:

```rust
# let program_json = "{}";
let program = nodora::Program::from_json(program_json)?;
# Ok::<(), nodora::Error>(())
```

## API

| Item                                                    | Purpose                                    |
| ------------------------------------------------------- | ------------------------------------------ |
| `compile(src) -> Program`                               | Compile Nodora source to a program.        |
| `Program::from_json(json) -> Program`                   | Load a precompiled program.                |
| `Program::evaluator() -> Evaluator`                     | Build a reusable evaluator.                |
| `Evaluator::evaluate(rule, &input) -> EvaluationResult` | Run a rule against any serializable input. |
| `EvaluationResult { outputs, emitted_signals }`         | Named outputs and emitted signals.         |

## Development

Building inside this repo forces a source build (via `.cargo/config.toml`), so a
Go 1.24+ toolchain with cgo is required for development:

```sh
cargo test                  # build the bridge from source + run the suite
cargo run --example basic   # run the example
```

## License

Apache-2.0.
