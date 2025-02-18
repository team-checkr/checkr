pub mod dot;

#[macro_export]
macro_rules! buchi {
    (
        $(
            $src: ident
                $([$($ltl:ident),*] => $dest: ident)*
        )*
        ===
        init = [$( $init:ident ),*]
        accepting = [$( $accepting_state:ident ),*]
    ) => {{
        use $crate::buchi::BuchiLikeMut;

        let alphabet = [$($($(Literal::from(stringify!($ltl)),)*)*)*].into_iter().collect();
        let mut graph = Buchi::new(alphabet);
        $(
            #[allow(unused, unused_mut, non_snake_case)]
            let mut $src = graph.push(stringify!($src).to_string());
            $(
                let dest = graph.push(stringify!($dest).into());
                graph.add_transition($src, dest, [$(Literal::from(stringify!($ltl))),*].into());
            )*
        )*

        $(graph.add_init_state($init);)*
        $(graph.add_accepting_state($accepting_state);)*

        graph
    }};
}

#[macro_export]
macro_rules! gbuchi {
    (
        $(
            $src: ident
                $([$ltl:ident] => $dest: ident)*
        )*
        ===
        init = [$( $init:ident ),*]
        $(accepting = [$( $accepting_states:expr ),*])*
    ) => {{
        use $crate::buchi::BuchiLikeMut;

        let alphabet = [$($(Literal::from(stringify!($ltl)),)*)*].into_iter().collect();
        let mut graph = GeneralBuchi::new(alphabet);
        $(
            #[allow(unused_mut, non_snake_case)]
            let mut $src = graph.push(stringify!($src).to_string());
            $(
                let dest = graph.push(stringify!($dest).into());
                graph.add_transition($src, dest, [stringify!($ltl).into()].into());
            )*
        )*

        $(graph.add_init_state($init);)*
        $($(graph.add_accepting_state(&$accepting_states.into_iter().collect());)*)*

        graph
    }};
}

#[macro_export]
macro_rules! kripke {
    (
        $(
            $world:ident = [$( $prop:ident),*]
        )*
        ===
        $(
            $src:ident R $dst:ident
        )*
        ===
        init = [$( $init:ident ),*]
    ) => {{
        let mut kripke = KripkeStructure::<String, Literal>::new(vec![$(stringify!($init).into(),)*]);

        $(
            let $world = kripke.add_node(stringify!($world).into(), [$(stringify!($prop).into()),*].into_iter().collect());
        )*

        $(
            kripke.add_relation($src.clone(), $dst.clone());
        )*

        kripke
    }};
}
