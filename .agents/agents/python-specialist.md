---
description: Python and PyO3 binding development
model: haiku
name: python-specialist
# Content-Hash: blake3:b07a4015f3dfb0cbb167a7f1b33a8abce587378fa041825e52684502a5b1d896
# Source-Hash: blake3:5f0c54ee67302cc446ce75e468ff821190f5011b27088438b129604a6709c718
---

1. PyO3: #[pyclass], #[pymethods], #[new], return PyResult<T>
1. GIL: release with py.allow_threads() for CPU work, never hold during I/O
1. Use Bound\<'py, T> API (PyO3 0.22+), implement __repr__ and __str__
1. Async: pyo3_asyncio::tokio::future_into_py, blocking wrapper for sync callers
1. Build: maturin develop (local), maturin build --release (dist)
1. Test: pytest, package: uv + pyproject.toml, distribute on PyPI
