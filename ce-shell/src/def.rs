use std::{marker::PhantomData, sync::Arc};

use ce_core::{AnalysisResult, Env, RenderProps, ValidationResult};

use crate::io::{Input, Output};
use dioxus::prelude::*;
use futures_util::StreamExt;

#[macro_export]
macro_rules! define_shell {
    ($($krate:path[$name:ident, $display:literal]),*$(,)?) => {
        use std::{str::FromStr, sync::Arc};

        use ce_core::{Env, EnvError, Generate, ValidationResult};
        use itertools::Itertools;

        use dioxus::prelude::*;

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
            pub fn parse_input(self, src: &str) -> Result<Input, $crate::io::Error> {
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
            pub fn parse_output(self, src: &str) -> Result<Output, $crate::io::Error> {
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
            #[tracing::instrument(skip_all, fields(analysis = self.to_string()))]
            pub fn render<'a, R>(
                self,
                cx: Scope<'a, R>,
                input: &'a Input,
                set_input: Coroutine<Input>,
                real_output: &'a Option<Output>,
            ) -> Element<'a> {
                match self {
                    $(Analysis::$name => cx.render(rsx!(def::RenderEnv::<$krate> {
                        analysis: self,
                        set_input: set_input,
                        input: input,
                        real_output: real_output,
                    }))),*
                }
            }
        }

        impl Input {
            #[tracing::instrument(skip_all, fields(analysis = self.analysis.to_string()))]
            pub fn reference_output(&self) -> Result<Output, EnvError> {
                match self.analysis {
                    $(Analysis::$name => {
                        type Input = <$krate as Env>::Input;
                        let input: Input = serde_json::from_value((*self.json).clone()).unwrap();
                        Ok(Output {
                            analysis: self.analysis,
                            json: serde_json::to_value(&<$krate>::run(&input)?)
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
                        let input: <$krate as Env>::Input = serde_json::from_value((*self.json).clone()).unwrap();
                        let output: <$krate as Env>::Output = serde_json::from_value((*output.json).clone()).unwrap();
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

#[derive(Props)]
pub struct RenderEnvProps<'a, E: Env + 'static> {
    _marker: Option<PhantomData<&'a E>>,
    analysis: crate::Analysis,
    input: &'a Input,
    set_input: Coroutine<Input>,
    #[props(!optional)]
    real_output: &'a Option<Output>,
}

pub fn RenderEnv<'a, E: Env + 'static>(cx: Scope<'a, RenderEnvProps<'a, E>>) -> Element<'a> {
    let props = &cx.props;
    let analysis = props.analysis;

    let tx = use_coroutine(cx, |mut rx| {
        to_owned![props.set_input];
        async move {
            while let Some(new) = rx.next().await {
                let json = Arc::new(serde_json::to_value(new).expect("input is always valid json"));
                let new = Input { analysis, json };
                set_input.send(new);
            }
        }
    });

    let all_same_analysis = analysis == props.input.analysis();

    let input: &E::Input = use_memo(cx, (props.input,), |(input,)| {
        serde_json::from_value((*input.json).clone()).unwrap()
    });

    let real_result =
        use_memo(
            cx,
            (input, props.real_output),
            |(input, real_output)| match real_output {
                None => AnalysisResult::Nothing,
                Some(real) => {
                    let real = serde_json::from_value((*real.json).clone()).unwrap();
                    let reference = E::run(&input).unwrap();
                    let validation = E::validate(&input, &real).unwrap();
                    AnalysisResult::Active {
                        reference,
                        real,
                        validation,
                    }
                }
            },
        );

    let result = use_state::<AnalysisResult<E>>(cx, || real_result.clone());

    use_effect(cx, (real_result,), |(real_result,)| {
        to_owned![result];
        async move {
            match (real_result, &*result) {
                (
                    AnalysisResult::Nothing,
                    AnalysisResult::Active {
                        reference,
                        real,
                        validation,
                    },
                ) => result.set(AnalysisResult::Stale {
                    reference: reference.clone(),
                    real: real.clone(),
                    validation: validation.clone(),
                }),
                (AnalysisResult::Nothing, _) => {}
                (next, _) => result.set(next),
            };
        }
    });

    use_shared_state_provider::<Option<ValidationResult>>(cx, || None);
    let validation_result = use_shared_state::<Option<ValidationResult>>(cx).unwrap();
    use_effect(cx, (&**result,), |(result,)| {
        to_owned![validation_result];
        async move {
            match result {
                AnalysisResult::Nothing | AnalysisResult::Stale { .. } => {
                    *validation_result.write() = None
                }
                AnalysisResult::Active { validation, .. } => {
                    *validation_result.write() = Some(validation.clone())
                }
            }
        }
    });

    let render_props = use_memo(cx, (tx, input, &**result), |(tx, input, result)| {
        RenderProps::new(tx.clone(), input, result)
    });

    if !all_same_analysis {
        return cx.render(rsx!(div { class: "grid place-items-center text-xl", "Loading..." }));
    }

    E::render(cx, render_props)
}
