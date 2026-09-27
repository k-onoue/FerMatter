#import "@preview/cetz:0.5.2"
#import "@preview/cetz:0.5.2": canvas as sketch
#set page(paper: "a4", margin: 25mm)
#set text(size: 11pt)
*A Few Vector Sketches*

Mathlidelux Demo

= A triangle
A synthetic drawing with three labeled vertices.

#cetz.canvas({
  import cetz.draw: *
  line((0, 0), (4, 0), (2, 2.6), close: true, stroke: rgb("3478d4") + 1.2pt, fill: rgb("edf4ff"))
  content((0, -0.4), $A$)
  content((4, -0.4), $B$)
  content((2, 3), $C$)
}) <triangle>

The area is $S = b h / 2$.

= A workflow
#figure(
  sketch({
    import cetz.draw: *
    rect((0, 0), (2.4, 1), stroke: rgb("3478d4"), fill: rgb("edf4ff"))
    rect((3.6, 0), (6, 1), stroke: rgb("3478d4"), fill: rgb("edf4ff"))
    rect((7.2, 0), (9.6, 1), stroke: rgb("3478d4"), fill: rgb("edf4ff"))
    line((2.4, 0.5), (3.6, 0.5), stroke: rgb("3478d4") + 1.2pt)
    line((6, 0.5), (7.2, 0.5), stroke: rgb("3478d4") + 1.2pt)
    content((1.2, 0.5), [Source])
    content((4.8, 0.5), [SVG])
    content((8.4, 0.5), [Slides])
  }),
  caption: [A fictional workflow used for demonstration.],
) <workflow>
