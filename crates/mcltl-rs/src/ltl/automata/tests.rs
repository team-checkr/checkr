use itertools::Itertools;

use super::*;
use crate::{
    ltl::expression::{LTLExpression, Literal},
    testing::expect,
};

#[test]
fn it_should_create_graph_from_ltl() {
    let expr: NnfLtl<Literal> = NnfLtl::lit("p").U(NnfLtl::lit("q"));

    let nodes = AutomataGraph::create_graph(&expr);
    assert_eq!(3, nodes.len());
}

fn check_nnf(s: LTLExpression, f: impl FnOnce(String)) {
    let nnf = s.nnf();
    f(nnf.to_string())
}

fn check_nodes(s: LTLExpression, f: impl FnOnce(String)) {
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

fn lit(s: &str) -> LTLExpression {
    LTLExpression::Literal(s.into())
}

#[test]
fn nnf_a() {
    check_nnf(lit("p"), expect!(@"p"));
    check_nnf(lit("p").U(lit("q")), expect!(@"(p U q)"));
}

#[test]
fn nodes_a() {
    check_nodes(
        lit("p"),
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
        lit("p").U(lit("q")),
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
