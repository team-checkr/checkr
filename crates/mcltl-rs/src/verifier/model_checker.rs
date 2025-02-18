use itertools::Itertools;

use crate::{
    buchi::{
        AtomicProperty, Buchi, BuchiLike, BuchiNode, BuchiNodeId, ProductBuchi, ProductBuchiNodeId,
        ProductBuchiNodeSet,
    },
    nodes::NodeSet,
    state::State,
};

impl<S: State, AP: AtomicProperty> Buchi<S, AP> {
    /// Find a cycle containing an accepting state if it exists.
    ///
    /// Implementation of Algorithm B from ["Memory-Efficient Algorithms for the
    /// Verification of Temporal Properties" by M. Vardi, P. Wolper, M.
    /// Yannakakis](https://link.springer.com/content/pdf/10.1007/bf00121128.pdf)
    pub fn find_accepting_cycle(&self) -> Option<AcceptingCycle<S, AP>> {
        AcceptingCycle::find(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct AcceptingCycle<S, AP: AtomicProperty> {
    S1: Vec<BuchiNodeId<S, AP>>,
    S2: Vec<BuchiNodeId<S, AP>>,
}

impl<S: State, AP: AtomicProperty> AcceptingCycle<S, AP> {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.S1.len() + self.S2.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = BuchiNodeId<S, AP>> + '_ {
        self.S1.iter().chain(self.S2.iter()).copied()
    }
    /// Implementation of Algorithm B from ["Memory-Efficient Algorithms for the
    /// Verification of Temporal Properties" by M. Vardi, P. Wolper, M.
    /// Yannakakis](https://link.springer.com/content/pdf/10.1007/bf00121128.pdf)
    fn find<B: BuchiLike<S, AP, NodeId = BuchiNodeId<S, AP>>>(product_buchi: &B) -> Option<Self> {
        #[allow(non_snake_case)]
        for s0 in product_buchi.init_states() {
            // S1 := {s0}
            let mut S1 = [s0].to_vec();
            // S2 := ∅
            let mut S2 = Vec::new();

            // M1 := M2 := 0
            let mut M1: NodeSet<BuchiNode<S, AP>> = Default::default();
            let mut M2: NodeSet<BuchiNode<S, AP>> = Default::default();

            // while S1 ≠ ∅
            while let Some(&x) = S1.last() {
                // tracing::debug!(?S1, ?S2, ?M1, ?M2, "outer");

                // x := top(S1)

                // tracing::debug!(adj=?product_buchi.adj(x), ids=?product_buchi.adj_ids(x).collect_vec(), found=?product_buchi.adj_ids(x).find(|y|
                //     {let res = !M1.contains(*y);
                //     // tracing::debug!(?y, set=?M1, found=?res);
                //     res}
                // ));
                // if there is a y in succ(x) with M1[h(y)] = 0
                if let Some(y) = product_buchi.adj_ids(x).find(|y| !M1.contains(*y)) {
                    // tracing::debug!(?x, ?y, "found");

                    // let y be the first such member of succ(x)
                    // M1[h(y)] := 1
                    M1.insert(y);
                    // push y into S1
                    S1.push(y);
                } else {
                    // tracing::debug!(?x, "not found");

                    // M2.clear();

                    // pop x from S1
                    assert_eq!(Some(x), S1.pop());

                    // if x ∈ F
                    if product_buchi.is_accepting_state(x) {
                        // tracing::debug!(?x, "accepting");

                        // push x into S2
                        S2.push(x);

                        // while S2 ≠ ∅
                        while let Some(&v) = S2.last() {
                            // tracing::debug!(?S1, ?S2, ?M1, ?M2, "inner");

                            // v := top(S2)

                            // tracing::debug!(?v, ?x, succ=?product_buchi.adj(v), contained=?product_buchi.adj(v).contains_key(x));
                            // if x ∈ succ(v)
                            if product_buchi.adj_ids(v).contains(&x) {
                                // tracing::debug!(?S1, ?S2, "found!");
                                return Some(Self { S1, S2 });
                            }

                            // if M2[h(w)] = 1 for all w ∈ succ(v)
                            match product_buchi.adj_ids(v).find(|w| !M2.contains(*w)) {
                                None => {
                                    // pop v from S2
                                    assert_eq!(Some(v), S2.pop());
                                }
                                Some(w) => {
                                    // let w be the first member of succ(v) with M2[h(w)] = 0
                                    // M2[h(w)] := 1
                                    M2.insert(w);
                                    // push w into S2
                                    S2.push(w);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

impl<S: State, T: State, AP: AtomicProperty> ProductBuchi<'_, '_, S, T, AP> {
    /// Find a cycle containing an accepting state if it exists.
    ///
    /// Implementation of Algorithm B from ["Memory-Efficient Algorithms for the
    /// Verification of Temporal Properties" by M. Vardi, P. Wolper, M.
    /// Yannakakis](https://link.springer.com/content/pdf/10.1007/bf00121128.pdf)
    pub fn find_accepting_cycle(&self) -> Option<ProductAcceptingCycle<S, T, AP>> {
        // for i in 0..25 {
        //     let before = std::time::Instant::now();
        //     print!("run: {i}");
        //     ProductAcceptingCycle::find(self);
        //     println!(" took {:?}", before.elapsed());
        // }
        ProductAcceptingCycle::find(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct ProductAcceptingCycle<S, T, AP: AtomicProperty> {
    S1: Vec<ProductBuchiNodeId<S, T, AP>>,
    S2: Vec<ProductBuchiNodeId<S, T, AP>>,
}

impl<S: State, T: State, AP: AtomicProperty> ProductAcceptingCycle<S, T, AP> {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.S1.len() + self.S2.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = ProductBuchiNodeId<S, T, AP>> + '_ {
        self.S1.iter().chain(self.S2.iter()).copied()
    }
    /// Implementation of Algorithm B from ["Memory-Efficient Algorithms for the
    /// Verification of Temporal Properties" by M. Vardi, P. Wolper, M.
    /// Yannakakis](https://link.springer.com/content/pdf/10.1007/bf00121128.pdf)
    fn find(product_buchi: &ProductBuchi<S, T, AP>) -> Option<Self> {
        Self::find_impl(product_buchi)
    }
    #[inline(never)]
    #[allow(non_snake_case)]
    fn find_impl(product_buchi: &ProductBuchi<S, T, AP>) -> Option<Self> {
        let mut S1 = Vec::with_capacity(1024);
        let mut S2 = Vec::with_capacity(1024);

        let mut M1: ProductBuchiNodeSet<S, T, AP> = Default::default();
        let mut M2: ProductBuchiNodeSet<S, T, AP> = Default::default();

        for s0 in product_buchi.init_states() {
            // S1 := {s0}
            S1.clear();
            S1.push(s0);
            // S2 := ∅
            S2.clear();

            // M1 := M2 := 0
            M1.clear();
            M2.clear();

            // while S1 ≠ ∅
            while let Some(&x) = S1.last() {
                // tracing::debug!(?S1, ?S2, ?M1, ?M2, "outer");

                // x := top(S1)

                // tracing::debug!(adj=?product_buchi.adj(x), ids=?product_buchi.adj_ids(x).collect_vec(), found=?product_buchi.adj_ids(x).find(|y|
                //     {let res = !M1.contains(*y);
                //     // tracing::debug!(?y, set=?M1, found=?res);
                //     res}
                // ));
                // if there is a y in succ(x) with M1[h(y)] = 0
                if let Some(y) = product_buchi.adj_ids(x).find(|y| !M1.contains(*y)) {
                    // tracing::debug!(?x, ?y, "found");

                    // let y be the first such member of succ(x)
                    // M1[h(y)] := 1
                    M1.insert(y);
                    // push y into S1
                    S1.push(y);
                } else {
                    // tracing::debug!(?x, "not found");

                    // M2.clear();

                    // pop x from S1
                    // assert_eq!(Some(x), S1.pop());
                    S1.pop();

                    // if x ∈ F
                    if product_buchi.accepting_states().any(|y| y == x) {
                        // tracing::debug!(?x, "accepting");

                        // push x into S2
                        S2.push(x);

                        // while S2 ≠ ∅
                        while let Some(&v) = S2.last() {
                            // tracing::debug!(?S1, ?S2, ?M1, ?M2, "inner");

                            // v := top(S2)

                            // tracing::debug!(?v, ?x, succ=?product_buchi.adj(v), contained=?product_buchi.adj(v).contains_key(x));
                            // if x ∈ succ(v)
                            if product_buchi.adj_ids(v).any(|y| y == x) {
                                // tracing::debug!(?S1, ?S2, "found!");
                                return Some(Self { S1, S2 });
                            }

                            // if M2[h(w)] = 1 for all w ∈ succ(v)
                            match product_buchi.adj_ids(v).find(|w| !M2.contains(*w)) {
                                None => {
                                    // pop v from S2
                                    // assert_eq!(Some(v), S2.pop());
                                    S2.pop();
                                }
                                Some(w) => {
                                    // let w be the first member of succ(v) with M2[h(w)] = 0
                                    // M2[h(w)] := 1
                                    M2.insert(w);
                                    // push w into S2
                                    S2.push(w);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::buchi;

    use crate::ltl::expression::Literal;

    #[test]
    fn it_should_found_non_empty() {
        let buchi: Buchi<String, Literal> = buchi! {
            q0
                [a] => q1
            q1
                [b] => q2
            q2
                [e] => q3
                [c] => q4 // cycle containing an accepting state
            q3
                [f] => q1
            q4
                [d] => q3
            ===
            init = [q0]
            accepting = [q1]
        };

        insta::assert_snapshot!(buchi.display(), @r###"
        States:
         "q0" []
           =[a]=> "q1"
         "q1" []
           =[b]=> "q2"
         "q2" []
           =[e]=> "q3"
           =[c]=> "q4"
         "q3" []
           =[f]=> "q1"
         "q4" []
           =[d]=> "q3"
        Initial: "q0"
        Accept:  ["q1"]
        "###);

        let res = AcceptingCycle::find(&buchi);

        let cycle = res.unwrap();

        assert_eq!(4, cycle.len());
    }

    #[test]
    fn it_should_found_empty_because_the_cycle_doesnt_contain_an_accepting_state() {
        let buchi: Buchi<String, Literal> = buchi! {
            q0
                [a] => q1
            q1
                [b] => q2
            q2
                [e] => q3
                [c] => q4
            q3
            q4
                [d] => q3
            ===
            init = [q0]
            accepting = [q1]
        };

        insta::assert_snapshot!(buchi.display(), @r###"
        States:
         "q0" []
           =[a]=> "q1"
         "q1" []
           =[b]=> "q2"
         "q2" []
           =[e]=> "q3"
           =[c]=> "q4"
         "q3" []
         "q4" []
           =[d]=> "q3"
        Initial: "q0"
        Accept:  ["q1"]
        "###);

        let res = AcceptingCycle::find(&buchi);

        assert_eq!(res, None);
    }

    #[test]
    fn it_should_found_emptiness() {
        let buchi: Buchi<String, Literal> = buchi! {
            q0
                [a] => q1
            q1
            ===
            init = [q0]
            accepting = [q0, q1]
        };

        insta::assert_snapshot!(buchi.display(), @r###"
        States:
         "q0" []
           =[a]=> "q1"
         "q1" []
        Initial: "q0"
        Accept:  ["q0", "q1"]
        "###);

        let res = AcceptingCycle::find(&buchi);

        assert_eq!(res, None);
    }
}
