use std::fmt;

use itertools::Itertools;

use crate::{
    ltl::{self, expression::Literal},
    testing::expect,
};

use super::*;

#[test]
fn it_should_create_graph_from_ltl() {
    let expr: NnfLtl<Literal> = NnfLtl::lit("p").U(NnfLtl::lit("q"));

    let nodes = AutomataGraph::create_graph(&expr);
    assert_eq!(3, nodes.len());
}

fn check_nnf(src: impl fmt::Display, f: impl FnOnce(String)) {
    let s: ltl::expression::LTLExpression =
        ltl::expression::LTLExpression::try_from(src.to_string().as_str()).unwrap();
    let nnf = s.nnf();
    f(nnf.to_string())
}

fn check_nodes(src: impl fmt::Display, f: impl FnOnce(String)) {
    let s: ltl::expression::LTLExpression =
        ltl::expression::LTLExpression::try_from(src.to_string().as_str()).unwrap();
    let nnf = s.nnf();
    let nodes = AutomataGraph::create_graph(&nnf);
    f(format!(
        "{}",
        nodes
            .iter()
            .map(|n| {
                format!(
                    "{:?}
                        incoming: [{:?}]
                        next:     [{}]
                        old:      [{}]
                        new:      [{}]",
                    n.name,
                    n.incoming.iter().format(", "),
                    n.next.iter().format(", "),
                    n.oldf.iter().format(", "),
                    n.newf.iter().format(", ")
                )
                .lines()
                .map(|l| l.trim())
                .join("\n  ")
            })
            .format("\n")
    ));
}

#[test]
fn nnf_a() {
    check_nnf("p", expect!(@"p"));
    check_nnf("p U q", expect!(@"(p U q)"));
}

#[test]
fn nodes_a() {
    check_nodes(
        "p",
        expect!(@r###"
            A1
              incoming: [A0i]
              next:     []
              old:      [p]
              new:      []
            A2
              incoming: [A1, A2]
              next:     []
              old:      []
              new:      []
            "###),
    );
    check_nodes(
        "p U q",
        expect!(@r###"
            A2
              incoming: [A0i, A2]
              next:     [(p U q)]
              old:      [p, (p U q)]
              new:      []
            A6
              incoming: [A0i, A2]
              next:     []
              old:      [q, (p U q)]
              new:      []
            A7
              incoming: [A6, A7]
              next:     []
              old:      []
              new:      []
            "###),
    );
}
