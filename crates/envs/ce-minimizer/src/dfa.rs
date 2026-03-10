use std::{collections::HashMap, usize};

pub type Node = usize;

#[derive(Debug)]
pub struct State {
    id: Node,
    name: String,
    accepting: bool,
    initial: bool
}

#[derive(Default, Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize)]
pub struct DFA {
    state_count: usize,
    edges: Vec<Edge>,
    initial: Node, 
    accepting: Vec<Node>,
    alphabet: Vec<char>
}

#[derive(Default, Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize)]
pub struct Edge {
    from: Node,
    symbol: char,
    to: Node,
}

pub struct NamedDFA {
    pub dfa: DFA,
    pub names: Vec<String>,  // names[i] = name of state i
}

#[derive(Default, Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize)]
pub struct RawDFA {
    state_names: Vec<String>, 
    alphabet: Vec<char>, 
    initial: Option<String>, 
    accepting: Vec<String>, 
    transitions: Vec<(String, char, String)> 
}

#[derive(Debug, thiserror::Error)]
pub enum ParseErrorDFA {
    #[error("invalid transition")]
    BadTransition,

    #[error("invalid alphabet symbol: one char expected")]
    BadAlphabetSymbol,

    #[error("initial state missing")]
    NoInitialState,

    #[error("accepting states missing")]
    NoAcceptingStates,

    #[error("bad input")]
    BadInput,
}

pub fn parse_dfa(input: &str) -> Result<RawDFA,ParseErrorDFA> {
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
                    //Can also be ignored, error thrown when a transition symbol is not a char
                    return Err(ParseErrorDFA::BadAlphabetSymbol);
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
        if let Some(_) = line.strip_prefix("transitions:") {
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
                    return Err(ParseErrorDFA::BadTransition);
                }
            }
            Section::Null | Section::Initial => {
                return Err(ParseErrorDFA::BadInput);
            }
        }
    }
    
    Ok(
    RawDFA {
        state_names,
        alphabet,
        initial,
        accepting,
        transitions,
    })
}

impl NamedDFA {
    pub fn build(raw_dfa: RawDFA) -> Result<Self,ParseErrorDFA> {
        let name_to_index: HashMap<String, Node> = raw_dfa.state_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();

        // Check if raw dfa has an initial state and that the state declared in initial is also in states
        let initial = *name_to_index.get(&raw_dfa.initial.ok_or(ParseErrorDFA::NoInitialState)?).ok_or(ParseErrorDFA::BadInput)?;

        let accepting: Result<Vec<Node>, ParseErrorDFA> = raw_dfa.accepting.iter()
            .map(|name| Ok(name_to_index.get(name).copied().ok_or(ParseErrorDFA::BadInput)?))
            .collect();

        let accepting = accepting?;

        let edges: Result<Vec<Edge>, ParseErrorDFA> = raw_dfa.transitions.iter()
            .map(|(from, sym, to)| Ok(Edge {
                from: *name_to_index.get(from).ok_or(ParseErrorDFA::BadTransition)?,
                symbol: *sym,
                to: *name_to_index.get(to).ok_or(ParseErrorDFA::BadTransition)?,
            }))
            .collect();
        
        let edges = edges?;
            
        Ok(
        NamedDFA {
            dfa: DFA { state_count: raw_dfa.state_names.len(), edges, initial, accepting, alphabet: raw_dfa.alphabet },
            names: raw_dfa.state_names
        })
    } 
}

impl DFA {
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge)
    }  

    pub fn to_dot(&self) -> String {
        let mut s = "digraph DFA {\n  rankdir=LR\n\n".to_string();

        // Initial state arrow
        s.push_str("  __start [label=\"\", shape=none]\n");
        s.push_str(&format!("  __start -> {}\n", self.initial));

        // Accept states
        for &node in &self.accepting {
            s.push_str(&format!("  {} [shape=doublecircle]\n", node));
        }

        // ////multiple symbols on one edge
        // let mut edge_map: HashMap<(Node, Node), Vec<char>> = HashMap::new();
        // for edge in &self.edges {
        //     edge_map.entry((edge.from, edge.to)).or_default().push(edge.symbol);
        // }

        for edge in &self.edges {
            s.push_str(&format!("  {} -> {} [label=\"{}\"]\n", edge.from, edge.to, edge.symbol));
        }

        s.push_str("}");
        s
    }

    fn find_equivalent_states(&mut self) {
        
    }
}