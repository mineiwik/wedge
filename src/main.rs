use constants::{AMORTIZATION, FIELD_OF_VIEW, VERTICES_PER_FACET};
use js_sys::{Uint8Array, WebAssembly};
use std::cell::RefCell;
use std::rc::Rc;
use utils::{compile_shader, link_program, request_animation_frame, resize_canvas, window};
use wasm_bindgen::prelude::*;
use web_sys::{
    console, Event, FileReader, HtmlDivElement, HtmlInputElement, WebGlBuffer, WebGlProgram,
    WebGlRenderingContext, WebGlUniformLocation,
};

mod constants;
mod event_handlers;
mod linalg;
mod stl;
mod utils;

#[derive(Debug, Clone)]
struct ProgramInfo(
    WebGlProgram,
    u32,
    (
        Result<WebGlUniformLocation, String>,
        Result<WebGlUniformLocation, String>,
    ),
);

#[derive(Debug, Clone)]
struct Buffers(WebGlBuffer, WebGlBuffer);

fn main() {
    set_panic_hook();
    set_file_reader().unwrap()
}

fn set_file_reader() -> Result<(), JsValue> {
    let document = window()
        .document()
        .expect("should have a document on window");
    let file_in_div = document.get_element_by_id("file-input-div").unwrap();
    let file_in_div: HtmlDivElement = file_in_div.dyn_into()?;

    let fileinput: HtmlInputElement = document
        .create_element("input")?
        .dyn_into::<HtmlInputElement>()?;

    fileinput.set_id("file-input");
    fileinput.set_class_name("file-input");
    fileinput.set_type("file");

    let filereader = FileReader::new()?;

    let closure = Closure::wrap(Box::new(move |event: Event| {
        let element = event.target().unwrap().dyn_into::<FileReader>().unwrap();
        let buffer = Uint8Array::new(&element.result().unwrap());
        let v = buffer.to_vec();

        match stl::get_data(&v) {
            Ok((vertices, num_vertices)) => another(vertices, num_vertices).unwrap(),
            Err(e) => console::log_1(&format!("The given file is corrupted: Error: {}", e).into()),
        }
    }) as Box<dyn FnMut(_)>);

    filereader.set_onloadend(Some(closure.as_ref().unchecked_ref()));
    closure.forget();

    let closure = Closure::wrap(Box::new(move |event: Event| {
        let element = event
            .target()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let filelist = element.files().unwrap();
        let file = filelist.get(0).expect("should have a file handle.");
        filereader.read_as_array_buffer(&file).unwrap();
    }) as Box<dyn FnMut(_)>);

    fileinput.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())?;
    closure.forget();

    file_in_div.append_child(&fileinput)?;
    Ok(())
}

fn another(vertices: Vec<f32>, num_vertices: u32) -> Result<(), JsValue> {
    let document = window()
        .document()
        .expect("should have a document on window");

    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let gl = canvas
        .get_context("webgl")?
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()?;

    let vertex_shader_source = r#"
        attribute vec4 aVertexPosition;
        uniform mat4 uModelViewMatrix;
        uniform mat4 uProjectionMatrix;

        varying lowp vec4 vColor;
        
        void main(void) {
            gl_Position = uProjectionMatrix * uModelViewMatrix * aVertexPosition;
            vColor = aVertexPosition;
        }
    "#;

    let fragment_shader_source = r#"
        varying lowp vec4 vColor;
        void main(void) {
            gl_FragColor = vColor;
        }
    "#;

    let shader_program = init_shader_program(&gl, vertex_shader_source, fragment_shader_source)?;

    let programm_info = {
        let vertex_pos = gl.get_attrib_location(&shader_program, "aVertexPosition") as u32;
        let projection_matrix = gl
            .get_uniform_location(&shader_program, "uProjectionMatrix")
            .ok_or_else(|| String::from("cannot get uProjectionMatrix"));
        let model_view_matric = gl
            .get_uniform_location(&shader_program, "uModelViewMatrix")
            .ok_or_else(|| String::from("cannot get uModelViewMatrix"));
        ProgramInfo(
            shader_program,
            vertex_pos,
            (projection_matrix, model_view_matric),
        )
    };

    // objects we'll be drawing.
    let buffers: Buffers = init_buffers(&gl, vertices, num_vertices)?;

    // Draw the scene repeatedly
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    let zoom = Rc::new(RefCell::new(-5.0));
    let drag = Rc::new(RefCell::new(false));
    let theta = Rc::new(RefCell::new(0.0));
    let phi = Rc::new(RefCell::new(0.0));
    let dx = Rc::new(RefCell::new(0.0));
    let dy = Rc::new(RefCell::new(0.0));

    // Define event handlers
    event_handlers::set_event_handlers(
        canvas.clone(),
        zoom.clone(),
        drag.clone(),
        theta.clone(),
        phi.clone(),
        dx.clone(),
        dy.clone(),
    );

    // Resize canvas to fit window
    resize_canvas(canvas.clone());

    // RequestAnimationFrame
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_d| {
        if !*drag.borrow() {
            *dx.borrow_mut() *= AMORTIZATION;
            *dy.borrow_mut() *= AMORTIZATION;
            *theta.borrow_mut() += *dx.borrow();
            *phi.borrow_mut() += *dy.borrow();
        }
        draw_scene(
            &gl.clone(),
            programm_info.clone(),
            buffers.clone(),
            *zoom.borrow(),
            *theta.borrow(),
            *phi.borrow(),
            num_vertices,
            &canvas,
        )
        .unwrap();
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut(f32)>));

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}

fn init_shader_program(
    gl: &WebGlRenderingContext,
    vs_source: &str,
    fs_source: &str,
) -> Result<WebGlProgram, String> {
    let v_shader = compile_shader(gl, WebGlRenderingContext::VERTEX_SHADER, vs_source);
    let f_shader = compile_shader(gl, WebGlRenderingContext::FRAGMENT_SHADER, fs_source);

    link_program(gl, &v_shader?, &f_shader?)
}

fn init_buffers(
    gl: &WebGlRenderingContext,
    vertices: Vec<f32>,
    num_vertices: u32,
) -> Result<Buffers, JsValue> {
    let position_buffer = gl
        .create_buffer()
        .ok_or("failed to create positionBuffer buffer")?;

    gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));

    let position_array = float_32_array!(vertices);
    gl.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ARRAY_BUFFER,
        &position_array,
        WebGlRenderingContext::STATIC_DRAW,
    );

    let index_buffer = gl
        .create_buffer()
        .ok_or("failed to create indexBuffer buffer")?;
    gl.bind_buffer(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        Some(&index_buffer),
    );

    let mut indices: Vec<u16> = vec![];

    for i in 0..num_vertices {
        indices.push(i as u16);
    }

    let index_array = uint_16_array!(indices);
    gl.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        &index_array,
        WebGlRenderingContext::STATIC_DRAW,
    );
    Ok(Buffers(position_buffer, index_buffer))
}

#[allow(clippy::too_many_arguments)]
fn draw_scene(
    gl: &WebGlRenderingContext,
    program_info: ProgramInfo,
    buffers: Buffers,
    zoom: f32,
    theta: f32,
    phi: f32,
    num_vertices: u32,
    canvas: &web_sys::HtmlCanvasElement,
) -> Result<(), JsValue> {
    let Buffers(position_buffer, index_buffer) = buffers;
    let ProgramInfo(
        shader_program,
        vertex_position,
        (location_projection_matrix, location_model_view_matrix),
    ) = program_info;
    gl.clear_color(0.375, 0.375, 0.375, 1.0);
    gl.clear_depth(1.0);
    gl.enable(WebGlRenderingContext::DEPTH_TEST);

    gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT | WebGlRenderingContext::DEPTH_BUFFER_BIT);

    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);

    let aspect: f32 = canvas.width() as f32 / canvas.height() as f32;
    let z_near = 1.0;
    let z_far = 100.0;

    let mut projection_matrix = mat4::new_zero();

    mat4::perspective(
        &mut projection_matrix,
        &FIELD_OF_VIEW,
        &aspect,
        &z_near,
        &z_far,
    );

    let mut model_view_matrix = mat4::new_identity();

    let mat_to_translate = model_view_matrix;
    mat4::translate(&mut model_view_matrix, &mat_to_translate, &[0.0, 0.0, zoom]);

    let mat_to_rotate = model_view_matrix;
    mat4::rotate_x(&mut model_view_matrix, &mat_to_rotate, &phi);
    let mat_to_rotate = model_view_matrix;
    mat4::rotate_y(&mut model_view_matrix, &mat_to_rotate, &theta);

    gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));
    gl.vertex_attrib_pointer_with_i32(
        vertex_position,
        VERTICES_PER_FACET,
        WebGlRenderingContext::FLOAT,
        false,
        0,
        0,
    );
    gl.enable_vertex_attrib_array(vertex_position);
    gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, None);

    gl.bind_buffer(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        Some(&index_buffer),
    );

    gl.use_program(Some(&shader_program));

    gl.uniform_matrix4fv_with_f32_array(
        Some(&location_projection_matrix?),
        false,
        &projection_matrix,
    );
    gl.uniform_matrix4fv_with_f32_array(
        Some(&location_model_view_matrix?),
        false,
        &model_view_matrix,
    );
    gl.draw_elements_with_i32(
        WebGlRenderingContext::TRIANGLES,
        num_vertices as i32,
        WebGlRenderingContext::UNSIGNED_SHORT,
        0,
    );

    Ok(())
}

fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
