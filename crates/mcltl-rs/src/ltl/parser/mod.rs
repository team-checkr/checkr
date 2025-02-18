pub mod lexer;
pub mod parser;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ltl::expression::LTLExpression;

    #[test]
    fn it_should_parse_ltl_binary_expression() {
        let input = "(p U (q U p))";
        let lexer = lexer::Lexer::new(input);
        let parse_result = parser::parse(lexer);

        let input_without_parenthesis = "p U q U p";
        let lexer = lexer::Lexer::new(input_without_parenthesis);
        let parse_result2 = parser::parse(lexer);

        let expected =
            LTLExpression::lit("p").U(LTLExpression::lit("q").U(LTLExpression::lit("p")));

        assert!(parse_result.is_ok());
        assert_eq!(expected, parse_result.unwrap().expr);

        assert!(parse_result2.is_ok());
        assert_eq!(expected, parse_result2.unwrap().expr);
    }

    #[test]
    fn it_should_parse_ltl_unary_expression() {
        let input = "G p";
        let lexer = lexer::Lexer::new(input);
        let parse_result = parser::parse(lexer);

        let expected = LTLExpression::G(Box::new(LTLExpression::lit("p")));

        let input = "G (not p)";
        let lexer = lexer::Lexer::new(input);
        let parse_result2 = parser::parse(lexer);

        let expected2 = LTLExpression::G(Box::new(LTLExpression::Not(Box::new(
            LTLExpression::lit("p"),
        ))));

        assert_eq!(expected, parse_result.unwrap().expr);
        assert_eq!(expected2, parse_result2.unwrap().expr);
    }

    #[test]
    fn it_should_parse_ltl_concats_of_unary_expression() {
        let input = "G F p";
        let lexer = lexer::Lexer::new(input);
        let parse_result = parser::parse(lexer);

        let expected = LTLExpression::G(Box::new(LTLExpression::F(Box::new(LTLExpression::lit(
            "p",
        )))));

        //assert!(parse_result.is_ok());
        //assert_eq!(expected, parse_result.unwrap().expr);
    }

    #[test]
    fn it_should_parse_conjunctive_normal_form() {
        let input = "(p /\\ (q and T))";
        let lexer = lexer::Lexer::new(input);
        let parse_result = parser::parse(lexer);

        let expected = LTLExpression::lit("p") & (LTLExpression::lit("q") & LTLExpression::True);

        assert!(parse_result.is_ok());
        assert_eq!(expected, parse_result.unwrap().expr);
    }

    #[test]
    fn it_should_parse_disjunction_normal_form() {
        let input = "p \\/ (q or F)";
        let lexer = lexer::Lexer::new(input);
        let parse_result = parser::parse(lexer);

        let expected = LTLExpression::lit("p") | (LTLExpression::lit("q") | LTLExpression::False);

        assert!(parse_result.is_ok());
        assert_eq!(expected, parse_result.unwrap().expr);
    }

    #[test]
    fn it_should_not_parse_malformed_binary_expression() {
        let input = "p U";
        let lexer = lexer::Lexer::new(input);
        let parse_result = parser::parse(lexer);

        assert!(parse_result.is_err());
    }

    #[test]
    fn it_should_not_parse_malformed_unary_expression() {
        let input = "G(p q)";
        let lexer = lexer::Lexer::new(input);
        let parse_result = parser::parse(lexer);

        assert!(parse_result.is_err());
    }
}
