use web_sys::HtmlCanvasElement;

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

pub fn resize_canvas(canvas: HtmlCanvasElement) {
    let new_width = window().inner_width().unwrap().as_f64().unwrap();
    let new_height = window().inner_height().unwrap().as_f64().unwrap();
    canvas.set_width(new_width as u32);
    canvas.set_height(new_height as u32);
}