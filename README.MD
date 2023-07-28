# [WIP] remote-archive

remote-archive is a utility for exploring remote archive files without downloading the entire contents of the archive.

The idea behind it is to use the `Range` HTTP header to read the entries' metadata and skip the content of the entries in the archive.

- [ ] Option for downloading a specific file
- [ ] File types
    - [x] ZIP
    - [ ] TAR (USTAR)
    - [ ] RAR
    - [ ] 7z

```
Usage: {remote_archive} [--url <url>] [-f <file>] [-p <proxy>] [-t]

Options:
  --url             the URL to download(mutually exclusive with 'file')
  -f, --file        the file to process (mutually exclusive with 'url')
  -p, --proxy       the proxy server to use
  -t, --tree        to show file contents as tree
  --help            display usage information
```

## an example usage (exploring the zip file from the middle of it (450GB)): 

```
$ cargo run -- --url 'http://foo.bar/foo.zip' --proxy socks5://127.0.0.1:9050 -t


│   ├── folder 1
│   │   ├── folder 2
│   │   │   ├── file1.xlsx
│   │   │   ├── file2.xlsx
│   │   ├── folder 2
│   │   ├── folder 3
│   │   │   ├── file3.xlsx
│   │   │   ├── file4.xlsx

....
```

## an example usage (multi-file): 

```
$ cargo run -- -f urls.txt --proxy socks5://127.0.0.1:9050 -t


│   ├── folder 1
│   │   ├── folder 2
│   │   │   ├── file1.xlsx
│   │   │   ├── file2.xlsx
│   │   ├── folder 2
│   │   ├── folder 3
│   │   │   ├── file3.xlsx
│   │   │   ├── file4.xlsx

....
```
