#set document(title: "A Small Study in Motion", author: "Mathlidelux Demo")
#set page(paper: "a4", margin: 24mm)
#set text(size: 11pt)
#set math.equation(numbering: "(1)")
#import "symbols.typ": RR, norm

= A Small Study in Motion

This fictional teaching note explores a smooth trajectory, a least-squares fit,
and a few geometric relationships. It contains no research data.

The position $x(t) in RR^2$ changes with time $t >= 0$.

$ x(t) = vec(cos t, sin t) $ <eq:trajectory>

$ norm(x(t)) = 1 $ <eq:unit-circle>

#include "sections/model.typ"
#include "sections/fitting.typ"

== A geometric view

$ A = pi r^2 $ <eq:area>

$ R(theta) = mat(cos theta, -sin theta; sin theta, cos theta) $ <eq:rotation>

$ f(x) = cases(x^2 & "if " x >= 0, -x & "otherwise") $ <eq:piecewise>

For a unit circle, $r = 1$ and $A = pi$.
