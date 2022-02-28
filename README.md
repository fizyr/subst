# subst

Shell-like variable substitution for strings and byte strings.

## Features

* Perform substitution in `&str` or in `&[u8]`.
* Provide a custom map of variables or use environment variables.
* Short format: `"Hello $name!"`
* Long format: `"Hello ${name}!"`
* Default values: `"Hello ${name:person}!"`
* Recursive substitution in default values: `"${XDG_CONFIG_HOME:$HOME/.config}/my-app/config.toml"`
* Perform substitution on all string values in YAML data (optional, requires the `yaml` feature).

Variable names can consist of alphanumeric characters and underscores.
They are allowed to start with numbers.

## Examples

The [`substitute()`][substitute] function can be used to perform substitution on a `&str`.
The variables can either be a [`HashMap`][std::collections::HashMap] or a [`BTreeMap`][std::collections::BTreeMap].

```rust
let mut variables = HashMap::new();
variables.insert("name", "world");
assert_eq!(subst::substitute("Hello $name!", &variables)?, "Hello world!");
```

The variables can also be taken directly from the environment with the [`Env`][Env] map.

```rust
assert_eq!(
  subst::substitute("$XDG_CONFIG_HOME/my-app/config.toml", &subst::Env)?,
  "/home/user/.config/my-app/config.toml",
);
```

Substitution can also be done on byte strings using the [`substitute_bytes()`][substitute_bytes] function.

```rust
let mut variables = HashMap::new();
variables.insert("name", b"world");
assert_eq!(subst::substitute_bytes(b"Hello $name!", &variables)?, b"Hello world!");
```

[substitute]: https://docs.rs/subst/latest/subst/fn.substitute.html
[substitute_bytes]: https://docs.rs/subst/latest/subst/fn.substitute_bytes.html
[Env]: https://docs.rs/subst/latest/subst/struct.Env.html
[std::collections::HashMap]: https://doc.rust-lang.org/stable/std/collections/struct.HashMap.html
[std::collections::BTreeMap]: https://doc.rust-lang.org/stable/std/collections/struct.BTreeMap.html
