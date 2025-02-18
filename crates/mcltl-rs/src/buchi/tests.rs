use super::*;
use crate::gbuchi;

fn lit(s: &str) -> NnfLtl<Literal> {
    NnfLtl::lit(s)
}

#[test]
fn it_should_extract_buchi_from_nodeset() {
    // p U q
    let ltl_expr = lit("p").U(lit("q"));

    let gbuchi = ltl_expr.gba(None);

    insta::assert_snapshot!(gbuchi.display(), @r###"
        States:
         A2 []
           =[p | p,q]=> A2
           =[p | p,q]=> A6
         A6 []
           =[p,q | q]=> A7
         A7 []
           =[*]=> A7
        Initial: A2 A6
        Accept:  [{A6 A7}]
        "###);
}

#[test_log::test]
fn it_should_convert_gba_construct_from_ltl_into_ba() {
    // Fp1 U Gp2
    let ltl_expr = NnfLtl::F(lit("p")).U(NnfLtl::G(lit("q")));

    let gbuchi = ltl_expr.gba(None);

    insta::assert_snapshot!(gbuchi.display(), @r###"
        States:
         A11 []
           =[p,q | q]=> A16
           =[p,q | q]=> A23
         A16 []
           =[p,q | q]=> A16
           =[p,q | q]=> A23
         A23 []
           =[p,q]=> A26
         A26 []
           =[p,q | q]=> A26
         A33 []
           =[p | p,q]=> A4
           =[p | p,q]=> A33
           =[p | p,q]=> A40
         A4 []
           =[ | p | p,q | q]=> A4
           =[ | p | p,q | q]=> A11
           =[ | p | p,q | q]=> A33
           =[ | p | p,q | q]=> A45
         A40 []
           =[p,q | q]=> A26
         A45 []
           =[p,q]=> A26
        Initial: A4 A33 A40
        Accept:  [{A33 A23 A26 A40 A45}, {A11 A16 A23 A26 A40 A45}]
        "###);

    let buchi = gbuchi.to_buchi();

    insta::assert_snapshot!(buchi.display(), @r###"
    States:
     (A11, 0) []
       =[p,q | q]=> (A16, 0)
       =[p,q | q]=> (A23, 0)
     (A11, 1) []
       =[p,q | q]=> (A16, 0)
       =[p,q | q]=> (A23, 0)
     (A16, 0) []
       =[p,q | q]=> (A16, 0)
       =[p,q | q]=> (A23, 0)
     (A16, 1) []
       =[p,q | q]=> (A16, 0)
       =[p,q | q]=> (A23, 0)
     (A23, 0) []
       =[p,q]=> (A26, 1)
     (A23, 1) []
       =[p,q]=> (A26, 0)
     (A26, 0) []
       =[p,q | q]=> (A26, 1)
     (A26, 1) []
       =[p,q | q]=> (A26, 0)
     (A33, 0) []
       =[p | p,q]=> (A4, 1)
       =[p | p,q]=> (A33, 1)
       =[p | p,q]=> (A40, 1)
     (A33, 1) []
       =[p | p,q]=> (A4, 1)
       =[p | p,q]=> (A33, 1)
       =[p | p,q]=> (A40, 1)
     (A4, 0) []
       =[ | p | p,q | q]=> (A4, 0)
       =[ | p | p,q | q]=> (A11, 0)
       =[ | p | p,q | q]=> (A33, 0)
       =[ | p | p,q | q]=> (A45, 0)
     (A4, 1) []
       =[ | p | p,q | q]=> (A4, 1)
       =[ | p | p,q | q]=> (A11, 1)
       =[ | p | p,q | q]=> (A33, 1)
       =[ | p | p,q | q]=> (A45, 1)
     (A40, 0) []
       =[p,q | q]=> (A26, 1)
     (A40, 1) []
       =[p,q | q]=> (A26, 0)
     (A45, 0) []
       =[p,q]=> (A26, 1)
     (A45, 1) []
       =[p,q]=> (A26, 0)
    Initial: (A4, 0) (A33, 0) (A40, 0)
    Accept:  [(A33, 0), (A23, 0), (A26, 0), (A40, 0), (A45, 0)]
    "###);
}

#[test]
fn it_should_convert_gba_into_ba() {
    let gbuchi: GeneralBuchi<String, Literal> = gbuchi! {
        INIT
            [a] => INIT
            [b] => s1
        s1
            [a] => INIT
            [b] => s1
        ===
        init = [INIT]
        accepting = [vec![INIT]]
        accepting = [vec![s1]]
    };

    insta::assert_snapshot!(gbuchi.display(), @r###"
        States:
         "INIT" []
           =[a]=> "INIT"
           =[b]=> "s1"
         "s1" []
           =[a]=> "INIT"
           =[b]=> "s1"
        Initial: "INIT"
        Accept:  [{"INIT"}, {"s1"}]
        "###);

    let buchi = gbuchi.to_buchi();

    insta::assert_snapshot!(buchi.display(), @r###"
    States:
     ("INIT", 0) []
       =[a]=> ("INIT", 1)
       =[b]=> ("s1", 1)
     ("INIT", 1) []
       =[a]=> ("INIT", 1)
       =[b]=> ("s1", 1)
     ("s1", 0) []
       =[a]=> ("INIT", 0)
       =[b]=> ("s1", 0)
     ("s1", 1) []
       =[a]=> ("INIT", 0)
       =[b]=> ("s1", 0)
    Initial: ("INIT", 0)
    Accept:  [("INIT", 0)]
    "###);
}

#[test]
fn it_should_convert_gba_into_ba2() {
    let gbuchi: GeneralBuchi<String, Literal> = gbuchi! {
        INIT
            [a] => q3
            [b] => q2
        q2
            [b] => q2
            [a] => q3
        q3
            [a] => q3
            [b] => q2
        q4
            [a] => q3
            [b] => q2
        ===
        init = [INIT]
        accepting = [vec![INIT, q3]]
        accepting = [vec![INIT, q2]]
    };

    insta::assert_snapshot!(gbuchi.display(), @r###"
        States:
         "INIT" []
           =[a]=> "q3"
           =[b]=> "q2"
         "q2" []
           =[b]=> "q2"
           =[a]=> "q3"
         "q3" []
           =[a]=> "q3"
           =[b]=> "q2"
         "q4" []
           =[a]=> "q3"
           =[b]=> "q2"
        Initial: "INIT"
        Accept:  [{"INIT" "q3"}, {"INIT" "q2"}]
        "###);

    let buchi = gbuchi.to_buchi();

    insta::assert_snapshot!(buchi.display(), @r###"
    States:
     ("INIT", 0) []
       =[a]=> ("q3", 1)
       =[b]=> ("q2", 1)
     ("INIT", 1) []
       =[a]=> ("q3", 0)
       =[b]=> ("q2", 0)
     ("q2", 0) []
       =[b]=> ("q2", 0)
       =[a]=> ("q3", 0)
     ("q2", 1) []
       =[b]=> ("q2", 0)
       =[a]=> ("q3", 0)
     ("q3", 0) []
       =[a]=> ("q3", 1)
       =[b]=> ("q2", 1)
     ("q3", 1) []
       =[a]=> ("q3", 1)
       =[b]=> ("q2", 1)
     ("q4", 0) []
       =[a]=> ("q3", 0)
       =[b]=> ("q2", 0)
     ("q4", 1) []
       =[a]=> ("q3", 1)
       =[b]=> ("q2", 1)
    Initial: ("INIT", 0)
    Accept:  [("INIT", 0), ("q3", 0)]
    "###);
}

#[test]
fn it_should_do_product_of_automata() {
    let alphabet: Alphabet<Literal> = [Literal::from("a"), Literal::from("b")].into();

    let mut buchi1: Buchi<String, Literal> = Buchi::new(alphabet.clone());
    let r1 = buchi1.push("INIT".into());
    let r2 = buchi1.push("r2".into());

    buchi1.add_transition(r1, r1, [Literal::from("a")].into());
    buchi1.add_transition(r1, r2, [Literal::from("b")].into());

    buchi1.add_transition(r2, r2, [Literal::from("b")].into());
    buchi1.add_transition(r2, r1, [Literal::from("a")].into());

    buchi1.add_accepting_state(r1);
    buchi1.add_init_state(r1);

    insta::assert_snapshot!(buchi1.display(), @r###"
        States:
         "INIT" []
           =[a]=> "INIT"
           =[b]=> "r2"
         "r2" []
           =[b]=> "r2"
           =[a]=> "INIT"
        Initial: "INIT"
        Accept:  ["INIT"]
        "###);

    let mut buchi2: Buchi<String, Literal> = Buchi::new(alphabet);
    let q1 = buchi2.push("INIT".into());
    let q2 = buchi2.push("q2".into());

    buchi2.add_transition(q1, q1, [Literal::from("b")].into());
    buchi2.add_transition(q1, q2, [Literal::from("a")].into());

    buchi2.add_transition(q2, q2, [Literal::from("a")].into());
    buchi2.add_transition(q2, q1, [Literal::from("b")].into());

    buchi2.add_accepting_state(q1);
    buchi2.add_init_state(q1);

    insta::assert_snapshot!(buchi2.display(), @r###"
        States:
         "INIT" []
           =[b]=> "INIT"
           =[a]=> "q2"
         "q2" []
           =[a]=> "q2"
           =[b]=> "INIT"
        Initial: "INIT"
        Accept:  ["INIT"]
        "###);

    let buchi_product = buchi1.product(&buchi2);

    insta::assert_snapshot!(buchi_product.display(), @r###"
        States:
         ("INIT", "INIT") []
           =[a]=> ("INIT", "q2")
           =[b]=> ("r2", "INIT")
         ("INIT", "q2") []
           =[a]=> ("INIT", "q2")
           =[b]=> ("r2", "INIT")
         ("r2", "INIT") []
           =[b]=> ("r2", "INIT")
           =[a]=> ("INIT", "q2")
        Initial: ("INIT", "INIT")
        Accept:  [("INIT", "INIT")]
        "###);
}

#[test]
fn it_should_extract_buchi_from_nodeset2() {
    // p1 U (p2 U p3)
    let ltl_expr = lit("p1").U(lit("p2").U(lit("p3")));

    let gbuchi = ltl_expr.gba(None);

    insta::assert_snapshot!(gbuchi.display(), @r###"
        States:
         A10 []
           =[p1,p2 | p1,p2,p3 | p2 | p2,p3]=> A10
           =[p1,p2 | p1,p2,p3 | p2 | p2,p3]=> A14
         A14 []
           =[p1,p2,p3 | p1,p3 | p2,p3 | p3]=> A15
         A15 []
           =[*]=> A15
         A2 []
           =[p1 | p1,p2 | p1,p2,p3 | p1,p3]=> A2
           =[p1 | p1,p2 | p1,p2,p3 | p1,p3]=> A7
           =[p1 | p1,p2 | p1,p2,p3 | p1,p3]=> A8
         A7 []
           =[p1,p2 | p1,p2,p3 | p2 | p2,p3]=> A10
           =[p1,p2 | p1,p2,p3 | p2 | p2,p3]=> A14
         A8 []
           =[p1,p2,p3 | p1,p3 | p2,p3 | p3]=> A15
        Initial: A2 A7 A8
        Accept:  [{A7 A8 A10 A14 A15}, {A2 A8 A14 A15}]
        "###);
}

#[test]
fn it_should_extract_buchi_from_nodeset3() {
    // Fp1 U Gp2
    let ltl_expr = NnfLtl::F(lit("p")).U(NnfLtl::G(lit("q")));

    let gbuchi = ltl_expr.gba(None);

    insta::assert_snapshot!(gbuchi.display(), @r###"
        States:
         A11 []
           =[p,q | q]=> A16
           =[p,q | q]=> A23
         A16 []
           =[p,q | q]=> A16
           =[p,q | q]=> A23
         A23 []
           =[p,q]=> A26
         A26 []
           =[p,q | q]=> A26
         A33 []
           =[p | p,q]=> A4
           =[p | p,q]=> A33
           =[p | p,q]=> A40
         A4 []
           =[ | p | p,q | q]=> A4
           =[ | p | p,q | q]=> A11
           =[ | p | p,q | q]=> A33
           =[ | p | p,q | q]=> A45
         A40 []
           =[p,q | q]=> A26
         A45 []
           =[p,q]=> A26
        Initial: A4 A33 A40
        Accept:  [{A33 A23 A26 A40 A45}, {A11 A16 A23 A26 A40 A45}]
        "###);
}

#[test]
fn it_should_extract_buchi_from_nodeset4() {
    // Fp1 U Gp2
    let ltl_expr = NnfLtl::G(lit("p1")).U(lit("p2"));

    let gbuchi = ltl_expr.gba(None);

    insta::assert_snapshot!(gbuchi.display(), @r###"
        States:
         A11 []
           =[p1,p2]=> A14
         A14 []
           =[p1 | p1,p2]=> A14
         A19 []
           =[*]=> A19
         A3 []
           =[p1,p2 | p2]=> A19
         A4 []
           =[p1 | p1,p2]=> A4
           =[p1 | p1,p2]=> A11
        Initial: A3 A4
        Accept:  [{A3 A11 A14 A19}]
        "###);
}
