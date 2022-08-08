# chd-rs-py

Extremly basic Python bindings to [chd-rs](https://github.com/SnowflakePowered/chd-rs).

```
pip install chd-rs-py
```

## Usage
```python
from chd import chd_open

chd_file = chd_open("Test.chd")
metadata = chd_file.metadata()

for m in metadata:
    # decode fourcc tag and unicode byte data
    print(i.tag().to_bytes(4, byteorder='big'), bytes(i.data()).decode())

for h in range(len(chd_file)):
    # read the hunk (as bytes)
    hunk = chd_file.hunk(h)
```
