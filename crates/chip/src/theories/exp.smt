(declare-fun exp (Int Int) Int)
(assert (forall ((x Int)) (= 1 (exp x 0))))
(assert (forall ((x Int) (n Int))
    (=> (> n 0) (= (exp x n) (* x (exp x (- n 1)))))))
