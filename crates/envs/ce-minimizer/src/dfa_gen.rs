use crate::rand::{Rng, seq::IndexedRandom};

pub struct DfaContext {
    state_count: usize, 
    alphabet: Vec<char>, 
    
}

impl DfaContext {
    
    pub fn new_small<R: Rng>(rng: &mut R) -> Self {
        DfaContext {
            state_count: rng.random_range(2..=4),
            alphabet: vec!['a', 'b'],
        }
    }

}

