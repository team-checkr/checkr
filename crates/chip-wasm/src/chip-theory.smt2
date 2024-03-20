
;(set-logic NIA)
;(set-option :timeout 5000)

(declare-fun min (Int Int) Int)
(declare-fun max (Int Int) Int)
(declare-fun fac (Int) Int)
(declare-fun fib (Int) Int)
(declare-fun exp (Int Int) Int)

(assert (forall ((x Int) (y Int)) 
    (=> (< x y) (= x (min x y)))
))

(assert (forall ((x Int) (y Int)) 
    (=> (>= x y) (= y (min x y)))
))

(assert (forall ((x Int) (y Int)) 
    (=> (< x y) (= y (max x y)))
))

(assert (forall ((x Int) (y Int)) 
    (=> (>= x y) (= x (max x y)))
))

(assert
    (= 1 (fac 0))
)

(assert (forall ((n Int)) 
    (=> (> n 0) (= (fac n) (* n (fac (- n 1)))))
))

(assert
    (= 0 (fib 0))
)

(assert
    (= 1 (fib 1))
)

(assert (forall ((n Int)) 
    (=> (> n 1) (= (fib n) (+ (fib (- n 1)) (fib (- n 2)))))
))

(assert (forall ((x Int)) 
    (= 1 (exp x 0))
))

(assert (forall ((x Int) (n Int)) 
    (=> (> n 0) (= (exp x n) (* x (exp x (- n 1)))))
))

