## conn_mon - Connection Monitor Tool

 A program to monitor the quality of a connection.
 
 NB: Only supports Linux right now.
 
 ## Usage Examples
 
 Run with `config.json` in current working directory
 
 ```
 cargo run -- -c sample_config_full.json
 ```
 
 Run with full sample config
 
A cargo alias has been added to simplify running using the minimal sample config file

```sh
cargo r
```

## License

All code in this repository is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are very good reasons to include both as noted in 
this [issue](https://github.com/bevyengine/bevy/issues/2373) on [Bevy](https://bevyengine.org)'s repo.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
