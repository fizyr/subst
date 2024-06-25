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

If you want to quickly perform substitution on a string, use [`substitute()`] or [`substitute_bytes()`].

It is also possible to use one of the template types.
The templates parse the source string or bytes once, and can be expanded as many times as you want.
There are four different template types to choose from:
* [`Template`]: borrows the source string.
* [`TemplateBuf`]: owns the source string.
* [`ByteTemplate`]: borrows the source bytes.
* [`ByteTemplateBuf`]: owns the source bytes.

## Examples

The [`substitute()`][substitute] function can be used to perform substitution on a `&str`.
The variables can either be a [`HashMap`][std::collections::HashMap] or a [`BTreeMap`][std::collections::BTreeMap].

```rust
let mut variables = HashMap::new();
variables.insert("name", "world");
assert_eq!(subst::substitute("Hello $name!", &variables)?, "Hello world!");
```

The variables can also be taken directly from the environment with the [`Env`] map.

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

You can also parse a template once and expand it multiple times:

```rust
let mut variables = HashMap::new();
let template = subst::Template::from_str("Welcome to our hair salon, $name!")?;
for name in ["Scrappy", "Coco"] {
  variables.insert("name", name);
  let message = template.expand(&variables)?;
  println!("{}", message);
}
```

[`substitute()`]: https://docs.rs/subst/latest/subst/fn.substitute.html
[`substitute_bytes()`]: https://docs.rs/subst/latest/subst/fn.substitute_bytes.html
[`Template`]: https://docs.rs/subst/latest/subst/struct.Template.html
[`TemplateBuf`]: https://docs.rs/subst/latest/subst/struct.TemplateBuf.html
[`ByteTemplate`]: https://docs.rs/subst/latest/subst/struct.ByteTemplate.html
[`ByteTemplateBuf`]: https://docs.rs/subst/latest/subst/struct.ByteTemplateBuf.html
[`Env`]: https://docs.rs/subst/latest/subst/struct.Env.html
[std::collections::HashMap]: https://doc.rust-lang.org/stable/std/collections/struct.HashMap.html
[std::collections::BTreeMap]: https://doc.rust-lang.org/stable/std/collections/struct.BTreeMap.html
