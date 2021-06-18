# clap-config

This is a glue between the [`clap`][1] and [`config`][2] crates.
Load your command line arguments into your common config at ease!

This crate was created for [P2P Validator](https://p2p.org).

# Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
clap-config = "0.1.0"
clap = "2"
config = "0.11"
serde = "1"
serde_derive = "1"
```

Now you can use it in your code:

```rust
use config::{Environment, File};
use clap_config::Clap;
use clap::{App, Arg};

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub format: Option<String>,
    pub verbosity: usize,
    pub subcommand: Option<SubConfig>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SubConfig {
    pub ids: Vec<u32>,
    pub flag: bool,
}

fn main() {
    let app = App::new("app")
        .arg(Arg::with_name("format")
            .takes_value(true)
            .short("f")
            .long("format"))
        .arg(Arg::with_name("verbosity")
            .short("v")
            .long("verbose")
            .multiple(true))
        .subcommand(App::new("subcommand")
            .arg(Arg::with_name("flag")
                .short("F")
                .long("flag"))
            .arg(Arg::with_name("ids")
                .short("i")
                .long("id")
                .required(true)
                .takes_value(true)
                .multiple(true)));
    
    let mut conf = Config::new();
    conf.merge(File::from("config.toml")).unwrap()
        .merge(Environment::with_prefix("APP_CONFIG")).unwrap()
        .merge(Clap::new(app)).unwrap();
    let options: Config = conf.try_into().unwrap();
    println!("Running with config: {:?}", options);
}
```


[1]: https://github.com/clap-rs/clap
[2]: https://github.com/mehcode/config-rs
