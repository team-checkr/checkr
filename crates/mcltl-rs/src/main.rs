use mcltl::buchi::{BuchiLike, ProductBuchi};
use mcltl::ltl::expression::LTLExpression;
use mcltl::verifier::kripke;

use clap::Parser;

use std::convert::TryFrom;
use std::fs;

macro_rules! ok {
    ($arg : expr) => {
        let padding = " ".repeat(75 - $arg.len());
        println!("{}{}[\x1b[1;32mOK\x1b[0m]", $arg, padding);
    };
}

macro_rules! error {
    ($arg : expr, $reason : expr) => {
        let padding = " ".repeat(75 - $arg.len());
        eprintln!("{}{}[\x1b[1;31mERROR\x1b[0m]", $arg, padding);
        eprintln!("failed due to: {}", $reason);
    };
}

#[derive(Parser)]
#[command(version = "1.0tt")]
struct Opts {
    #[clap(short = 'k', long = "program")]
    program_path: String,
    #[clap(short = 'p', long = "property")]
    property: String,
}

fn verify_property(contents: &str, opts: Opts) {
    let kripke_program = kripke::KripkeStructure::parse(contents);

    if let Err(e) = kripke_program {
        error!("Parsing kripke program", e);
        return;
    } else {
        ok!("Parsing kripke program");
    }

    let buchi_program = kripke_program.unwrap().to_buchi(None);

    let ltl_property = LTLExpression::try_from(opts.property.as_str());

    if let Err(e) = ltl_property {
        error!("Parsing LTL property", e);
        return;
    } else {
        ok!("Parsing LTL property");
    }

    let nnf_ltl_property = ltl_property.unwrap().nnf();
    ok!("Converting LTL property in NNF");

    let gbuchi_property = nnf_ltl_property.gba(None);
    ok!("Constructing the graph of the LTL property");

    let buchi_property = gbuchi_property.to_buchi();
    ok!("converting the generalized Buchi automaton into classic Buchi automaton");

    let product_ba = ProductBuchi::new(&buchi_program, &buchi_property);
    ok!("Constructing the product of program and property automata");

    let res = product_ba.find_accepting_cycle();

    if let Some(cycle) = res {
        eprintln!("\n\x1b[1;31mResult: LTL property does not hold\x1b[0m");
        eprintln!("counterexample:\n");

        for (top, _) in cycle.iter() {
            eprint!("{} → ", buchi_program.fmt_node(top));
        }
    } else {
        println!("\n\x1b[1;32mResult: LTL property hold!\x1b[0m");
    }
}

fn main() {
    let opts: Opts = Opts::parse();

    let contents = fs::read_to_string(opts.program_path.as_str());

    if let Err(e) = contents {
        error!(
            format!("Loading kripke file at {}", opts.program_path.as_str()),
            e
        );
    } else {
        ok!("Loading kripke file");
        verify_property(&contents.unwrap(), opts);
    }
}
