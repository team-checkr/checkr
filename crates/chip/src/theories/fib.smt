(declare-fun fib (Int) Int)
(assert (= 0 (fib 0)))
(assert (= 1 (fib 1)))
(assert (forall ((n Int))
    (=> (> n 1) (= (fib n) (+ (fib (- n 1)) (fib (- n 2)))))))
