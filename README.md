# clown

<a href="https://crates.io/crates/clown"><img alt="Crate Info" src="https://img.shields.io/crates/v/clown.svg"/></a>
<a href="https://docs.rs/clown/"><img alt="API Docs" src="https://img.shields.io/badge/docs.rs-clown-yellow"/></a>

An approximation of "capture-by-clone" lambdas in Rust.    
Requires nightly and `#![feature(proc_macro_hygiene, stmt_expr_attributes)]`

Turns this:
```rust
#[clown] || do_call(honk!(foo.bar), slip!(baz.bop))
```
into this:
```rust
{
    let __honk_0 = (foo.bar).clone();
	let __slip_0 = baz.bop;
    move || do_call(__honk_0, __slip_0)
}
```
