use chd::header::Version;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::{create_exception, wrap_pyfunction};
use std::fs::File;
use std::io::BufReader;

struct ChdPyError(chd::Error);
create_exception!(chd_pyapi, ChdError, PyException);

impl From<chd::Error> for ChdPyError {
    fn from(err: chd::Error) -> Self {
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
    inner: Option<chd::Chd<BufReader<File>>>,
    cmp_vec: Vec<u8>,
}

#[pyclass]
struct Metadata {
    inner: chd::metadata::Metadata,
}

#[pyclass]
struct Header {
    inner: chd::header::Header,
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

    pub fn sha1(&self) -> Option<[u8; 20]> {
        self.inner.sha1()
    }

    pub fn parent_sha1(&self) -> Option<[u8; 20]> {
        self.inner.parent_sha1()
    }

    pub fn raw_sha1(&self) -> Option<[u8; 20]> {
        self.inner.raw_sha1()
    }

    pub fn version(&self) -> u32 {
        match self.inner.version() {
            Version::ChdV1 => 1,
            Version::ChdV2 => 2,
            Version::ChdV3 => 3,
            Version::ChdV4 => 4,
            Version::ChdV5 => 5,
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

impl Chd {
    fn inner_mut(&mut self) -> PyResult<&mut chd::Chd<BufReader<File>>> {
        match self.inner.as_mut() {
            Some(h) => Ok(h),
            None => Err(ChdError::new_err("underlying object has been deleted")),
        }
    }

    fn inner(&self) -> PyResult<&chd::Chd<BufReader<File>>> {
        match self.inner.as_ref() {
            Some(h) => Ok(h),
            None => Err(ChdError::new_err("underlying object has been deleted")),
        }
    }
}

#[pymethods]
impl Chd {
    #[pyo3(name = "__len__")]
    pub fn len(&mut self) -> PyResult<usize> {
        Ok(self.inner()?.header().hunk_count() as usize)
    }

    pub fn metadata(&mut self) -> PyResult<Vec<Metadata>> {
        let vecs: Vec<chd::metadata::Metadata> = self
            .inner_mut()?
            .metadata_refs()
            .try_into()
            .map_err(ChdPyError)?;
        Ok(vecs.into_iter().map(|m| Metadata { inner: m }).collect())
    }

    pub fn hunk(&mut self, hunk_num: usize) -> PyResult<Vec<u8>> {
        let mut cmp_vec = std::mem::take(&mut self.cmp_vec);
        let mut out = self.inner()?.get_hunksized_buffer();
        let mut hunk = self
            .inner_mut()?
            .hunk(hunk_num as u32)
            .map_err(ChdPyError)?;
        hunk.read_hunk_in(&mut cmp_vec, &mut out)
            .map_err(ChdPyError)?;
        self.cmp_vec = cmp_vec;
        Ok(out)
    }

    pub fn header(&mut self) -> PyResult<Header> {
        Ok(Header {
            inner: self.inner()?.header().clone(),
        })
    }
}

#[pyfunction]
fn chd_open(path: String, parent: Option<&mut Chd>) -> PyResult<Chd> {
    let file = File::open(path)?;
    let parent = parent
        .map(|p| p.inner.take())
        .map_or(None, |v| v)
        .map(Box::new);

    let chd = chd::Chd::open(BufReader::new(file), parent).map_err(ChdPyError)?;
    Ok(Chd {
        inner: Some(chd),
        cmp_vec: Vec::new(),
    })
}

#[pyfunction]
fn chd_read_header(path: String) -> PyResult<Header> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let h = chd::header::Header::try_read_header(&mut r).map_err(ChdPyError)?;
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
