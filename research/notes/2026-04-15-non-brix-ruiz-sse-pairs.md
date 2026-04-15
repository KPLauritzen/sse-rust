# Literature Note: Non-Brix-Ruiz SSE Pairs

Goal: collect explicit strong shift equivalence (SSE) positive examples outside
the current `k=3` / `k=4` Brix-Ruiz family, without trying to force them into
the current solver surface.

I split the examples into two buckets:

- direct matrix pairs or families explicitly displayed in the cited source;
- matrix pairs inferred from a displayed graph move together with the standard
  out-split / in-split matrix theorem.

## Directly displayed matrix examples

### 1. Riedel/Baker large-lag `2x2` family

Source:
- [references/boyle-schmieding-2023-2006.01051/source.tex](../../references/boyle-schmieding-2023-2006.01051/source.tex)
  around Example `riedelexample`
- arXiv: <https://arxiv.org/abs/2006.01051>

Displayed family:

```text
A_k = [[k,   2],
       [1,   k]]

B_k = [[k-1, 1],
       [1,   k+1]]
```

Facts recorded in the source:

- for every positive integer `k`, `A_k` and `B_k` are SSE over `Z_+`;
- the minimum `Z_+` lag goes to infinity with `k`.

Why this is useful:

- this is an infinite hard-positive `2x2` family;
- unlike the Brix-Ruiz family, the positive result is uniform in `k`;
- it is a good place to tune lag-sensitive heuristics.

Concrete positive instances:

```text
k = 2:
[[2, 2], [1, 2]]   <->   [[1, 1], [1, 3]]

k = 3:
[[3, 2], [1, 3]]   <->   [[2, 1], [1, 4]]
```

### 2. Lind-Marcus Example `7.2.2` transitivity chain

Source:
- [references/bilich-dor-on-ruiz-2024-2411.05598/source.tex](../../references/bilich-dor-on-ruiz-2024-2411.05598/source.tex)
  lines around the example citing Lind-Marcus `7.2.2`
- arXiv: <https://arxiv.org/abs/2411.05598>

Displayed matrices:

```text
A = [[1, 1, 0],
     [0, 0, 1],
     [1, 1, 1]]

B = [[1, 1],
     [1, 1]]

C = [[2]]
```

Facts recorded in the source:

- `A` is elementary strong shift related to `B`;
- `B` is elementary strong shift related to `C`;
- `A` is not elementary strong shift related to `C`.

Consequences:

- `A <-> C` is a concrete lag-`2` SSE positive pair;
- it is a useful tiny example where SSE holds but ESSE does not.

Local reconstruction of one valid ESSE chain:

```text
A = R1 S1,  B = S1 R1

R1 = [[0, 1],
      [1, 0],
      [1, 1]]

S1 = [[0, 0, 1],
      [1, 1, 0]]

B = R2 S2,  C = S2 R2

R2 = [[1],
      [1]]

S2 = [[1, 1]]
```

The endpoint matrices are source-backed; the explicit factors above are a local
reconstruction, not quoted from the paper.

### 3. Full `2`-shift higher-block family

Source:
- [references/boyle-schmieding-2023-2006.01051/source.tex](../../references/boyle-schmieding-2023-2006.01051/source.tex)
  Remark `higherblockedgesfts`
- arXiv: <https://arxiv.org/abs/2006.01051>

Displayed matrices:

```text
A^[1] = [[2]]

A^[2] = [[1, 1],
         [1, 1]]

A^[3] = [[1, 1, 0, 0],
         [0, 0, 1, 1],
         [1, 1, 0, 0],
         [0, 0, 1, 1]]
```

Facts recorded in the source:

- these are higher-block presentations of the same edge shift;
- hence they are conjugate and therefore SSE.

Useful endpoint pairs:

```text
[[2]]  <->  [[1, 1], [1, 1]]

[[2]]  <->  [[1, 1, 0, 0],
             [0, 0, 1, 1],
             [1, 1, 0, 0],
             [0, 0, 1, 1]]

[[1, 1], [1, 1]]  <->  the same 4x4 matrix above
```

Why this is useful:

- gives explicit `1x1`, `2x2`, and `4x4` positives in one family;
- repeated rows and columns make it a good testbed for quotient-style split
  heuristics.

## Graph examples turned into matrix pairs

These are not written as adjacency matrices in the paper. The graphs are
displayed explicitly, and the matrices below are the corresponding adjacency
matrices in a chosen vertex order. The SSE claim then follows from the standard
out-split / in-split matrix theorem cited in the same paper.

### 4. Golden mean out-split

Source:
- [references/brix-2022-1912.05212/source.tex](../../references/brix-2022-1912.05212/source.tex)
  Example `ex:golden-mean`
- arXiv: <https://arxiv.org/abs/1912.05212>

Base graph adjacency:

```text
G = [[1, 1],
     [1, 0]]
```

Inferred out-split adjacency, with vertex order `[v1, w, v2]`:

```text
G_out = [[1, 0, 1],
         [1, 0, 1],
         [0, 1, 0]]
```

One ESSE witness:

```text
D = [[1, 0, 1],
     [0, 1, 0]]

E = [[1, 0],
     [1, 0],
     [0, 1]]

G     = D E
G_out = E D
```

Why this is useful:

- a clean `2x2 -> 3x3` one-step positive;
- the target has duplicate rows, so it is a good split-detection benchmark.

### 5. Golden mean in-split without empty partition

Source:
- same Brix example sequence as above, using the first in-split graph
- theorem `thm:out-in-split` in the same source

Base graph adjacency:

```text
G = [[1, 1],
     [1, 0]]
```

Inferred in-split adjacency, with vertex order `[v1, w, v2]`:

```text
G_in = [[1, 1, 0],
        [0, 0, 1],
        [1, 1, 0]]
```

One ESSE witness:

```text
D^t = [[1, 0],
       [0, 1],
       [1, 0]]

E = [[1, 1, 0],
     [0, 0, 1]]

G    = E D^t
G_in = D^t E
```

Why this is useful:

- another `2x2 -> 3x3` one-step positive, but with duplicate columns rather
  than the exact out-split pattern.

### 6. Golden mean in-split with an empty partition set

Source:
- same Brix in-split example, using the right-hand graph with a source
- theorem `thm:out-in-split` in the same source

Base graph adjacency:

```text
G = [[1, 1],
     [1, 0]]
```

Inferred in-split adjacency, with vertex order `[v1, w, v2]`:

```text
G_src = [[1, 1, 0],
         [1, 0, 0],
         [1, 1, 0]]
```

One ESSE witness:

```text
D^t = [[1, 0],
       [0, 1],
       [1, 0]]

E = [[1, 1, 0],
     [1, 0, 0]]

G     = E D^t
G_src = D^t E
```

Why this is useful:

- explicit positive with a source-producing split;
- useful if we want examples that deliberately leave the essential class.

## Parametric source-backed templates

### 7. Generic elementary row splitting template

Source:
- [references/boyle-kim-roush-2013-1209.5096/source.tex](../../references/boyle-kim-roush-2013-1209.5096/source.tex)
  around the row-splitting example
- arXiv: <https://arxiv.org/abs/1209.5096>

Displayed template:

```text
A = [[a, b],
     [c, d]]
```

with decompositions

```text
a1 + a2 + a3 = a
b1 + b2 + b3 = b
c1 + c2      = c
d1 + d2      = d
```

produces a one-step ESSE to

```text
C = [[a1, a1, a1, b1, b1],
     [a2, a2, a2, b2, b2],
     [a3, a3, a3, b3, b3],
     [c1, c1, c1, d1, d1],
     [c2, c2, c2, d2, d2]]
```

over any semiring where the chosen entries live, in particular over `Z_+` when
all pieces are nonnegative integers.

Why this is useful:

- this is a literature-backed positive generator, not just one example;
- it gives a controllable way to manufacture `2x2 -> 5x5` one-step positives
  with tunable row-duplication structure.

## Shortlist For Search Tuning

If the goal is heuristic tuning rather than theorem collection, the most useful
targets from this note are:

1. the Riedel family `A_k <-> B_k` for lag-hard `2x2` positives;
2. the Lind-Marcus `A <-> C` lag-`2` non-ESSE positive;
3. the full `2`-shift higher-block family for clean size growth;
4. the golden mean split examples for exact split/amalgamation move recovery.
