// Run with: `cargo run --example basic`

use serde_json::json;

fn main() -> Result<(), nodora::Error> {
    let program = nodora::compile(
        r#"
        rule AdultCheck {
            out is_adult = input.age >= 18
        }
        "#,
    )?;

    let evaluator = program.evaluator()?;
    let result = evaluator.evaluate("AdultCheck", &json!({ "age": 21 }))?;

    println!("is_adult = {}", result.outputs["is_adult"]);
    Ok(())
}
