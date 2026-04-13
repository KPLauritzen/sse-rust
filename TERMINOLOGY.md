# Terminology

Ubiquitous language for this project, grounded in the literature.
When our code uses a different name, the mapping is noted.

---

## Core equivalence relations

### Elementary strong shift equivalence (ESSE)

Two square nonnegative integer matrices **A** and **B** are *elementary strong
shift equivalent* if there exist rectangular nonnegative integer matrices **U**
(n x m) and **V** (m x n) such that

    A = UV,    B = VU.

The pair (U, V) is called a *factorisation* of the elementary step.
The intermediate dimension m may differ from n: when m > n the step
"expands", when m < n it "contracts", and when m = n it is
"dimension-preserving".

**Source:** Williams (1973); Lind & Marcus (2021, Ch. 7).

### Strong shift equivalence (SSE)

A and B are *strong shift equivalent* if there is a finite chain of elementary
strong shift equivalences connecting them:

    A = A_0 ~_e A_1 ~_e ... ~_e A_l = B.

The number of elementary steps l is the *lag*.

**Source:** Williams (1973); Lind & Marcus (2021, Definition 7.2.1).

### Shift equivalence (SE)

A and B are *shift equivalent* with lag m >= 1 if there exist nonnegative
integer matrices R and S such that

    A^m = RS,   B^m = SR,   AR = RB,   SA = BS.

SE is decidable (Kim & Roush). SE is necessary but not sufficient for SSE
(Kim & Roush 1999 counterexamples).

**Source:** Williams (1973); Kim & Roush (1999); Lind & Marcus (2021,
Definition 7.3.1).

---

## Moves: the building blocks of an SSE path

An SSE path is a chain of elementary steps. Each elementary step is a
*factorisation* A = UV, B = VU. The literature describes several structured
families of factorisations. These families are not disjoint -- they are
increasingly specialised subsets of "all factorisations".

### Factorisation (generic)

Any decomposition A = UV with U (n x m), V (m x n) over the nonneg integers.
This is the most general form of an elementary SSE step.

The codebase calls these by their shape:
- `square_factorisation_2x2` -- m = n = 2
- `rectangular_factorisation_2x3` -- n = 2, m = 3
- `rectangular_factorisation_3x3_to_2` -- n = 3, m = 2
- `square_factorisation_3x3` -- m = n = 3

### Conjugation (dimension-preserving similarity)

A special case of square factorisation where U = P is invertible over Z_+
(not merely nonneg-invertible; typically P is an *elementary matrix*
P = I +/- k * e_i * e_j^T) and V = P^{-1} A, giving B = P^{-1} A P.
Conjugation is dimension-preserving (m = n).

This is not a standard term in the SSE literature; it is our computational
shorthand for structured same-dimension moves. The literature would simply
call this a dimension-preserving elementary SSE step with a special factor
shape.

The codebase further subdivides 3x3 conjugations into:
- `elementary_conjugation_3x3` -- P = I +/- k * e_i * e_j^T
- `opposite_shear_conjugation_3x3` -- product of two shears with opposite signs
- `parallel_shear_conjugation_3x3` -- product of two shears sharing a pivot
- `convergent_shear_conjugation_3x3` -- product of two convergent shears

### State splitting (out-split / in-split)

A *state splitting* is a structured factorisation A = DE where:
- **D** is a {0,1}-valued *division matrix* (n x m) encoding a partition of
  states, and
- **E** is the *edge matrix* (m x n).

The result C = ED is the adjacency matrix of the split graph.

There are two dual kinds:

**Out-split (Move (O)):** Partition the outgoing edges of a vertex v into
groups. Each group becomes a new vertex. The division matrix D has exactly
one 1 per column, encoding which child each parent maps to. The dimension
increases by (number of children - 1).

At the matrix level: partition the *columns* of a row of A among the
children. D is n x (n+1), E is (n+1) x n.

**In-split (Move (I-)):** Partition the incoming edges of a vertex v into
groups. This is the transpose dual: perform an out-split on A^T, then
transpose back.

At the matrix level: partition the *rows* of a column of A among the
children.

**Key point:** Every out-split/in-split *is* a factorisation (a special one
where one factor is {0,1}-valued), but not every factorisation is a state
splitting.

**Source:** Williams (1973); Lind & Marcus (2021, Section 2.4); Brix (2022,
Definitions 4.1 and 4.3).

### Amalgamation (out-amalgamation / in-amalgamation)

The reverse of a state splitting. Merge two vertices whose adjacency
structure is compatible.

**Out-amalgamation:** Merge two vertices with identical columns in the
adjacency matrix ("same past" -- they receive edges from the same sources
with the same multiplicities). This is the reverse of an out-split.
Dimension decreases by 1.

**In-amalgamation:** Merge two vertices with identical rows ("same future" --
they emit edges to the same targets with the same multiplicities). This is
the reverse of an in-split.

An amalgamation is also an elementary SSE step: if C = ED was produced by a
split, then the amalgamation gives A = DE (same factorisation, read in the
other direction).

**Source:** Lind & Marcus (2021, Sections 2.4 and 7.2).

### Balanced in-split (Move (I+))

A pair of in-splits of the *same graph* at the *same vertex* using the *same
number of partition elements*, producing two graphs that are eventually
conjugate. This is Brix's variation of Williams's classical in-split, used to
characterise one-sided eventual conjugacy.

This is *not* a single elementary SSE step. It is a pair of moves producing
two related graphs.

**Source:** Eilers & Ruiz (2019, Definition 3.6); Brix (2022, Definition 4.6).

---

## Relationship between graph moves and factorisations

```
all factorisations A = UV, B = VU
  |
  +-- dimension-changing factorisations (m != n)
  |     |
  |     +-- state splittings: one factor is a {0,1} division matrix
  |     |     +-- out-split (partition outgoing edges)
  |     |     +-- in-split (partition incoming edges)
  |     |
  |     +-- generic rectangular factorisations
  |
  +-- dimension-preserving factorisations (m = n)
        |
        +-- conjugations: U invertible, V = U^{-1}A
        |
        +-- generic square factorisations
```

The codebase's `SearchMode` reflects this:
- **`Mixed`** uses both graph moves (splits/amalgamations) and generic
  factorisations (including conjugations).
- **`GraphOnly`** restricts to state splittings and amalgamations only.

---

## Intermediary equivalence relations (between SE and SSE)

Carlsen, Dor-On & Eilers (2024) introduce three relations that are
intermediate between SE and SSE. For finite essential matrices, all three
collapse to SSE.

### Compatible shift equivalence (CSE)

SE of lag m via matrices R, S, together with *path isomorphisms*
phi_R, phi_S, psi_A, psi_B satisfying a compatibility condition
(Definition 4.1 in Carlsen-Dor-On-Eilers).

SSE implies CSE. For finite essential matrices, CSE = SSE.

### Representable shift equivalence (RSE)

SE where the shift equivalence can be represented as bounded operators on
Hilbert space (Definition 5.1).

CSE implies RSE.

### Strong Morita shift equivalence (SMSE)

SE where the associated Pimsner dilations are strongly Morita equivalent
(Definition 6.1).

RSE implies SMSE.

**Source:** Carlsen, Dor-On & Eilers (2024, Theorem 1.3): for finite
essential matrices, SSE = CSE = RSE = SMSE.

---

## Balanced and aligned equivalences

### Balanced elementary equivalence

A and B are *balanced elementary equivalent* if there exist rectangular
nonneg matrices R_A, S, R_B such that

    A = S R_A,   B = S R_B,   R_A S = R_B S.

Note the asymmetry versus ESSE (A = UV, B = VU): here the left factor S is
shared but the right factors R_A, R_B differ.

**Balanced strong shift equivalence** is the transitive closure.

**Source:** Brix (2022, p. 3).

### Aligned module shift equivalence

A graph/module-level witness involving fiberwise bijections
sigma_g, sigma_h, omega_e, omega_f satisfying associator relations.

This is the relation currently implemented in `src/aligned.rs` for the
module-level search. At the module level it is *not known* whether aligned
module shift equivalence implies SSE (Remark 5.5 in Brix-Dor-On-Hazrat-Ruiz
2025). However, at the matrix level the question is settled: see "Concrete
shift" below.

**Source:** Brix, Dor-On, Hazrat & Ruiz (2025, Definitions 5.1 and 5.2).

### Concrete shift / aligned, balanced, compatible shift equivalence (matrix-level)

A *concrete shift* between matrices A and B with lag m is a tuple
(R, S, varphi_R, varphi_S, psi_A, psi_B) where R and S are the SE
matrices and the four maps are *path isomorphisms*:

    varphi_R : E_A x E_R -> E_R x E_B
    varphi_S : E_B x E_S -> E_S x E_A
    psi_A    : E_R x E_S -> E_A^m
    psi_B    : E_S x E_R -> E_B^m

A concrete shift is called:

- **Aligned** if varphi and psi satisfy two "associator" equations
  (one step of varphi_R then varphi_S reassembles via psi_A).
- **Balanced** if the iterated maps varphi^(m) decompose via psi^{-1}
  x psi.
- **Compatible** if varphi^(m) decomposes via psi and psi^{-1} (the
  Carlsen-Dor-On-Eilers formulation).

For essential matrices over N, the implication chain is:
compatible => aligned => balanced, and all three coincide with SSE.

**Source:** Bilich, Dor-On & Ruiz (2024, arXiv:2411.05598, Definition 3.3
and Corollary following Theorem 3.11). The paper is in
`references/bilich-dor-on-ruiz-2024-2411.05598/`.

The codebase's `ConcreteShiftRelation2x2` enum (`Aligned`, `Balanced`,
`Compatible`) refers to fixed-lag matrix-level witness verification for
these three relations, specialised to 2x2 matrices.

---

## Graph-theory terms

### Directed graph

A quadruple G = (V, E, r, s) where V is the vertex set, E the edge set,
and r, s : E -> V are the *range* and *source* maps.

Convention in this project (following Lind & Marcus): an edge e goes *from*
s(e) *to* r(e), so the adjacency matrix entry A_{vw} counts edges with
source v and range w.

**Source:** Lind & Marcus (2021, Section 2.2); Brix (2022, Section 2).

### Essential matrix

A square nonneg integer matrix with no zero rows and no zero columns.
Equivalently, its directed graph has no sources and no sinks.

**Source:** Carlsen, Dor-On & Eilers (2024, p. 5).

### Irreducible matrix

A square nonneg integer matrix A such that for every pair of indices (i,j)
there exists k >= 1 with (A^k)_{ij} > 0. Equivalently, the directed graph is
strongly connected.

### Higher block graph / higher power

The N'th *higher block graph* E^[N] has edge set E^N (paths of length N) and
vertex set E^{N-1}. There is a canonical conjugacy from the edge shift of E
to that of E^[N].

**Source:** Lind & Marcus (2021, Section 1.4); Brix (2022, p. 4).

### Canonical form

Not a literature term. In this codebase, the *canonical form* of a matrix is
a representative of its equivalence class under simultaneous row and column
permutation (permutation similarity). Used to dedup the BFS frontier.

---

## Search-engine terms (codebase-specific)

These terms are not from the literature; they describe our solver's
architecture.

- **Frontier:** The set of matrices at the current BFS layer awaiting
  expansion.
- **Bidirectional BFS:** Search expanding from both A and B toward the middle.
- **Move family:** A named class of elementary SSE steps (e.g. `outsplit`,
  `rectangular_factorisation_2x3`, `elementary_conjugation_3x3`). Used for
  telemetry and selective search.
- **Spectral pruning:** Filtering candidate matrices by trace/determinant
  consistency before full canonicalization.
- **Sidecar search:** An auxiliary search substrate (balanced, conjugacy,
  aligned) that runs alongside or as a fallback to the main BFS.

### Path metrics

Avoid bare **path length** in project docs and harness output. It is too easy
to confuse four different counts.

- **Lag / witness lag / witness step count:** the number of elementary SSE
  steps in a validated witness. For a witness
  `A = A_0 ~_e A_1 ~_e ... ~_e A_l = B`, the lag is `l`.
- **Path matrix count:** the number of matrices listed in a witness path. This
  is always `lag + 1` for a validated witness.
- **Guide matrix sequence:** an ordered list of matrices used to seed guided
  search. Consecutive guide matrices are waypoints; they do not by themselves
  assert that each hop is already a direct elementary witness.
- **Guide step count:** the number of transitions between consecutive guide
  matrices in a stored guide matrix sequence. This is `guide_matrix_count - 1`.

The practical consequence is important for seeded fixtures. A label such as
`endpoint_16_path` refers to a guide with `17` listed matrices and `16` guide
transitions. If the harness reconstructs direct witness segments between those
waypoints, the resulting witness lag may be larger or smaller than `16`.

---

## Quick reference: "is X the same as Y?"

| Question | Answer |
|----------|--------|
| Is an out-split a factorisation? | Yes. It is a *special* factorisation where one factor is a {0,1} division matrix. |
| Is every factorisation a graph move? | No. Generic factorisations (e.g. square factorisations, conjugations) do not correspond to splits or amalgamations. |
| Is an in-split just an out-split on the transpose? | Yes. That is how both the literature and our code define it. |
| Is an amalgamation the reverse of a split? | Yes. Out-amalgamation reverses out-split; in-amalgamation reverses in-split. |
| Is a conjugation a graph move? | No. It is a dimension-preserving factorisation, not a split or amalgamation. |
| Are SE and SSE the same? | No. SE is necessary but not sufficient for SSE (Kim & Roush 1999). |
| Is balanced elementary equivalence the same as ESSE? | No. ESSE: A = UV, B = VU. Balanced: A = SR_A, B = SR_B, R_A S = R_B S. Different shape. |
| Is "graph-only mode" the same as "only splits"? | Not quite. It uses splits *and* amalgamations, which are both graph moves. |
| Does aligned module SE imply SSE? | At the module level, open (Brix-Dor-On-Hazrat-Ruiz 2025 Remark 5.5). At the matrix level, yes: aligned = balanced = compatible = SSE for essential matrices (Bilich-Dor-On-Ruiz 2024). |

---

## Primary references

- Williams, R.F. (1973). Classification of subshifts of finite type.
- Lind, D. & Marcus, B. (2021). *An Introduction to Symbolic Dynamics and
  Coding* (2nd ed.). Cambridge. -- **The standard textbook.**
- Kim, K.H. & Roush, F.W. (1999). Williams's conjecture is false for
  irreducible subshifts.
- Brix, K.A. (2022). Balanced strong shift equivalence, balanced in-splits,
  and eventual conjugacy.
- Carlsen, T.M., Dor-On, A. & Eilers, S. (2024). Shift equivalences through
  the lens of Cuntz-Krieger algebras.
- Bilich, B., Dor-On, A. & Ruiz, E. (2024). Shift equivalence relations
  through the lens of C*-correspondences. arXiv:2411.05598.
- Brix, K.A., Dor-On, A., Hazrat, R. & Ruiz, E. (2025). Unital aligned
  shift equivalence and the graded classification conjecture.
