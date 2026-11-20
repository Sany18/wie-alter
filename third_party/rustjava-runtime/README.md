# rustjava-runtime 0.4.1 (patched)

The crate [rustjava-runtime](https://crates.io/crates/rustjava-runtime) 0.4.1 from [RustJava](https://github.com/dlunch/RustJava)
by Inseok Lee (MIT), used through `[patch.crates-io]` in the workspace `Cargo.toml`, with one change in
`src/classes/java/io/input_stream_reader.rs` and `src/classes/java/io/output_stream_writer.rs`: `InputStreamReader` and
`OutputStreamWriter` take ISO-8859-1 (also named ISO8859_1, LATIN1) and US-ASCII besides UTF-8 and EUC-KR, one char
per byte. `String` already did. CLDC's default encoding is ISO-8859-1, and a platform with that default (the
`file.encoding` property) needs readers and writers that take it. Drop this copy once the change is in a RustJava release.
