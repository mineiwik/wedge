use std::ops::{Add, AddAssign, MulAssign, Sub, SubAssign};
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
