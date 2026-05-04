(declare-fun min (Int Int) Int)
(assert (forall ((x Int) (y Int))
    (= (min x y) (ite (<= x y) x y))))
