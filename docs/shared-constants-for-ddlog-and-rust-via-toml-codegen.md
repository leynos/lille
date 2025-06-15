# Shared Constants for DDlog and Rust via TOML Codegen

---

## 1 Pick a neutral, data-only format as the *authority*

I usually reach for **TOML** because

- it is already in the Rust ecosystem via `Cargo.toml`, so the syntax feels familiar;

- `toml`/`toml_edit` give you trivial parsing/serialisation in Rust;

- it round-trips cleanly in Python when you need a quick standalone script (see §4).

Example `constants.toml`

```toml
[network]
max_clients = 2048           # u32
timeout_ms  = 1500           # u16

[paths]
config_dir = "/etc/myapp"    # string
```

---

## 2 Generate a **Rust** module inside `build.rs`

`build.rs` already runs before every build, so it is the obvious place to parse the TOML and spit out `const` items into `$OUT_DIR/constants.rs`, then `include!` them:

```rust
// build.rs
use std::{env, fs, path::PathBuf};
fn main() {
    let toml = fs::read_to_string("constants.toml").unwrap();
    let parsed: toml::Value = toml.parse().unwrap();
    let out    = PathBuf::from(env::var("OUT_DIR").unwrap()).join("constants.rs");
    let mut rs = String::from("// automatically generated – DO NOT EDIT
");

    for (section, table) in parsed.as_table().unwrap() {
        for (k, v) in table.as_table().unwrap() {
            match v {
                toml::Value::Integer(i) => rs += &format!("pub const {}_{}: u32 = {};
",
                                                           section.to_uppercase(),
                                                           k.to_uppercase(), i),
                toml::Value::String(s)  => rs += &format!("pub const {}_{}: &str = {:?};
",
                                                           section.to_uppercase(),
                                                           k.to_uppercase(), s),
                _ => panic!("unsupported type"),
            }
        }
    }

    fs::write(&out, rs).unwrap();
    println!("cargo:rerun-if-changed=constants.toml");      // rebuild trigger
}
```

and in your library/test code:

```rust
include!(concat!(env!("OUT_DIR"), "/constants.rs"));
```

Using a build script is endorsed by the Cargo Book for exactly this sort of ahead-of-time codegen ([doc.rust-lang.org](https://doc.rust-lang.org/cargo/reference/build-scripts.html)).

If you want nicer ergonomics, crates such as `build_const` wrap the “write-file-of-consts” pattern for you ([docs.rs](https://docs.rs/build_const)).

---

## 3 Generate a **DDlog** file in the same pass

DDlog itself has no facility to read TOML at compile-time, so you write out a normal `.dl` module next to your other logic. From the same `build.rs` you can emit, say, `src/logic/constants.dl` (or `$OUT_DIR` and add that directory to DDlog’s `-L` search path):

```rust
let mut dl = String::from("// @generated – DO NOT EDIT
");
for (section, table) in parsed.as_table().unwrap() {
    for (k, v) in table.as_table().unwrap() {
        match v {
            toml::Value::Integer(i) => dl += &format!(
                "function {}_{}(): bit<32> {{ {} }}
",
                section, k, i),
            toml::Value::String(s)  => dl += &format!(
                "function {}_{}(): string {{ \"{}\" }}
",
                section, k, s),
            _ => panic!("unsupported type"),
        }
    }
}
fs::write("logic/constants.dl", dl).unwrap();
```

Because the DDlog compiler itself is just another executable, you can (optionally) invoke it from `build.rs` once the file is in place:

```rust
std::process::Command::new("ddlog")
    .args(["-i", "logic/main.dl", "-L", "logic"])
    .status()
    .expect("failed to run ddlog");
```

That keeps the whole pipeline behind a single `cargo build && cargo test`.\
(Compiling DDlog from inside `build.rs` is perfectly legal; it is just another code-generation step, exactly like building C stubs the Cargo docs mention.)

The generated Rust crate from DDlog ends up in `logic_ddlog/`, and you can link it as a normal workspace member or via `path =` in `Cargo.toml`, as the introductory tutorial shows ([hexgolems.com](https://hexgolems.com/2020/10/getting-started-with-ddlog/)).

---

## 4 Why not an external Python + Makefile?

You *can* substitute the `build.rs` with

1. `python gen_constants.py`

2. `make ddlog`

but you lose two advantages:

- **Change tracking** – Cargo will *only* re-run scripts that printed the appropriate `cargo:rerun-if-changed=` line; hand-rolled Make rules tend to go stale.

- **One-shot builds** – contributors can type nothing more exotic than `cargo check` and still get everything regenerated.

That said, if the constants are huge or the transformation is heavyweight, a dedicated script may be more pleasant to maintain. The Python sketch you had in mind is perfectly serviceable; the code above will translate line-for-line using `toml` in place of `serde`.

---

## 5 Putting it together

- `constants.toml` is the single editable artefact.

- `build.rs` (or a crate like `build_const`) regenerates

  - `$OUT_DIR/constants.rs` – surfaced to your Rust code via `include!`, and

  - `logic/constants.dl` – imported from your DDlog program with `import logic.constants;`.

- Optionally `build.rs` also calls `ddlog` so the generated Rust crate stays in lock-step.

This pattern keeps DDlog, Rust *and* your test-harness in perfect agreement, without any manual synchronisation, and it plays nicely with incremental `cargo` builds and CI caching. If you later need the constants in a third language the only change is to teach the generator a new emitter function.

---

### Further reading

- Cargo Book: *Build Scripts* – official guide to the `build.rs` lifecycle ([doc.rust-lang.org](https://doc.rust-lang.org/cargo/reference/build-scripts.html))

- Ben Lovy, *Automatically Generate Rust Modules With Cargo Build Scripts* – pragmatic walkthrough of emitting Rust from `build.rs` ([dev.to](https://dev.to/deciduously/automatically-generate-rust-modules-with-cargo-build-scripts-157h))

- Crate `build_const` – minimal helper for writing `const` files during the build ([docs.rs](https://docs.rs/build_const))

- Hexgolems tutorial, *Getting Started with DDlog* – shows the standard “`ddlog -i … && cargo build`” flow used above ([hexgolems.com](https://hexgolems.com/2020/10/getting-started-with-ddlog/))

That should give you a reproducible, zero-drift constants pipeline without adding any friction to your day-to-day `cargo test`.