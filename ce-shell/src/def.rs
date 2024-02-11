#[macro_export]
macro_rules! define_shell {
    ($($krate:path[$name:ident, $display:literal]),*$(,)?) => {
        use std::{str::FromStr, sync::Arc};

        use ce_core::{Env, EnvError, Generate, ValidationResult};
        use itertools::Itertools;

        pub mod envs {
            $(pub use $krate;)*
        }

        #[derive(tapi::Tapi)]
        #[serde(tag = "analysis", content = "io")]
        pub enum Envs {
            $($name {
                input: <$krate as Env>::Input,
                output: <$krate as Env>::Output,
            },)*
        }

        #[derive(tapi::Tapi, Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub enum Analysis {
            $($name,)*
        }

        impl std::fmt::Display for Analysis {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Analysis::$name => write!(f, $display),)*
                }
            }
        }

        impl FromStr for Analysis {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($display => Ok(Analysis::$name),)*
                    $(stringify!($name) => Ok(Analysis::$name),)*
                    _ => Err(format!("analysis can be one of: {}", [$(stringify!($name),)*].into_iter().format(", "))),
                }
            }
        }

        impl Analysis {
            pub fn options() -> &'static [Analysis] {
                &[$(Analysis::$name),*]
            }
            pub fn code(&self) -> &'static str {
                match self {
                    $(Analysis::$name => stringify!($name)),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.to_string()))]
            pub fn gen_input(self, rng: &mut rand::rngs::SmallRng) -> Input {
                match self {
                    $(Analysis::$name => {
                        let input = <$krate as Env>::Input::gen(&mut (), rng);
                        Input {
                            analysis: self,
                            json: Arc::new(serde_json::to_value(input).expect("input is always valid json")),
                        }
                    }),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.to_string(), ?src))]
            pub fn input_from_str(self, src: &str) -> Result<Input, $crate::io::Error> {
                match self {
                    $(Analysis::$name => {
                        let input = serde_json::from_str::<<$krate as Env>::Input>(src)
                            .map_err($crate::io::Error::JsonError)?;
                        Ok(Input {
                            analysis: self,
                            json: Arc::new(serde_json::to_value(input).expect("input is always valid json")),
                        })
                    }),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.to_string(), ?src))]
            pub fn input_from_slice(self, src: &[u8]) -> Result<Input, $crate::io::Error> {
                match self {
                    $(Analysis::$name => {
                        let input = serde_json::from_slice::<<$krate as Env>::Input>(src)
                            .map_err($crate::io::Error::JsonError)?;
                        Ok(Input {
                            analysis: self,
                            json: Arc::new(serde_json::to_value(input).expect("input is always valid json")),
                        })
                    }),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.to_string(), ?src))]
            pub fn output_from_str(self, src: &str) -> Result<Output, $crate::io::Error> {
                match self {
                    $(Analysis::$name => {
                        let output = serde_json::from_str::<<$krate as Env>::Output>(src)
                            .map_err($crate::io::Error::JsonError)?;
                        Ok(Output {
                            analysis: self,
                            json: Arc::new(serde_json::to_value(output).expect("output is always valid json")),
                        })
                    }),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.to_string(), ?src))]
            pub fn output_from_from_bytes(self, src: &[u8]) -> Result<Output, $crate::io::Error> {
                match self {
                    $(Analysis::$name => {
                        let output = serde_json::from_slice::<<$krate as Env>::Output>(src)
                            .map_err($crate::io::Error::JsonError)?;
                        Ok(Output {
                            analysis: self,
                            json: Arc::new(serde_json::to_value(output).expect("output is always valid json")),
                        })
                    }),*
                }
            }
        }

        impl Input {
            #[tracing::instrument(skip_all, fields(analysis = self.analysis.to_string()))]
            pub fn reference_output(&self) -> Result<Output, EnvError> {
                match self.analysis {
                    $(Analysis::$name => {
                        let input: <$krate as Env>::Input = serde_json::from_value((*self.json).clone())
                            .map_err(EnvError::from_parse_input(&self.json))?;
                        let reference_output = <$krate>::run(&input)?;
                        Ok(Output {
                            analysis: self.analysis,
                            json: serde_json::to_value(&reference_output)
                                .expect("all output should be serializable")
                                .into(),
                        })
                    }),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.analysis.to_string()))]
            pub fn validate_output(&self, output: &Output) -> Result<ValidationResult, EnvError> {
                assert_eq!(self.analysis(), output.analysis());

                match self.analysis {
                    $(Analysis::$name => {
                        let input: <$krate as Env>::Input = serde_json::from_value((*self.json).clone())
                            .map_err(EnvError::from_parse_input(&self.json))?;
                        let output: <$krate as Env>::Output = serde_json::from_value((*output.json).clone())
                            .map_err(EnvError::from_parse_output(&output.json))?;
                        <$krate as Env>::validate(&input, &output)
                    }),*
                }
            }
        }

        $(
            impl EnvExt for $krate {
                const ANALYSIS: Analysis = Analysis::$name;

                fn generalize_input(input: &Self::Input) -> Input {
                    Input {
                        analysis: Self::ANALYSIS,
                        json: Arc::new(serde_json::to_value(input).expect("input is always valid json")),
                    }
                }

                fn generalize_output(output: &Self::Output) -> Output {
                    Output {
                        analysis: Self::ANALYSIS,
                        json: Arc::new(serde_json::to_value(output).expect("output is always valid json")),
                    }
                }
            }
        )*
    };
}
