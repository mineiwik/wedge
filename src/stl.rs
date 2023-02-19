use std::slice::Iter;

use crate::utils::{Vec3, VecOps};

const STL_HEADER_BYTES: usize = 0x50;
const STL_NUMBER_FACETS_BYTES: usize = 0x4;
const STL_FACET_RECORD_BYTES: usize = 0x32;
const STL_F32_BYTES: usize = 4;
const STL_EXTRA_BYTES: usize = 2;
const STL_AXES: usize = 3;
const STL_VERTICES_PER_FACET: usize = 3;

#[derive(Debug, Clone)]
pub struct InvalidFileContentError {
    msg: String,
}

impl InvalidFileContentError {
    fn new(msg: &str) -> Self {
        Self { msg: msg.into() }
    }
}

impl std::fmt::Display for InvalidFileContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

pub fn get_data(bytes: &[u8]) -> Result<(Vec<f32>, u32), InvalidFileContentError> {
    let (payload, num_facets) = extract_data(bytes)?;
    let vertices = get_vertices(payload, num_facets)?;
    let num_vertices = STL_AXES as u32 * num_facets;

    Ok((vertices, num_vertices))
}

fn get_vertices(payload: Vec<u8>, num_facets: u32) -> Result<Vec<f32>, InvalidFileContentError> {
    let mut payload = payload.iter();
    let mut min_values = Vec3::new(f32::INFINITY);
    let mut max_values = Vec3::new(f32::NEG_INFINITY);
    let mut vertices: Vec<f32> = vec![];

    for _ in 0..num_facets {
        payload.by_ref().take(STL_AXES * STL_F32_BYTES).count();
        for _ in 0..STL_VERTICES_PER_FACET {
            for idx in 0..STL_AXES {
                get_vertex(
                    payload.by_ref(),
                    max_values.get_mut(idx).unwrap(),
                    min_values.get_mut(idx).unwrap(),
                    &mut vertices,
                );
            }
        }
        payload.by_ref().take(STL_EXTRA_BYTES).count();
    }

    if payload.next().is_some() {
        return Err(InvalidFileContentError::new("STL: Payload is too large"));
    }

    let half_lengths = (max_values - min_values).scale(0.5);
    let translations = half_lengths - max_values;
    let scale = half_lengths.get_max();

    vertices.iter_mut().enumerate().for_each(|(idx, v)| {
        *v += *translations.get(idx % 3).unwrap();
    });

    vertices.iter_mut().for_each(|v| {
        *v /= scale;
    });

    Ok(vertices)
}

fn get_vertex(
    payload: &mut Iter<u8>,
    max_value: &mut f32,
    min_value: &mut f32,
    vertices: &mut Vec<f32>,
) {
    let v: Vec<u8> = payload.take(STL_F32_BYTES).cloned().collect();
    let v: [u8; STL_F32_BYTES] = v.try_into().unwrap();
    let v = f32::from_le_bytes(v);

    *max_value = max_value.max(v);
    *min_value = min_value.min(v);
    vertices.push(v);
}

fn extract_data(bytes: &[u8]) -> Result<(Vec<u8>, u32), InvalidFileContentError> {
    let mut b_it = bytes.iter();
    let header: Vec<&u8> = b_it.by_ref().take(STL_HEADER_BYTES).collect();
    if header.len() != STL_HEADER_BYTES {
        return Err(InvalidFileContentError::new("STL: header too short"));
    }
    let num_facets: Vec<u8> = b_it
        .by_ref()
        .take(STL_NUMBER_FACETS_BYTES)
        .cloned()
        .collect();
    if num_facets.len() != STL_NUMBER_FACETS_BYTES {
        return Err(InvalidFileContentError::new(
            "STL: number of facets not u32",
        ));
    }
    let num_facets: Result<[u8; 4], _> = num_facets.try_into();
    if num_facets.is_err() {
        return Err(InvalidFileContentError::new(""));
    }
    let num_facets = u32::from_le_bytes(num_facets.unwrap());
    let payload: Vec<_> = b_it.by_ref().cloned().collect();
    if payload.len() % STL_FACET_RECORD_BYTES != 0 {
        return Err(InvalidFileContentError::new(
            "STL: payload is not aligned properly",
        ));
    }
    if payload.len() / STL_FACET_RECORD_BYTES != (num_facets as usize) {
        return Err(InvalidFileContentError::new(
            "STL: payload does not match specified length",
        ));
    }
    Ok((payload, num_facets))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cube_bytes_to_vertices() {
        let cube = std::fs::read("tests/files/cube.stl").unwrap();

        let (mut vertices, num_vertices) = get_data(&cube).unwrap();
        vertices.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        assert_eq!(num_vertices, 36);
        assert!(vertices.first().unwrap().le(&1.0));
    }

    #[test]
    fn test_box_bytes_to_vertices() {
        let cube = std::fs::read("tests/files/box.stl").unwrap();

        let (mut vertices, _) = get_data(&cube).unwrap();
        vertices.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

        assert!(vertices.first().unwrap().le(&1.0));
    }
}
