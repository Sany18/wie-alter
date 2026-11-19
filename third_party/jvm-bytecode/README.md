# jvm-bytecode 0.4.1 (patched)

The crate [jvm-bytecode](https://crates.io/crates/jvm-bytecode) 0.4.1 from [RustJava](https://github.com/dlunch/RustJava)
by Inseok Lee (MIT), used through `[patch.crates-io]` in the workspace `Cargo.toml`, with one change in
`src/interpreter.rs`: the interpreter yields to the runtime every 4096 backward jumps of one method. Without it a Java loop
that calls no blocking method (a thread spinning on a field another thread sets, a long computation) never gives the
cooperative executor back: the other threads never run (the app hangs) and one host tick can take hundreds of ms.
Drop this copy once the change is in a RustJava release.
