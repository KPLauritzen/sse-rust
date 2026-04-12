# Baker `k=3` factor-shape note

## Question

Why does the current solver still miss the short Lind-Marcus/Baker path for
`brix_ruiz_k3`, even though it can now compress the blind 16-move graph path?

## Evidence

1. Endpoint-guided shortcut search is already materially better than pure
   graph-only search.

   Command:

   ```sh
   cargo run --quiet --features research-tools --bin find_brix_ruiz_path_shortcuts -- \
     --max-shortcut-lag 6 --max-dim 4 --max-entry 5 --min-gap 2 --max-gap 6 \
     --refine-rounds 1 --search-mode mixed
   ```

   Result:

   - best total lag = `11`
   - route shape = `2x2 -> 3x3 -> 3x3 -> 4x4 -> ... -> 3x3 -> 2x2`

   This means the search stack is already capable of finding shorter mixed
   proofs than the 16-move blind graph path without using the hardcoded Baker
   waypoints.

2. The literal Baker steps are still not covered by the main move vocabulary.

   Command:

   ```sh
   cargo run --quiet --features research-tools --bin check_lind_marcus_path
   ```

   Current coverage summary:

   - step 1: covered by `rectangular_factorisation_2x3` and same-past outsplit
   - step 2: missing
   - step 3: covered by `elementary_conjugation`
   - step 4: covered by `elementary_conjugation`
   - step 5: missing
   - step 6: missing
   - step 7: covered by `rectangular_factorisation_3x3_to_2`

3. A broad same-size `4x4` conjugation expansion is not the right target.

   I tested a generic extension of the existing paired-shear `3x3` families to
   `4x4`. It did **not** reach Baker step 5, and it did **not** improve the
   11-step shortcut result. That strongly suggests the missing short move is not
   a hidden same-size conjugation family.

4. Baker step 5 hides a lower-dimensional bridge.

   The published step-5 factors are

   ```text
   U =
   0 1 1 1
   1 0 1 1
   1 0 0 0
   0 1 0 0

   V =
   0 1 0 1
   0 2 1 0
   0 0 1 0
   1 0 0 0
   ```

   with

   ```text
   UV =
   1 2 2 0
   1 1 1 1
   0 1 0 1
   0 2 1 0

   VU =
   1 1 1 1
   3 0 2 2
   1 0 0 0
   0 1 1 1
   ```

   The important structural fact is that `U` is singular and has duplicate
   columns:

   - `det(U) = 0`
   - columns 3 and 4 are equal

   So this step is better understood as a hidden `4x4 -> 3x3 -> 4x4`
   refactorization than as a primitive `4x4 -> 4x4` move.

## Conclusion

The next solver improvement should target the missing `3x4` / `4x3`
rectangular vocabulary, not broader `4x4` same-size conjugation.

Concretely, the most promising families are:

- explicit row-splitting proposals,
- explicit column-splitting proposals,
- diagonal/hidden-rank refactorizations that expose `4x4 -> 3x3 -> 4x4`
  bridges.

## Next step

Implement one narrow structured `3x4` / `4x3` family and use
`check_lind_marcus_path` plus the shortcut-search total lag as the acceptance
criteria. A candidate is successful only if it covers one of Baker steps 2, 5,
or 6 or reduces the current best lag below `11`.

## Follow-up

The first narrow family was a binary-sparse `4x4 -> 3x3` rectangular
enumerator. It succeeded on the local acceptance gate partially:

- Baker step 6 is now covered directly.
- The hidden `3x3` bridge inside Baker step 5 is now recovered directly.
- Baker step 2 is still missing.
- The default shortcut search (`max_shortcut_lag=6`) still bottoms out at
  total lag `11`.

So the next implementation target should be the dual `3x3 -> 4x4` structured
family rather than more work on the `4x4 -> 3x3` side.
