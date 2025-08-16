use wasm_bindgen::JsCast;

pub mod worker;

pub fn get_canvas_element(selectors: &str) -> web_sys::HtmlCanvasElement {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let element = document.query_selector(selectors).unwrap().unwrap();
    let canvas = element.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    let scale_factor = window.device_pixel_ratio();
    let width = (canvas.client_width() as f64 * scale_factor) as u32;
    let height = (canvas.client_height() as f64 * scale_factor) as u32;
    canvas.set_width(width);
    canvas.set_height(height);
    canvas
}
