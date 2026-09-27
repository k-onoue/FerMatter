#set text(size: 12pt)
#let prediction = $hat(y)$

== Fitting observations

For a synthetic observation $y_i$, use the estimate $prediction_i$.

$ hat(y)_i = beta_0 + beta_1 t_i $ <eq:linear-model>

$ cal(L)(beta) = sum_(i=1)^n (y_i - hat(y)_i)^2 $ <eq:least-squares>

$ hat(beta) = (X^T X)^(-1) X^T y $ <eq:normal-equation>

$ "RMSE" = sqrt(1/n sum_(i=1)^n (y_i - hat(y)_i)^2) $ <eq:rmse>
