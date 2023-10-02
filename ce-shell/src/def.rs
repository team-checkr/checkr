use std::{marker::PhantomData, sync::Arc};

use ce_core::{Env, RenderProps};

use crate::io::{Input, Output};
use dioxus::prelude::*;
use futures_util::StreamExt;
use itertools::Itertools;

#[macro_export]
macro_rules! define_shell {
    ($($krate:path[$name:ident, $display:literal]),*$(,)?) => {
        use std::{str::FromStr, sync::Arc};

        use ce_core::{Env, EnvError, Generate, ValidationResult};
        use itertools::Itertools;

        use dioxus::prelude::*;


        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Analysis {
            $(
                $name,
            )*
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
                    _ => Err(format!("analysis can be one of: {}", [$(stringify!($name),)*].into_iter().format(", "))),
                }
            }
        }

        impl Analysis {
            pub fn options() -> &'static [Analysis] {
                &[$(Analysis::$name),*]
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
            pub fn parse_input(self, src: &str) -> Input {
                match self {
                    $(Analysis::$name => {
                        let input = serde_json::from_str::<<$krate as Env>::Input>(src).unwrap();
                        Input {
                            analysis: self,
                            json: Arc::new(serde_json::to_value(input).expect("input is always valid json")),
                        }
                    }),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.to_string(), ?src))]
            pub fn parse_output(self, src: &str) -> Output {
                match self {
                    $(Analysis::$name => {
                        let output = serde_json::from_str::<<$krate as Env>::Output>(src).unwrap();
                        Output {
                            analysis: self,
                            json: Arc::new(serde_json::to_value(output).expect("output is always valid json")),
                        }
                    }),*
                }
            }
            #[tracing::instrument(skip_all, fields(analysis = self.to_string()))]
            pub fn render<'a, R>(
                self,
                cx: Scope<'a, R>,
                set_input: UseState<Input>,
                reference_output: Output,
                real_output: Output,
            ) -> Element<'a> {
                let input = set_input.get().clone();
                match self {
                    $(Analysis::$name => cx.render(rsx!(def::RenderEnv::<$krate> {
                        analysis: self,
                        set_input: set_input,
                        input: input,
                        reference_output: reference_output,
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
    };
}

#[inline_props]
pub fn RenderEnv<E: Env + 'static>(
    cx: Scope,
    _marker: Option<PhantomData<E>>,
    analysis: crate::Analysis,
    set_input: UseState<Input>,
    input: Input,
    reference_output: Output,
    real_output: Output,
) -> Element<'a> {
    let analysis = *analysis;

    let tx = use_coroutine(cx, |mut rx| {
        to_owned![set_input];
        async move {
            while let Some(new) = rx.next().await {
                let json = Arc::new(serde_json::to_value(new).expect("input is always valid json"));
                let new = Input { analysis, json };
                set_input.set(new);
            }
        }
    });

    if ![
        analysis,
        input.analysis,
        reference_output.analysis,
        real_output.analysis,
    ]
    .iter()
    .all_equal()
    {
        return cx.render(rsx!(div { "Loading..." }));
    }

    let props = use_memo(
        cx,
        (tx, input, reference_output, real_output),
        |(tx, input, reference_output, real_output)| {
            let input = serde_json::from_value((*input.json).clone()).unwrap();
            let reference_output =
                serde_json::from_value((*reference_output.json).clone()).unwrap();
            let real_output = serde_json::from_value((*real_output.json).clone()).unwrap();
            RenderProps {
                set_input: tx,
                input: Arc::new(input),
                reference_output: Arc::new(reference_output),
                real_output: Arc::new(real_output),
                marker: Default::default(),
            }
        },
    );
    E::render(cx, props)
}
