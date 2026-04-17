use crate::rand::{Rng, seq::IndexedRandom};

use std::collections::HashMap;

pub fn generate_random_dfa<R: Rng>(rng: &mut R, state_count: usize, allow_nondeterminism: bool) -> String {
        
        let mut states: Vec<String> = (0..state_count)
            .map(|i| format!("q{i}"))
            .collect();
        
        let alphabet_pool = vec!['0', '1', '2', '3'];
        let mut alphabet: Vec<char> = alphabet_pool
            .into_iter()
            .filter(|_| rng.random_bool(0.7))
            .collect();

        if alphabet.is_empty() {
            alphabet.push('0'); 
        }

        let mut accepting_states: Vec<String> = states
            .iter()
            .filter(|_| rng.random_bool(0.3))
            .cloned()
            .collect();

        if accepting_states.is_empty() {
            accepting_states.push(states[0].clone()); 
        }

        let mut transition_map: HashMap<(String, char), String> = HashMap::new();

        let mut transitions: Vec<String> = Vec::new();
        for state in &states {
            for symbol in &alphabet {
                let target = states.choose(rng).unwrap();
                transition_map.insert((state.clone(), *symbol), target.clone());
                transitions.push(format!("{state}, {symbol} -> {target}"));
            }
        }

        if rng.random_bool(0.5) {
            let dup = "q0'".to_string();
            states.push(dup.clone());
            if accepting_states.contains(&"q0".to_string()) {
                accepting_states.push(dup.clone());
            }

            for symbol in &alphabet {
                let target = transition_map[&("q0".to_string(), *symbol)].clone();
                transitions.push(format!("{dup}, {symbol} -> {target}"));
            }
            
            //After creating a duplicate state, nothing points to it
            //A transition that points to q0 will be made to point to q0'
            for transition in transitions.iter_mut() {
                if transition.ends_with("-> q0") {
                    *transition = transition.replace("-> q0", "-> q0'");
                    break;
                }
            }
        }

        if allow_nondeterminism {
            let state = states.choose(rng).unwrap();
            let symbol = alphabet.choose(rng).unwrap();
            let target = states.choose(rng).unwrap();
            let transition = format!("{state}, {symbol} -> {target}"); 
            if !transitions.contains(&transition) {
                transitions.push(transition);
            }
        }

        let dfa = format!(
            "states: {}\ninitial: q0\nalphabet: {}\naccepting: {}\ntransitions:\n{}",
            states.join(" "),
            alphabet.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" "), 
            accepting_states.join(" "), 
            transitions.join("\n")
        );

        dfa 
}
