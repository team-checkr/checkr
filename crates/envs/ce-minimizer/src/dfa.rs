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

// pub fn parse_dfa(input: &str) -> Result<RawDFA, String> {

// }

impl DFA {
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge)
    }

    pub fn new(&mut self, automaton: RawDFA) {

    }

    fn find_equivalent_states(&mut self) {
        
    }
}