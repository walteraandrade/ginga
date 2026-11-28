# Motion as text

`Trace::of(&motion, from, step)` reads a motion at regular times until it settles. Printing it gives one row per step:

```text
ms           value
0            0.000
50          25.000
100         50.000
150         75.000
200        100.000  settled
```

The format is stable: put it in a test and a change in motion shows up as a diff.

A `LookMotion` ([A look on an element](../gpui/look.md)) is written down the same way, with a column for each value its look names. `Trace::of` takes either: both are `Traceable`, and nothing else is.

```text
ms               x           y     opacity
0            0.000       0.000       0.000
50          20.000      10.000       0.500
100         40.000      20.000       1.000  settled
```
