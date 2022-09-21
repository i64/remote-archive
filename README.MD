# [WIP] REMOTE-ARCHIVE 

Remote-Archive is a utility for exploring remote archive files without downloading the entire contents of the archive.

The idea behind it is to use the `Range` HTTP header to read the entries' metadata and skip the content of the entries in the archive.

- [x] Option for using custom offsets
- [ ] Specificing the thread count
- [ ] Option for downloading a specific file
- [ ] File types
    - [x] ZIP
    - [ ] TAR (USTAR)
    - [ ] RAR
    - [ ] 7z

### an example usage (exploring the zip file from the middle of it (450GB)): 

```
$ cargo run -- --url 'http://foo.bar/a.zip'  --offset=255194890075 --proxy socks5://127.0.0.1:9050

[src/remote_file.rs:54] &range = 255194890075..255194894171
[src/remote_file.rs:54] &range = 255194890078..255194890082
[src/remote_file.rs:54] &range = 255194890082..255194890100
[src/remote_file.rs:54] &range = 255194890100..255194890100
[src/main.rs:61] &entry = Ok(
    Entry {
        entry: EndOfCentralDir(
            EndOfCentralDir {
                disk_of_end_of_central_dir: 0,
                disk_of_central_dir: 0,
                num_central_dir_entries_on_disk: 1,
                num_central_dir_entries_total: 1,
                len_central_dir: 88,
                ofs_central_dir: 11507406,
                len_comment: 0,
                comment: "",
            },
        ),
        range: 255194890078..255194890100,
    },
)
[src/remote_file.rs:54] &range = 255194890100..255194890104
[src/remote_file.rs:54] &range = 255194890104..255194890130
[src/remote_file.rs:54] &range = 255194890130..255194890247
[src/main.rs:61] &entry = Ok(
    Entry {
        entry: LocalFile(
            LocalFile {
                header: LocalFileHeader {
                    version: 20,
                    flags: 2048,
                    compression_method: Deflated,
                    file_mod_time: 20964,
                    file_mod_date: 21694,
                    crc32: 2242091140,
                    len_body_compressed: 641317,
                    len_body_uncompressed: 664719,
                    len_filename: 117,
                    len_extra: 28,
                    filename: "Y.xlsx",
                },
                body: None,
            },
        ),
        range: 255194890100..255195531592,
    },
)
```