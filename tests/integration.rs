use serde_json::json;

#[test]
fn compiles_and_evaluates_a_basic_rule() {
    let program = nodora::compile(
        r#"
        rule AdultCheck {
            out is_adult = input.age >= 18
        }
        "#,
    )
    .expect("compile");

    let evaluator = program.evaluator().expect("evaluator");

    let adult = evaluator
        .evaluate("AdultCheck", &json!({ "age": 21 }))
        .expect("evaluate");
    assert_eq!(adult.outputs["is_adult"], json!(true));

    let minor = evaluator
        .evaluate("AdultCheck", &json!({ "age": 12 }))
        .expect("evaluate");
    assert_eq!(minor.outputs["is_adult"], json!(false));
}

#[test]
fn reuses_one_evaluator_across_inputs() {
    let program = nodora::compile(
        r#"
        rule Total {
            out doubled = input.n * 2
        }
        "#,
    )
    .expect("compile");
    let evaluator = program.evaluator().expect("evaluator");

    for n in 0..5 {
        let r = evaluator
            .evaluate("Total", &json!({ "n": n }))
            .expect("evaluate");
        assert_eq!(r.outputs["doubled"], json!(n * 2));
    }
}

#[test]
fn emits_signals() {
    let program = nodora::compile(
        r#"
        signal Greet(name)
        rule Hello {
            emit Greet(input.name)
        }
        "#,
    )
    .expect("compile");

    let result = program
        .evaluator()
        .expect("evaluator")
        .evaluate("Hello", &json!({ "name": "world" }))
        .expect("evaluate");

    assert_eq!(result.emitted_signals.len(), 1);
    assert_eq!(result.emitted_signals[0].name, "Greet");
    assert_eq!(result.emitted_signals[0].args, vec![json!("world")]);
}

#[test]
fn no_signal_when_condition_false() {
    let program = nodora::compile(
        r#"
        signal Alarm(level)
        rule Guard {
            emit Alarm("hot") when input.hot
        }
        "#,
    )
    .expect("compile");
    let evaluator = program.evaluator().expect("evaluator");

    let quiet = evaluator
        .evaluate("Guard", &json!({ "hot": false }))
        .expect("evaluate");
    assert!(quiet.emitted_signals.is_empty());

    let loud = evaluator
        .evaluate("Guard", &json!({ "hot": true }))
        .expect("evaluate");
    assert_eq!(loud.emitted_signals.len(), 1);
}

#[test]
fn compile_error_is_reported() {
    let err = nodora::compile("rule Broken { out x = }").unwrap_err();
    assert!(matches!(err, nodora::Error::Engine(_)));
}

#[test]
fn roundtrips_program_json() {
    let program = nodora::compile("rule R { out v = 1 + 2 }").expect("compile");
    let json = program.to_json();

    let reloaded = nodora::Program::from_json(&json).expect("reload");
    let result = reloaded
        .evaluator()
        .expect("evaluator")
        .evaluate("R", &json!({}))
        .expect("evaluate");
    assert_eq!(result.outputs["v"], json!(3));
}
