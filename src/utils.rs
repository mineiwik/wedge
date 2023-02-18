use std::ops::{Add, AddAssign, MulAssign, Sub, SubAssign};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{HtmlCanvasElement, WebGlProgram, WebGlRenderingContext, WebGlShader};

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

pub fn resize_canvas(canvas: HtmlCanvasElement) {
    let new_width = window().inner_width().unwrap().as_f64().unwrap();
    let new_height = window().inner_height().unwrap().as_f64().unwrap();
    canvas.set_width(new_width as u32);
    canvas.set_height(new_height as u32);
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

pub type Vec3<T> = Vector<T, 3>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vector<T, const SIZE: usize>([T; SIZE]);

impl<T, const SIZE: usize> Vector<T, SIZE>
where
    T: Copy + PartialOrd,
{
    pub fn new(value: T) -> Self {
        Self([value; SIZE])
    }
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.0.get(idx)
    }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.0.get_mut(idx)
    }
    pub fn get_max(&self) -> T {
        let mut tmp = self.0;
        tmp.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        *tmp.first().unwrap()
    }
}

impl<T, const SIZE: usize> Add for Vector<T, SIZE>
where
    T: Add + AddAssign,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut res = self.0;
        res.iter_mut().zip(rhs.0).for_each(|(lhs, rhs)| *lhs += rhs);
        Self(res)
    }
}

impl<T, const SIZE: usize> Sub for Vector<T, SIZE>
where
    T: Sub + SubAssign,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut res = self.0;
        res.iter_mut().zip(rhs.0).for_each(|(lhs, rhs)| *lhs -= rhs);
        Self(res)
    }
}

impl<T, U, const SIZE: usize> VecOps<U> for Vector<T, SIZE>
where
    U: Copy,
    T: AddAssign + Copy + MulAssign<U>,
{
    fn translate(self, op: &Self) -> Self {
        let mut res = self.0;
        for (idx, rhs) in op.0.into_iter().enumerate() {
            res[idx] += rhs;
        }
        Self(res)
    }

    fn scale(self, scalar: U) -> Self {
        let mut res = self.0;
        for val in res.iter_mut() {
            *val *= scalar;
        }
        Self(res)
    }
}

pub trait VecOps<U> {
    fn translate(self, op: &Self) -> Self;
    fn scale(self, scalar: U) -> Self;
}
