use clap::{App, ArgMatches, ArgSettings};
use config::{ConfigError, Source, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Clap {
    args: HashMap<String, CliType>,
    pub matches: ArgMatches<'static>,
}

#[derive(Debug, Clone)]
enum CliType {
    Multiple,
    Single,
    Count,
    Boolean,
    Subcommand(HashMap<String, CliType>),
}

impl Clap {
    pub fn new(app: App<'static, 'static>) -> Self {
        fn get_args_types(app: &App) -> HashMap<String, CliType> {
            app.p
                .subcommands
                .iter()
                .map(|app| {
                    (
                        app.p.meta.name.clone(),
                        CliType::Subcommand(get_args_types(&app)),
                    )
                })
                .chain(app.p.opts.iter().map(|opt| {
                    (
                        opt.b.name.to_owned(),
                        match (
                            opt.b.settings.is_set(ArgSettings::TakesValue),
                            opt.b.settings.is_set(ArgSettings::Multiple),
                        ) {
                            (true, true) => CliType::Multiple,
                            (true, false) => CliType::Single,
                            (false, true) => CliType::Count,
                            (false, false) => CliType::Boolean,
                        },
                    )
                }))
                .collect()
        }

        Clap {
            args: get_args_types(&app),
            matches: app.get_matches(),
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

        Ok(extract_matches(&self.matches, &self.args))
    }
}
