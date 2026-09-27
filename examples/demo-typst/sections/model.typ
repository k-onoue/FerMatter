#import "../symbols.typ": norm
#let speed = $v$

== Motion

The velocity is $v(t) = (dif x(t)) / (dif t)$.

$ v(t) = vec(-sin t, cos t) $ <eq:velocity>

$ norm(v(t)) = 1 $ <eq:speed>

$ a(t) = (dif^2 x(t)) / (dif t^2) = -x(t) $ <eq:acceleration>

$ L = integral_0^(2 pi) norm(v(t)) dif t = 2 pi $ <eq:arc-length>