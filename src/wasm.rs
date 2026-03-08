use wasm_bindgen::prelude::*;

use crate::{ParseError, Parser};

fn to_js_error(error: ParseError) -> JsError {
    JsError::new(&error.to_string())
}

#[wasm_bindgen(js_name = SseParser)]
pub struct WasmParser {
    inner: Parser,
}

#[wasm_bindgen(js_class = SseParser)]
impl WasmParser {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Parser::new(),
        }
    }

    #[wasm_bindgen(js_name = feedBytes)]
    pub fn feed_bytes(&mut self, chunk: &[u8]) -> Result<JsValue, JsError> {
        let events = self.inner.feed(chunk).map_err(to_js_error)?;
        serde_wasm_bindgen::to_value(&events).map_err(|error| JsError::new(&error.to_string()))
    }

    #[wasm_bindgen(js_name = feedText)]
    pub fn feed_text(&mut self, chunk: &str) -> Result<JsValue, JsError> {
        self.feed_bytes(chunk.as_bytes())
    }

    pub fn finish(&mut self) -> Result<JsValue, JsError> {
        let events = self.inner.finish().map_err(to_js_error)?;
        serde_wasm_bindgen::to_value(&events).map_err(|error| JsError::new(&error.to_string()))
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[wasm_bindgen(getter, js_name = lastEventId)]
    pub fn last_event_id(&self) -> Option<String> {
        self.inner.last_event_id().map(str::to_owned)
    }

    #[wasm_bindgen(getter)]
    pub fn retry(&self) -> Option<f64> {
        self.inner.retry().map(|retry| retry as f64)
    }
}
