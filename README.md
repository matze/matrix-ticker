# matrix-ticker

A simple [Matrix](https://matrix.org) client running on a Raspberry Pi Zero that
displays messages from all known rooms.


## Configuration

Write a `config.toml` file containing

```toml
user_id = "@foobar:matrix.org"
password = "myfancypassword"
```

and run the binary. The program understands the typical env logger options, i.e.
control output via `RUST_LOG`.
