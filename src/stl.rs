use std::slice::Iter;

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
    let mut max_value: f32 = f32::NEG_INFINITY;
    let mut vertices: Vec<f32> = vec![];

    for _ in 0..num_facets {
        payload.by_ref().take(STL_AXES * STL_F32_BYTES).count();
        for _ in 0..STL_AXES * STL_VERTICES_PER_FACET {
            get_vertex(payload.by_ref(), &mut max_value, &mut vertices);
        }
        payload.by_ref().take(STL_EXTRA_BYTES).count();
    }

    if payload.next().is_some() {
        return Err(InvalidFileContentError::new("STL: Payload is too large"));
    }

    let vertices = vertices.into_iter().map(|x| x / max_value.abs()).collect();

    Ok(vertices)
}

fn get_vertex(payload: &mut Iter<u8>, max_value: &mut f32, vertices: &mut Vec<f32>) {
    let v: Vec<u8> = payload.take(STL_F32_BYTES).cloned().collect();
    let v: [u8; STL_F32_BYTES] = v.try_into().unwrap();
    let v = f32::from_le_bytes(v);

    if v.abs() > *max_value {
        *max_value = v.abs();
    }
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
