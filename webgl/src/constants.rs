use web_sys::WebGl2RenderingContext;

pub const WS_URL: &str = "wss://nx-hoster-sandbox.taco.kennysbasement.com/ws_deflated";

// Constants
pub const FPS: u8 = 60u8;
pub const MS_PER_TICK: f32 = 1000f32 / FPS as f32;

pub const TARGET: u32 = WebGl2RenderingContext::TEXTURE_2D;
pub const LEVEL: i32 = 0;
pub const INTERNAL_FORMAT: i32 = WebGl2RenderingContext::RGBA as i32;
pub const BORDER: i32 = 0;
pub const SRC_FORMAT: u32 = WebGl2RenderingContext::RGBA;
pub const SRC_TYPE: u32 = WebGl2RenderingContext::UNSIGNED_BYTE;

pub const INSTANCED_DRAW: bool = false;
pub const KEY_ACTIVATION_DELAY: f32 = 15f32;