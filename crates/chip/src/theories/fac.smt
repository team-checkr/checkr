(declare-fun fac (Int) Int)
(assert (= 1 (fac 0)))
(assert (forall ((n Int))
    (=> (> n 0) (= (fac n) (* n (fac (- n 1)))))))
