use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::{create_exception, wrap_pyfunction};
use std::fs::File;
use std::io::BufReader;

use chd::header::{ChdHeader, ChdHeader::*};
use chd::{metadata::ChdMetadata, ChdFile};

struct ChdPyError(chd::ChdError);
create_exception!(chd_pyapi, ChdError, PyException);

impl From<chd::ChdError> for ChdPyError {
    fn from(err: chd::ChdError) -> Self {
        ChdPyError(err)
    }
}

impl From<ChdPyError> for PyErr {
    fn from(err: ChdPyError) -> PyErr {
        ChdError::new_err(err.0.to_string())
    }
}

#[pyclass]
struct Chd {
    inner: ChdFile<BufReader<File>>,
    cmp_vec: Vec<u8>,
}

#[pyclass]
struct Metadata {
    inner: ChdMetadata,
}

#[pyclass]
struct Header {
    inner: ChdHeader,
}

#[pymethods]
impl Header {
    pub fn is_compressed(&self) -> bool {
        self.inner.is_compressed()
    }

    pub fn meta_offset(&self) -> Option<u64> {
        self.inner.meta_offset()
    }

    pub fn flags(&self) -> Option<u32> {
        self.inner.flags()
    }

    pub fn hunk_count(&self) -> u32 {
        self.inner.hunk_count()
    }

    pub fn hunk_size(&self) -> u32 {
        self.inner.hunk_size()
    }

    pub fn logical_bytes(&self) -> u64 {
        self.inner.logical_bytes()
    }

    pub fn unit_bytes(&self) -> u32 {
        self.inner.unit_bytes()
    }

    pub fn unit_count(&self) -> u64 {
        self.inner.unit_count()
    }

    pub fn has_parent(&self) -> bool {
        self.inner.has_parent()
    }

    #[pyo3(name = "__len__")]
    pub fn len(&self) -> usize {
        self.inner.len() as usize
    }

    pub fn sha1(&self) -> Option<&[u8]> {
        match &self.inner {
            V3Header(h) => Some(&h.sha1),
            V4Header(h) => Some(&h.sha1),
            V5Header(h) => Some(&h.sha1),
            _ => None,
        }
    }

    pub fn parent_sha1(&self) -> Option<&[u8]> {
        if self.inner.has_parent() {
            return match &self.inner {
                V3Header(h) => Some(&h.parent_sha1),
                V4Header(h) => Some(&h.parent_sha1),
                V5Header(h) => Some(&h.parent_sha1),
                _ => None,
            };
        }
        return None;
    }

    pub fn raw_sha1(&self) -> Option<&[u8]> {
        match &self.inner {
            V4Header(h) => Some(&h.raw_sha1),
            V5Header(h) => Some(&h.raw_sha1),
            _ => None,
        }
    }

    pub fn version(&self) -> u32 {
        match &self.inner {
            V1Header(_) => 1,
            V2Header(_) => 2,
            V3Header(_) => 3,
            V4Header(_) => 4,
            V5Header(_) => 5,
        }
    }
}

#[pymethods]
impl Metadata {
    pub fn tag(&self) -> usize {
        self.inner.metatag as usize
    }
    pub fn data(&self) -> Vec<u8> {
        self.inner.value.clone()
    }
}

#[pymethods]
impl Chd {
    #[pyo3(name = "__len__")]
    pub fn len(&self) -> usize {
        self.inner.header().hunk_count() as usize
    }

    pub fn metadata(&mut self) -> PyResult<Vec<Metadata>> {
        Ok(self
            .inner
            .metadata_refs()
            .try_into_vec()
            .map_err(ChdPyError)?
            .into_iter()
            .map(|m| Metadata { inner: m })
            .collect())
    }

    pub fn hunk(&mut self, hunk_num: usize) -> PyResult<Vec<u8>> {
        let mut cmp_vec = std::mem::take(&mut self.cmp_vec);
        let mut out = self.inner.get_hunksized_buffer();
        let mut hunk = self.inner.hunk(hunk_num as u32).map_err(ChdPyError)?;
        hunk.read_hunk_in(&mut cmp_vec, &mut out)
            .map_err(ChdPyError)?;
        self.cmp_vec = cmp_vec;
        Ok(out)
    }
}

#[pyfunction]
fn chd_open(path: String, parent: Option<String>) -> PyResult<Chd> {
    let file = File::open(path)?;
    let parent = parent
        .map(|p| File::open(p).map(BufReader::new))
        .map_or(Ok(None), |v| v.map(Some))?
        .map(|n| ChdFile::open(n, None))
        .map_or(Ok(None), |v| v.map(|v| Some(Box::new(v))))
        .map_err(ChdPyError)?;

    let chd = ChdFile::open(BufReader::new(file), parent).map_err(ChdPyError)?;
    Ok(Chd {
        inner: chd,
        cmp_vec: Vec::new(),
    })
}

#[pyfunction]
fn chd_read_header(path: String) -> PyResult<Header> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let h = ChdHeader::try_read_header(&mut r).map_err(ChdPyError)?;
    Ok(Header { inner: h })
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn chd(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(chd_open, m)?)?;
    m.add_function(wrap_pyfunction!(chd_read_header, m)?)?;
    Ok(())
}
