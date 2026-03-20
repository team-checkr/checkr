use core::{num, panic};

use tapi::kind::Name;

use super::{NamedDFA, Node, Edge, DFA};

#[derive(PartialEq)]
enum Equivalence { Equivalent, Distinguishable }

type EquivTable = Vec<Vec<bool>>;

impl NamedDFA {
    pub fn minimize(&self) -> NamedDFA {
        let table = self.build_equivalence_table();
        
        //partition the states into blocks of mutually equivalent states
        let n = self.dfa.state_count;
        let mut class = vec![usize::MAX; n]; //class[state] get the equivalence class
        let mut num_classes = 0;

        for p in 0..n {
            if class[p] != usize::MAX { continue; }

            class[p] = num_classes;
            for q in (p+1)..n {
                if self.equivalent(p,q, &table) == Equivalence::Equivalent {
                    class[q] = num_classes;
                }
            }
            num_classes += 1;
        }

        // representative[class_id] = the original state that represents it
        let mut representative = vec![usize::MAX; num_classes];
        for p in 0..n {
            let c = class[p];
            if representative[c] == usize::MAX {
                representative[c] = p; // first state seen = representative
            }
        }

        // new edges
        let mut new_edges: Vec<Edge> = Vec::new();
        for c in 0..num_classes {
            let rep = representative[c];
            // find all edges leaving the representative in the original DFA
            for edge in &self.dfa.edges {
                if edge.from == rep {
                    new_edges.push(Edge {
                        from: c, 
                        symbol: edge.symbol, 
                        to: class[edge.to],
                    });
                }
            }
        }

        // New accepting states = classes whose representative is accepting
        let new_accepting: Vec<Node> = (0..num_classes)
            .filter(|&c| self.dfa.accepting.contains(&representative[c]))
            .collect();

        // 5. New start state = class of the original initial state
        let new_initial = class[self.dfa.initial];

        // 6. New names = name of each class's representative
        let new_names: Vec<String> = (0..num_classes)
            .map(|c| {
                // collect all original state names whose class is c
                let members: Vec<&str> = (0..n)
                    .filter(|&p| class[p] == c)
                    .map(|p| self.names[p].as_str())
                    .collect();
                members.join(",")
            })
            .collect();


        NamedDFA {
            dfa: DFA {
                state_count: num_classes,
                edges: new_edges,
                initial: new_initial,
                accepting: new_accepting,
                alphabet: self.dfa.alphabet.clone(), // alphabet doesn't change
            },
            names: new_names,
        }
    }
    
    fn build_equivalence_table(&self) -> EquivTable {
        let n = self.dfa.state_count;

        // All pairs assumed equivalent (false), (true) pair is distinguishable
        let mut table = vec![vec![false; n]; n];

        // Base case
        for p in 0..n {
            for q in 0..n {
                let p_acc = self.dfa.accepting.contains(&p);
                let q_acc: bool = self.dfa.accepting.contains(&q);
                if p_acc != q_acc {
                    table[p][q] = true;
                    table[q][p] = true;
                }
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for p in 0..n {
                for q in 0..n {
                    if table[p][q] { continue; }
                    for symbol in &self.dfa.alphabet {
                        let dp = self.dfa.delta(p, *symbol).expect("valid node expected");
                        let dq = self.dfa.delta(p, *symbol).expect("valid node expected");
                        if table[dp][dq] {
                            table[p][q] = true;
                            table[q][p] = true;
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }

        table
    }

    fn equivalent(&self, q0:Node, q1:Node, table: &EquivTable) -> Equivalence {
        if q0 >= self.dfa.state_count || q1 >= self.dfa.state_count {
            panic!("States not present in DFA"); 
        }
        if table[q0][q1] {
            Equivalence::Distinguishable
        } else {
            Equivalence::Equivalent
        }
        
    }  
}