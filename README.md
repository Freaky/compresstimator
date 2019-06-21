# Compresstimator

A simple vaguely statistical file compressibility tester using lz4 level 1.

```rust
let estimator = Compresstimator::new();

if estimator.compresstimate_file("my_huge_file")? > 0.95 {
	println!("Probably doesn't compress well.");
}
```
