use mcltl::verifier;

#[test]
fn it_should_not_hold_simple_until() {
    let property = "a U b";

    let res = verifier::verify(include_str!("./tests/test-data/program1.kripke"), property);
    assert!(res.is_some());
}

#[test]
fn it_should_hold_simple_until() {
    let property = "a U b";

    let res = verifier::verify(include_str!("./tests/test-data/program2.kripke"), property);
    assert!(res.is_none());
}

#[test]
fn it_should_not_hold_simple_until3() {
    let property = "a U b";
    let res = verifier::verify(include_str!("./tests/test-data/program3.kripke"), property);
    assert!(res.is_some());
}

#[test]
fn it_should_not_hold_simple_until4() {
    let property = "a U (b or c)";

    let res = verifier::verify(include_str!("./tests/test-data/program3.kripke"), property);
    assert!(res.is_some());
}

#[test]
fn it_should_not_hold_simple_until5() {
    let property = "(a U (b U c))";

    let res = verifier::verify(include_str!("./tests/test-data/program4.kripke"), property);
    assert!(res.is_none());
}
