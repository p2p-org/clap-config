use clap::{App, ArgMatches, ArgSettings};
use config::{ConfigError, Source, Value};
use std::collections::HashMap;
use std::ffi::OsString;

#[derive(Debug, Clone)]
pub struct Clap {
    args: HashMap<String, CliType>,
    pub matches: ArgMatches<'static>,
    subcommand_field: Option<String>,
}

#[derive(Debug, Clone)]
enum CliType {
    Multiple,
    Single,
    Count,
    Boolean,
    Subcommand(HashMap<String, CliType>),
}

impl From<App<'static, 'static>> for Clap {
    fn from(app: App<'static, 'static>) -> Clap {
        Clap::new(app)
    }
}

impl Clap {
    pub fn new(app: App<'static, 'static>) -> Self {
        Self::from_matches(Self::get_args_types(&app), app.get_matches())
    }

    pub fn from_args<I>(app: App<'static, 'static>, args: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        Self::from_matches(Self::get_args_types(&app), app.get_matches_from(args))
    }

    pub fn subcommand_field(mut self, field: &str) -> Self {
        self.subcommand_field = Some(field.to_owned());
        self
    }

    fn get_args_types(app: &App) -> HashMap<String, CliType> {
        fn convert(name: &str, takes_value: bool, multiple: bool) -> (String, CliType) {
            (
                name.to_owned(),
                match (takes_value, multiple) {
                    (true, true) => CliType::Multiple,
                    (true, false) => CliType::Single,
                    (false, true) => CliType::Count,
                    (false, false) => CliType::Boolean,
                },
            )
        }

        app.p
            .subcommands
            .iter()
            .map(|app| {
                (
                    app.p.meta.name.clone(),
                    CliType::Subcommand(Self::get_args_types(&app)),
                )
            })
            .chain(app.p.opts.iter().map(|opt| {
                convert(
                    opt.b.name,
                    opt.b.settings.is_set(ArgSettings::TakesValue),
                    opt.b.settings.is_set(ArgSettings::Multiple),
                )
            }))
            .chain(app.p.flags.iter().map(|flag| {
                convert(
                    flag.b.name,
                    flag.b.settings.is_set(ArgSettings::TakesValue),
                    flag.b.settings.is_set(ArgSettings::Multiple),
                )
            }))
            .chain(app.p.positionals.iter().map(|(_, pos)| {
                convert(
                    pos.b.name,
                    pos.b.settings.is_set(ArgSettings::TakesValue),
                    pos.b.settings.is_set(ArgSettings::Multiple),
                )
            }))
            .collect()
    }

    fn from_matches(args: HashMap<String, CliType>, matches: ArgMatches<'static>) -> Self {
        Self {
            args,
            matches,
            subcommand_field: None,
        }
    }
}

impl Source for Clap {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<HashMap<String, Value>, ConfigError> {
        fn extract_matches(
            matches: &ArgMatches,
            args: &HashMap<String, CliType>,
        ) -> HashMap<String, Value> {
            args.into_iter()
                .filter_map(|(name, tpe)| {
                    let conf_name = name.clone();
                    match tpe {
                        CliType::Multiple => matches.values_of(name).map(|values| {
                            (conf_name, Value::new(None, values.collect::<Vec<_>>()))
                        }),
                        CliType::Single => matches
                            .value_of(name)
                            .map(|value| (conf_name, Value::new(None, value))),
                        CliType::Count => Some((
                            conf_name,
                            Value::new(None, matches.occurrences_of(name) as i64),
                        )),
                        CliType::Boolean => {
                            Some((conf_name, Value::new(None, matches.is_present(name))))
                        }
                        CliType::Subcommand(subargs) => {
                            matches.subcommand_matches(name).map(|submatches| {
                                (
                                    conf_name,
                                    Value::new(None, extract_matches(submatches, subargs)),
                                )
                            })
                        }
                    }
                })
                .collect()
        }

        let mut matches = extract_matches(&self.matches, &self.args);

        if let (Some(subcommand_field), Some(subcommand)) =
            (&self.subcommand_field, self.matches.subcommand_name())
        {
            matches.insert(subcommand_field.clone(), Value::new(None, subcommand));
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{App, Arg};
    use serde_derive::Deserialize;

    #[derive(Debug, Deserialize, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct Config {
        pub format: Option<String>,
        pub verbosity: usize,
        pub subcommand: Option<SubConfig>,
        pub mode: Option<String>,
    }

    #[derive(Debug, Deserialize, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct SubConfig {
        pub ids: Vec<u32>,
        pub flag: bool,
    }

    fn new_app() -> App<'static, 'static> {
        App::new("app")
            .arg(
                Arg::with_name("format")
                    .takes_value(true)
                    .short("f")
                    .long("format"),
            )
            .arg(
                Arg::with_name("verbosity")
                    .short("v")
                    .long("verbose")
                    .multiple(true),
            )
            .subcommand(
                App::new("subcommand")
                    .arg(Arg::with_name("flag").short("F").long("flag"))
                    .arg(
                        Arg::with_name("ids")
                            .short("i")
                            .long("id")
                            .required(true)
                            .takes_value(true)
                            .multiple(true),
                    ),
            )
    }

    fn new_clap_config<I>(args: I) -> Clap
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        Clap::from_args(new_app(), args).subcommand_field("mode")
    }

    fn test_clap_with_args(args: Vec<&str>, expected: Config) {
        let mut conf = config::Config::new();
        let clap = new_clap_config(args);

        log::debug!("CLAP: {:?}", clap);

        conf.merge(clap).unwrap();
        assert_eq!(conf.try_into::<Config>().unwrap(), expected);
    }

    #[test]
    fn test_clap() {
        env_logger::init();

        test_clap_with_args(
            vec![
                "myprog",
                "-vvv",
                "--format=json",
                "subcommand",
                "-i1",
                "-i2",
                "-i3",
            ],
            Config {
                format: Some("json".into()),
                verbosity: 3,
                subcommand: Some(SubConfig {
                    ids: vec![1, 2, 3],
                    flag: false,
                }),
                mode: Some("subcommand".into()),
            },
        );
    }
}
