# knyst_bela

Simple interface for running Knyst on the bela

## Usage

cbindgen doesn't traverse dependencies so copy `knyst_bela.h` to your main FFI crate and include it in your cbindgen includes. Also make sure to use use this crate is a dependency of your main library crate so that the functions are include in the library and can be linked.
