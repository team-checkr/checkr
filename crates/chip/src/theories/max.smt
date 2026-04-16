(declare-fun max (Int Int) Int)
(assert (forall ((x Int) (y Int))
    (= (max x y) (ite (<= x y) y x))))
