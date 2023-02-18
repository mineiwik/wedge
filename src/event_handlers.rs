use std::{rc::Rc, cell::RefCell};
use std::f32::consts::PI;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{EventTarget, HtmlCanvasElement, Event, MouseEvent, WheelEvent};
use crate::constants::WHEEL_DRAG;
use crate::utils::{window, resize_canvas};

pub fn set_event_handlers(canvas: HtmlCanvasElement, zoom: Rc<RefCell<f32>>, drag: Rc<RefCell<bool>>, theta: Rc<RefCell<f32>>, phi: Rc<RefCell<f32>>, dx: Rc<RefCell<f32>>, dy: Rc<RefCell<f32>>) {
    let event_target: EventTarget = canvas.clone().into();
    // RESIZE
    {
        let canvas = canvas.clone();
        let resize_cb = Closure::wrap(Box::new(move |_event: Event| {
            resize_canvas(canvas.clone());
        }) as Box<dyn FnMut(Event)>);
        window().add_event_listener_with_callback("resize", resize_cb.as_ref().unchecked_ref()).unwrap();
        resize_cb.forget();
    }

    // ZOOM
    {
        let zoom = zoom.clone();
        let zoom_cb = Closure::wrap(Box::new(move |event: WheelEvent| {
            *zoom.borrow_mut() += event.delta_y() as f32 / WHEEL_DRAG;
        }) as Box<dyn FnMut(WheelEvent)>);
        event_target.add_event_listener_with_callback("wheel", zoom_cb.as_ref().unchecked_ref()).unwrap();
        zoom_cb.forget();
    }

    // MOUSEDOWN
    {
        let drag = drag.clone();
        let mousedown_cb = Closure::wrap(Box::new(move |_event: MouseEvent| {
            *drag.borrow_mut() = true;
        }) as Box<dyn FnMut(MouseEvent)>);
        event_target
            .add_event_listener_with_callback("mousedown", mousedown_cb.as_ref().unchecked_ref())
            .unwrap();
        mousedown_cb.forget();
    }
    // MOUSEUP and MOUSEOUT
    {
        let drag = drag.clone();
        let mouseup_cb = Closure::wrap(Box::new(move |_event: MouseEvent| {
            *drag.borrow_mut() = false;
        }) as Box<dyn FnMut(MouseEvent)>);
        event_target
            .add_event_listener_with_callback("mouseup", mouseup_cb.as_ref().unchecked_ref())
            .unwrap();
        event_target
            .add_event_listener_with_callback("mouseout", mouseup_cb.as_ref().unchecked_ref())
            .unwrap();
        mouseup_cb.forget();
    }
    // MOUSEMOVE
    {
        let mousemove_cb = Closure::wrap(Box::new(move |event: MouseEvent| {
            let canvas_width = Rc::new(RefCell::new(canvas.width() as f32));
            let canvas_height = Rc::new(RefCell::new(canvas.height() as f32));
            if *drag.borrow() {
                let cw = *canvas_width.borrow();
                let ch = *canvas_height.borrow();
                *dx.borrow_mut() = (event.movement_x() as f32) * 2.0 * PI / cw;
                *dy.borrow_mut() = (event.movement_y() as f32) * 2.0 * PI / ch;
                *theta.borrow_mut() += *dx.borrow();
                *phi.borrow_mut() += *dy.borrow();
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
        event_target
            .add_event_listener_with_callback("mousemove", mousemove_cb.as_ref().unchecked_ref())
            .unwrap();
        mousemove_cb.forget();
    }
}