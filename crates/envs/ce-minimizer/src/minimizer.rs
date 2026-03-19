use core::panic;

use tapi::kind::Name;

use super::{NamedDFA, Node};

enum Equivalence { Equivalent, Distinguishable }

type EquivTable = Vec<Vec<bool>>;

impl NamedDFA {
    fn minimize(&self) -> NamedDFA {

    
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
                        let dp = self.dfa.delta(p, *symbol).expect("valid node expected");
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