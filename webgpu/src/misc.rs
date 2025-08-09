
use wasm_bindgen::JsCast;
use web_sys::{console, HtmlCanvasElement};
use web_sys::js_sys;
pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

pub fn canvas() -> HtmlCanvasElement {
    let document = window().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    canvas.dyn_into::<HtmlCanvasElement>().unwrap()
}

pub async fn sleep(millis: i32) {
    let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
        window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis)
            .unwrap();
    };
    let p = js_sys::Promise::new(&mut cb);
    wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
}

pub fn log(s: &str) {
    console::log_1(&s.into());
}

pub fn debug(s: &str) {
    console::debug_1(&s.into());
}

pub fn warn(s: &str) {
    console::warn_1(&s.into());
}

pub fn error(s: &str) {
    console::error_1(&s.into());
}

pub fn now() -> f32 { window().performance().unwrap().now() as f32 }

pub async fn read_buffer(buffer: &wgpu::Buffer, size: usize) -> Vec<u8> {
    use wgpu::{Buffer, MapMode};
    use futures_intrusive::channel::shared::oneshot_channel;
    let slice = buffer.slice(..);
    let (sender, receiver) = oneshot_channel();

    slice.map_async(MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    receiver.receive().await.unwrap().unwrap();

    let data = slice.get_mapped_range().to_vec();

    buffer.unmap(); // always unmap after reading
    data
}