use js_sys::{Uint8Array, WebAssembly};
use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{
    console, Event, FileReader, HtmlDivElement, HtmlInputElement,
    WebGlBuffer, WebGlProgram, WebGlRenderingContext, WebGlShader, WebGlUniformLocation,
};
use utils::{window, resize_canvas};

mod stl;
mod event_handlers;
mod utils;
mod constants;

const AMORTIZATION: f32 = 0.95;

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

    let shader_program = initShaderProgram(&gl, vertex_shader_source, fragment_shader_source)?;

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
    let buffers: Buffers = initBuffers(&gl, vertices, num_vertices)?;

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
    event_handlers::set_event_handlers(canvas.clone(), zoom.clone(), drag.clone(), theta.clone(), phi.clone(), dx.clone(), dy.clone());
    
    // Resize canvas to fit window
    resize_canvas(canvas);

    // RequestAnimationFrame
    {
        // Request animation frame
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_d| {
            if !*drag.borrow() {
                *dx.borrow_mut() *= AMORTIZATION;
                *dy.borrow_mut() *= AMORTIZATION;
                *theta.borrow_mut() += *dx.borrow();
                *phi.borrow_mut() += *dy.borrow();
            }
            drawScene(
                &gl.clone(),
                programm_info.clone(),
                buffers.clone(),
                *zoom.borrow(),
                *theta.borrow(),
                *phi.borrow(),
                num_vertices,
            )
            .unwrap();
            request_animation_frame(f.borrow().as_ref().unwrap());
        }) as Box<dyn FnMut(f32)>));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }
    Ok(())
}

#[allow(non_snake_case)]
fn initShaderProgram(
    gl: &WebGlRenderingContext,
    vsSource: &str,
    fsSource: &str,
) -> Result<WebGlProgram, String> {
    let v_shader = compile_shader(gl, WebGlRenderingContext::VERTEX_SHADER, vsSource);
    let f_shader = compile_shader(gl, WebGlRenderingContext::FRAGMENT_SHADER, fsSource);

    link_program(gl, &v_shader?, &f_shader?)
}
#[allow(non_snake_case)]
fn initBuffers(
    gl: &WebGlRenderingContext,
    vertices: Vec<f32>,
    num_vertices: u32,
) -> Result<Buffers, JsValue> {
    let positionBuffer = gl
        .create_buffer()
        .ok_or("failed to create positionBuffer buffer")?;

    gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&positionBuffer));

    let position_array = float_32_array!(vertices);
    gl.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ARRAY_BUFFER,
        &position_array,
        WebGlRenderingContext::STATIC_DRAW,
    );

    let indexBuffer = gl
        .create_buffer()
        .ok_or("failed to create indexBuffer buffer")?;
    gl.bind_buffer(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        Some(&indexBuffer),
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
    Ok(Buffers(positionBuffer, indexBuffer))
}
#[allow(non_snake_case)]
#[allow(dead_code)]
fn drawScene(
    gl: &WebGlRenderingContext,
    programInfo: ProgramInfo,
    buffers: Buffers,
    zoom: f32,
    theta: f32,
    phi: f32,
    num_vertices: u32,
) -> Result<(), JsValue> {
    let Buffers(positionBuffer, indexBuffer) = buffers;
    let ProgramInfo(
        shaderProgram,
        vertexPosition,
        (location_projectionMatrix, location_modelViewMatrix),
    ) = programInfo;
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear_depth(1.0);
    gl.enable(WebGlRenderingContext::DEPTH_TEST);

    gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT | WebGlRenderingContext::DEPTH_BUFFER_BIT);

    let fieldOfView = 45.0 * PI / 180.0;
    let canvas: web_sys::HtmlCanvasElement = gl
        .canvas()
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
    let aspect: f32 = canvas.width() as f32 / canvas.height() as f32;
    let zNear = 1.0;
    let zFar = 100.0;
    let mut projectionMatrix = mat4::new_zero();

    mat4::perspective(&mut projectionMatrix, &fieldOfView, &aspect, &zNear, &zFar);

    let mut modelViewMatrix = mat4::new_identity();

    let mat_to_translate = modelViewMatrix;
    mat4::translate(&mut modelViewMatrix, &mat_to_translate, &[-0.0, 0.0, zoom]);

    let mat_to_rotate = modelViewMatrix;
    mat4::rotate_x(&mut modelViewMatrix, &mat_to_rotate, &phi);
    let mat_to_rotate = modelViewMatrix;
    mat4::rotate_y(&mut modelViewMatrix, &mat_to_rotate, &theta);

    {
        let numComponents = 3;
        let type_ = WebGlRenderingContext::FLOAT;
        let normalize = false;
        let stride = 0;
        let offset = 0;
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&positionBuffer));

        gl.vertex_attrib_pointer_with_i32(
            vertexPosition,
            numComponents,
            type_,
            normalize,
            stride,
            offset,
        );
        gl.enable_vertex_attrib_array(vertexPosition);
        // gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, None);
    }

    gl.bind_buffer(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        Some(&indexBuffer),
    );

    gl.use_program(Some(&shaderProgram));

    gl.uniform_matrix4fv_with_f32_array(
        Some(&location_projectionMatrix?),
        false,
        &projectionMatrix,
    );
    gl.uniform_matrix4fv_with_f32_array(Some(&location_modelViewMatrix?), false, &modelViewMatrix);
    {
        let vertexCount = num_vertices as i32;
        let type_ = WebGlRenderingContext::UNSIGNED_SHORT;
        let offset = 0;
        gl.draw_elements_with_i32(WebGlRenderingContext::TRIANGLES, vertexCount, type_, offset);
    }

    Ok(())
}

#[macro_export]
macro_rules! float_32_array {
    ($arr:expr) => {{
        let memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let arr_location = $arr.as_ptr() as u32 / 4;
        let array = js_sys::Float32Array::new(&memory_buffer)
            .subarray(arr_location, arr_location + $arr.len() as u32);
        array
    }};
}
#[macro_export]
macro_rules! uint_16_array {
    ($arr:expr) => {{
        let memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()?
            .buffer();
        let arr_location = $arr.as_ptr() as u32 / 2;
        let array = js_sys::Uint16Array::new(&memory_buffer)
            .subarray(arr_location, arr_location + $arr.len() as u32);
        array
    }};
}

pub fn compile_shader(
    context: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &WebGlRenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

pub fn request_animation_frame(f: &Closure<dyn FnMut(f32)>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
