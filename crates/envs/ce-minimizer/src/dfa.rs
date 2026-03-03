

type Table = Vec<Vec<bool>>;
pub type Node = usize;

#[derive(Default, Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    from: Node,
    symbol: char,
    to: Node,
}

#[derive(Debug)]
pub struct State {
    position: Node,
    state_name: String,
    accepting: bool,
    initial: bool
}

#[derive(Default, Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize)]
pub struct DFA {
    edges: Vec<Edge>
}

#[derive(Default, Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize)]
pub struct RawDFA {
    state_names: Vec<String>, 
    alphabet: Vec<char>, 
    initial: Option<String>, 
    accepting: Vec<String>, 
    transitions: Vec<(String, char, String)> 
}

// #[derive(Debug, Error)]
// pub enum ParseErrorDFA {
//     #[error("missing")]
//     MissingStates,

//     #[error("invalid transition at line")]
//     BadTransition,
// }

pub fn parse_dfa(input: &str) -> RawDFA {
    #[derive(PartialEq)]
    enum Section { Null, States, Initial, Accepting, Alphabet, Transitions}
    let mut current_section = Section::Null;

    let mut state_names: Vec<String> = Vec::new();
    let mut alphabet: Vec<char> = Vec::new();
    let mut initial: Option<String> = None;
    let mut accepting: Vec<String> = Vec::new();
    let mut transitions: Vec<(String, char, String)> = Vec::new();

    // helper: split "q0,q1 q2" into ["q0","q1","q2"]
    fn split_list(s: &str) -> Vec<String> {
        s.split(|c: char| c == ',' || c.is_whitespace())
            .filter(|t| !t.is_empty())
            .map(|t| t.trim().to_string())
            .collect()
    }

    fn parse_one_char(s: &str) -> Option<char> {
        let mut it = s.trim().chars();
        let c = it.next()?;
        if it.next().is_some() { return None; }
        Some(c)
    }

    fn parse_transition(line: &str) -> Option<(String, char, String)> {
        // expects: from,sym -> to
        let (lhs, rhs) = line.split_once("->")?;
        let to = rhs.trim().to_string();

        let (from, sym_str) = lhs.trim().split_once(',')?;
        let from = from.trim().to_string();

        let sym = parse_one_char(sym_str)?;
        if from.is_empty() || to.is_empty() { return None; }

        Some((from, sym, to))
    }

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        
        if let Some(rest) = line.strip_prefix("states:") {
            current_section = Section::States;
            state_names.extend(split_list(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("alphabet:") {
            current_section = Section::Alphabet;
            for token in split_list(rest) {
                if let Some(c) = parse_one_char(&token) {
                    alphabet.push(c);
                } else {
                    // invalid symbol -> in strict mode you'd error; here we just ignore or panic
                    panic!("Invalid alphabet symbol: {token}");
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("initial:") {
            current_section = Section::Initial;
            let v = rest.trim();
            if !v.is_empty() {
                initial = Some(v.to_string());
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("accepting:") {
            current_section = Section::Accepting;
            accepting.extend(split_list(rest));
            continue;
        }
        if line == "transitions:" {
            current_section = Section::Transitions;
            continue;
        }

         // Continuation lines
        match current_section {
            Section::States => {
                state_names.extend(split_list(line));
            }
            Section::Alphabet => {
                for token in split_list(line) {
                    if let Some(c) = parse_one_char(&token) {
                        alphabet.push(c);
                    }
                }
            }
            Section::Accepting => {
                accepting.extend(split_list(line));
            }
            Section::Transitions => {
                if let Some(t) = parse_transition(line) {
                    transitions.push(t);
                } else {
                    panic!("Invalid transition line: {line}");
                }
            }
            Section::Null | Section::Initial => {
                // line outside a section: ignore or panic
                panic!("Line outside a section: {line}");
            }
        }
    }
    
   
    RawDFA {
        state_names,
        alphabet,
        initial,
        accepting,
        transitions,
    }
}

impl DFA {
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge)
    }

    pub fn new(&mut self, automaton: RawDFA) {

    }

    fn find_equivalent_states(&mut self) {
        
    }
}