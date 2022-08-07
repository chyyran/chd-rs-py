use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::{create_exception, wrap_pyfunction};
use std::fs::File;
use std::io::BufReader;

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

#[pymethods]
impl Metadata {
    pub fn tag(&self) -> PyResult<usize> {
        Ok(self.inner.metatag as usize)
    }
    pub fn data(&self) -> PyResult<Vec<u8>> {
        Ok(self.inner.value.clone())
    }
}

#[pymethods]
impl Chd {
    #[pyo3(name = "__len__")]
    pub fn len(&self) -> PyResult<usize> {
        Ok(self.inner.header().hunk_count() as usize)
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
        let mut hunk = self
            .inner
            .hunk(hunk_num as u32)
            .map_err(ChdPyError)?;
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

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn chd(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(chd_open, m)?)?;
    Ok(())
}
