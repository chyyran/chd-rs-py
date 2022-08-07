from chd import chd_open

chd_file = chd_open("Test.chd")
metadata = chd_file.metadata()

for m in metadata:
    print(m.tag(), m.data())

for h in range(len(chd_file)):
    # read the hunk (as bytes)
    hunk = chd_file.hunk(h)