use std::ffi::{c_char, c_longlong, CStr, CString};

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod ffi {
    use super::{c_char, c_longlong};

    extern "C" {
        pub fn NodoraCompile(src: *const c_char) -> *mut c_char;
        pub fn NodoraNewEvaluator(ruleset_json: *const c_char) -> *mut c_char;
        pub fn NodoraEvaluate(
            id: c_longlong,
            rule_name: *const c_char,
            input_json: *const c_char,
        ) -> *mut c_char;
        pub fn NodoraDestroyEvaluator(id: c_longlong);
        pub fn NodoraFree(p: *mut c_char);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("nodora: {0}")]
    Engine(String),
    #[error("invalid argument: contains interior NUL byte")]
    NulByte(#[from] std::ffi::NulError),
    #[error("failed to serialize input: {0}")]
    SerializeInput(serde_json::Error),
    #[error("malformed response from engine: {0}")]
    MalformedResponse(String),
    #[error("engine returned a null response")]
    NullResponse,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Ruleset {
    value: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvaluationResult {
    #[serde(default)]
    pub outputs: serde_json::Map<String, Value>,
    #[serde(default)]
    pub emitted_signals: Vec<EmittedSignal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmittedSignal {
    pub name: String,
    #[serde(default)]
    pub args: Vec<Value>,
}

pub fn compile(src: &str) -> Result<Ruleset> {
    let src = CString::new(src)?;
    let data = call(|| unsafe { ffi::NodoraCompile(src.as_ptr()) })?;
    Ok(Ruleset { value: data })
}

impl Ruleset {
    pub fn from_json(json: &str) -> Result<Ruleset> {
        let value: Value =
            serde_json::from_str(json).map_err(|e| Error::MalformedResponse(e.to_string()))?;
        Ok(Ruleset { value })
    }

    pub fn as_value(&self) -> &Value {
        &self.value
    }

    pub fn to_json(&self) -> String {
        self.value.to_string()
    }

    pub fn evaluator(&self) -> Result<Evaluator> {
        let ruleset_json = CString::new(self.value.to_string())?;
        let data = call(|| unsafe { ffi::NodoraNewEvaluator(ruleset_json.as_ptr()) })?;
        let id = data
            .as_i64()
            .ok_or_else(|| Error::MalformedResponse("expected evaluator handle".into()))?;
        Ok(Evaluator { id })
    }
}

#[derive(Debug)]
pub struct Evaluator {
    id: i64,
}

impl Evaluator {
    pub fn evaluate<I>(&self, rule: &str, input: &I) -> Result<EvaluationResult>
    where
        I: Serialize,
    {
        let input_json = serde_json::to_string(input).map_err(Error::SerializeInput)?;
        let rule_c = CString::new(rule)?;
        let input_c = CString::new(input_json)?;
        let data = call(|| unsafe {
            ffi::NodoraEvaluate(self.id as c_longlong, rule_c.as_ptr(), input_c.as_ptr())
        })?;
        serde_json::from_value(data).map_err(|e| Error::MalformedResponse(e.to_string()))
    }
}

impl Drop for Evaluator {
    fn drop(&mut self) {
        unsafe { ffi::NodoraDestroyEvaluator(self.id as c_longlong) }
    }
}

/// invokes an FFI function that returns a `{"data": ...}` / `{"error": ...}`
/// JSON envelope as an owned C string, frees that string, and unwraps the
/// envelope into the inner `data` value
fn call<F>(f: F) -> Result<Value>
where
    F: FnOnce() -> *mut c_char,
{
    let ptr = f();
    if ptr.is_null() {
        return Err(Error::NullResponse);
    }

    let json = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::NodoraFree(ptr) };

    let mut envelope: Value =
        serde_json::from_str(&json).map_err(|e| Error::MalformedResponse(e.to_string()))?;
    if let Some(err) = envelope.get("error").and_then(Value::as_str) {
        return Err(Error::Engine(err.to_string()));
    }
    match envelope.get_mut("data") {
        Some(data) => Ok(data.take()),
        None => Err(Error::MalformedResponse("missing `data` field".into())),
    }
}
