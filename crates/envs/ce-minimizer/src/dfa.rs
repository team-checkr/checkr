use std::{collections::HashMap, usize};

use itertools::enumerate;

pub type Node = usize;

#[derive(Default, Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize)]
pub struct DFA {
    pub state_count: usize,
    edges: Vec<Edge>,
    pub initial: Node, 
    pub accepting: Vec<Node>,
    pub alphabet: Vec<char>
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

    #[error("state is not declared")]
    MissingState,

    #[error("alphabet symbol found in transition is not declared")]
    MissingSymbol,

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
        if from.contains(char::is_whitespace) || to.contains(char::is_whitespace) { return None; }

        Some((from, sym, to))
    }

    let mut has_transitions_section = false;

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
            has_transitions_section = true;
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
    
    if !has_transitions_section {
        return Err(ParseErrorDFA::BadInput);
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
        // start with declared states, then add any referenced but undeclared ones
        // infer states
        let mut all_names: Vec<String> = raw_dfa.state_names.clone();
        
        for (from, _, to) in &raw_dfa.transitions {
            if !all_names.contains(from) { all_names.push(from.clone()) }
            if !all_names.contains(to) { all_names.push(to.clone()) }
        }
        if let Some(ref init) = raw_dfa.initial {
            if !all_names.contains(init) { all_names.push(init.clone()); }
        }
        for name in &raw_dfa.accepting {
            if !all_names.contains(name) { all_names.push(name.clone()); }
        }

        // infer alphabet
        let mut all_alphabet_symbols = raw_dfa.alphabet.clone();

        for (_ , symbol, _) in &raw_dfa.transitions {
            if !all_alphabet_symbols.contains(symbol) { all_alphabet_symbols.push(*symbol) }
        }
        
        // check against declared states
        if !raw_dfa.state_names.is_empty() {
            for name in &all_names {
                if !raw_dfa.state_names.contains(name) {
                    return Err(ParseErrorDFA::MissingState);
                }
            }
        }

        // check against declared alphabet symbols
        if !raw_dfa.alphabet.is_empty() {
            for sym in &all_alphabet_symbols {
                if !raw_dfa.alphabet.contains(sym) {
                    return Err(ParseErrorDFA::MissingSymbol);
                }
            }   
        }

        // create index/id for the states
        let name_to_index: HashMap<String, Node> = all_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();

        // Check if raw dfa has an initial state and that the state declared in initial is also in states
        let initial = *name_to_index
            .get(&raw_dfa.initial.ok_or(ParseErrorDFA::NoInitialState)?)
            .ok_or(ParseErrorDFA::BadInput)?;

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
            dfa: DFA { state_count: all_names.len(), edges, initial, accepting, alphabet: all_alphabet_symbols },
            names: all_names
        })
    } 

    pub fn to_dot(&self) -> String {
        let mut s = "digraph DFA {\n  rankdir=LR\n\n".to_string();

        s.push_str("  // States\n");
        s.push_str("  __start [label=\"\", shape=none]\n"); // startstate
        for (node, state) in enumerate(&self.names) {
            s.push_str(&format!("  {} [label=\"{}\", shape={}]\n",
                node, 
                state, 
                if self.dfa.accepting.contains(&node) {"doublecircle"} else {"circle"}
            ));
        }
        s.push_str("\n");

        s.push_str("  // Initial\n");
        s.push_str(&format!("  __start -> {}\n", self.dfa.initial));
        s.push_str(&format!("\n"));

        s.push_str("  // Transitions\n");
        // multiple symbols on one edge
        
        let mut edge_map: HashMap<(Node, Node), Vec<char>> = HashMap::new();
        for edge in &self.dfa.edges {
            edge_map.entry((edge.from, edge.to)).or_default().push(edge.symbol);
        }

        for ((from, to), symbols) in edge_map {
            s.push_str(&format!("  {} -> {} [label=\"{}\"]\n", 
                from, 
                to, 
                {
                    let mut chars: Vec<String> = symbols.iter().map(|c| c.to_string()).collect();
                    chars.sort();
                    chars.join(",")
                }
            ));
        }

        s.push_str("}");
        s
    }
}

#[derive(Debug, Clone, PartialEq, tapi::Tapi, serde::Serialize, serde::Deserialize, thiserror::Error)]
pub enum SemanticErrorDFA {
    #[error("incomplete DFA")]
    Incomplete,

    #[error("not a DFA")]
    Nondeterministic,

    #[error("invalid initial state")]
    InvalidInitialState,

    #[error("invalid accepting state")]
    InvalidAcceptingState,
}

impl DFA {
    pub fn validate(&self) -> Vec<SemanticErrorDFA> {
        let mut errors = Vec::new();

        // completeness (check for  missing transitions)
        let incomplete = (0..self.state_count).any(|node| {
            self.alphabet.iter().any(|symbol| {
                !self.edges.iter().any(|e| e.from == node && e.symbol == *symbol)
            })
        });
        if incomplete { errors.push(SemanticErrorDFA::Nondeterministic); }

        // determinism
        let nondeterministic = (0..self.state_count).any(|node| {
            self.alphabet.iter().any(|symbol| {
                self.edges.iter().filter(|e| e.from == node && e.symbol == *symbol).count() > 1
            })
        });
        if nondeterministic { errors.push(SemanticErrorDFA::Nondeterministic); }

        // valid initial state
        if self.initial >= self.state_count {
            errors.push(SemanticErrorDFA::InvalidInitialState);
        }

        // valid accepting states
        if self.accepting.iter().any(|&acc| acc >= self.state_count) {
            errors.push(SemanticErrorDFA::InvalidAcceptingState);
        }

        errors
    }

    pub fn delta(&self, node:Node, symbol:char) -> Option<Node> {
        for edge in &self.edges {
            if edge.from == node && edge.symbol == symbol {
                return Some(edge.to);
            } 
        }

        None
    }
}