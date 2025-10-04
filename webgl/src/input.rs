use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use gloo::events::EventListener;
use wasm_bindgen::JsCast;
use crate::misc::{window};
use web_sys::{Event, KeyboardEvent, MouseEvent, FocusEvent};

pub struct Input {
}

impl Input {
    pub fn init_down(pressed_keys: Arc<Mutex<HashMap<String, f32>>>) {
        let document = window().document().unwrap();

        let on_keydown = EventListener::new(&document, "keydown", move |event| {

            let ke = event.clone()
                .dyn_into::<web_sys::KeyboardEvent>()
                .unwrap();
            
            let mut keys = pressed_keys.lock().unwrap();
            if !keys.contains_key(&ke.key().to_lowercase()) {
                keys.insert(ke.key().to_lowercase(), 0f32);
            }

        });

        on_keydown.forget();
    }
    
    pub fn init_up(pressed_keys: Arc<Mutex<HashMap<String, f32>>>) {
        let document = window().document().unwrap();

        let on_keyup = EventListener::new(&document, "keyup", move |event| {

            let ke = event.clone()
                .dyn_into::<web_sys::KeyboardEvent>()
                .unwrap();

            let mut keys = pressed_keys.lock().unwrap();
            if keys.contains_key(&ke.key().to_lowercase()) {
                keys.remove(&ke.key().to_lowercase());
            }
            

        });

        on_keyup.forget();
    }
    
    pub fn handle_input() {
        
    }
    
    pub fn init_mouse(mouse_position: Arc<Mutex<(i32, i32)>>) {
        let document = window().document().unwrap();

        let on_keyup = EventListener::new(&document, "mousemove", move |event| {

            let me = event.clone()
                .dyn_into::<web_sys::MouseEvent>()
                .unwrap();
            
            let mut mouse_pos = mouse_position.lock().unwrap();
            mouse_pos.0 = me.client_x();
            mouse_pos.1 = me.client_y();

            // print(&format!("Mouse X: {} Mouse Y: {}", me.client_x(), me.client_y()));
        });

        on_keyup.forget();
    }

    pub fn init_focus_lost_blur(pressed_keys: Arc<Mutex<HashMap<String, f32>>>) {
        let document = window().document().unwrap();

        let on_loss = EventListener::new(&document, "blur", move |event| {
            // let e = event.clone()
            //     .dyn_into::<web_sys::FocusEvent>()
            //     .unwrap();
            let mut k = pressed_keys.lock().unwrap();
            k.clear();
        });

        on_loss.forget();
    }

    pub fn init_focus_lost_visibilitychange(pressed_keys: Arc<Mutex<HashMap<String, f32>>>) {
        let document = window().document().unwrap();

        let on_loss = EventListener::new(&document, "visibilitychange", move |event| {
            // let e = event.clone()
            //     .dyn_into::<web_sys::FocusEvent>()
            //     .unwrap();
            let mut k = pressed_keys.lock().unwrap();
            k.clear();
        });

        on_loss.forget();
    }
}