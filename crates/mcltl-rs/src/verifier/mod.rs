use crate::{
    buchi::{self, ProductBuchi},
    ltl::{
        automata::AutomataId,
        expression::{LTLExpression, Literal},
    },
    verifier::model_checker::ProductAcceptingCycle,
};

pub mod kripke;
pub mod model_checker;

//WARN: use only integration tests for now until the API is stable
pub fn verify<'a>(
    program: &'a str,
    property: &'a str,
) -> Option<ProductAcceptingCycle<String, (AutomataId, usize), Literal>> {
    let kripke_program =
        kripke::KripkeStructure::parse(program).expect("cannot convert into kripke structure");
    let buchi_program: buchi::Buchi<String, Literal> = kripke_program.clone().to_buchi(None);

    let nnf_ltl_property = LTLExpression::try_from(property)
        .expect("cannot convert try form")
        .nnf();

    let gbuchi_property = nnf_ltl_property.gba(None);

    let buchi_property = gbuchi_property.to_buchi();

    let product_ba = ProductBuchi::new(&buchi_program, &buchi_property);

    product_ba.find_accepting_cycle()
}
