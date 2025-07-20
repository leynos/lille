# Behavioural testing in Rust with rust-rspec

A short overview of the `rspec` crate for behaviour-driven tests is presented
here. The framework offers a simple `given`/`when`/`then` syntax that clarifies
test intent.

## Adding the dependency

Add `rspec` as a development dependency in `Cargo.toml`:

```toml
[dev-dependencies]
rspec = "1.0"
```

## Example structure

A minimal suite uses an environment struct to share mutable state between
steps. The `before_all` and `before_each` hooks prepare that state before
expectations are run.

```rust
#[derive(Clone, Default, Debug)]
struct Env {
    counter: i32,
}

#[test]
fn example() {
    rspec::run(&rspec::given("a counter", Env::default(), |ctx| {
        ctx.before_each(|env| env.counter = 0);
        ctx.when("incremented", |ctx| {
            ctx.before_each(|env| env.counter += 1);
            ctx.then("it increases", |env| assert_eq!(env.counter, 1));
        });
    }));
}
```

Behavioural tests under `tests/` demonstrate real-world usage.
