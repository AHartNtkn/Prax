# v39 — Asserted Endpoints Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** the trie learns which nodes are facts, per
`docs/specs/2026-07-15-v39-asserted-endpoints.md` — one bit, one invariant (no unasserted
childless node), queries untouched, both the v32 and v38 requirements satisfied by
construction.

**Architecture:** the whole fix lives in `Prax.Db`'s two mutators plus its serializers;
`Prax.EL`'s algebra extends pointwise; everything else is documentation truth-updates and
pin adjudication. Engine/planner/query modules unchanged.

**Tech Stack:** Haskell (GHC 9.10, cabal), containers, tasty/tasty-hunit.

## Global Constraints

- The invariant is the contract: after any mutation, the trie contains no unasserted
  childless node. Queries (`unifySyms`/`exists`/`childKeys`) are NOT modified — their
  correctness under the invariant is the design.
- Goldens: expected byte-identical, with ONE known-likely adjudication site flagged below
  (the bar bell's phantom-customer count). Any golden movement = BLOCK with the itemized
  diff and the residue-trace; the controller adjudicates bugfix-recapture vs defect.
  Never rationalize, never re-capture unilaterally.
- Tests that EXPECT ghost behavior (DbSpec documents two) are updated to expect the fixed
  behavior — that update IS the observed RED. Zero warnings; hlint; ViewInvariant; the
  usual gates.
- Commit per green task with trailers:
  `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01U9P1EgzYxaLEpEsSQP7Ln5`.

---

### Task 1: The bit, the invariant, and every truth that changes

**Files:**
- Modify: `src/Prax/Db.hs` (representation + mutators + serializers + haddocks),
  `src/Prax/EL.hs` (pointwise extension), `src/Prax/Emotion.hs` (haddock truth-update),
  `test/Prax/DbSpec.hs`, `test/Prax/PersistSpec.hs`, `test/Prax/ELSpec.hs`,
  and IF adjudicated: bar goldens in their own separate commit.

**Design.**

Representation: `data Db = Db !Bool !Bool (IntMap Db)` — exclusion, ASSERTED (strict like
its sibling); `emptyDb = Db False False IntMap.empty`; `dbExcl` unchanged; add
`dbAsserted :: Db -> Bool`. Every 2-field `Db` pattern/construction in Db.hs and EL.hs
extends mechanically (-Wall + the type checker enumerate them; ~33 sites in Db.hs).

`insertToks` — the endpoint marks; pass-through preserves marks; eviction untouched:

```haskell
insertToks :: [(Sym, Maybe Char)] -> Db -> Db
insertToks [] (Db e _ m) = Db e True m           -- the endpoint IS a fact
insertToks ((n, op) : rest) (Db e a m) =
  let i = symId n
      Db _ aOld existing = IntMap.findWithDefault emptyDb i m
      childExcl = op == Just '!'
      cleared = case (op, rest) of
        (Just '!', (nextSym, _) : _) ->
          IntMap.filterWithKey (\k _ -> k == symId nextSym) existing
        _ -> existing
      child' = insertToks rest (Db childExcl aOld cleared)
  in Db e a (IntMap.insert i child' m)
```

(The exclusion flag is overwritten per insert exactly as today — label-faithful semantics
untouched; only assertedness is preserved through traversal.)

`retractNames` — eager pruning falls out of the recursion:

```haskell
retractNames :: [Sym] -> Db -> Db
retractNames [] db = db
retractNames [n] (Db e a m) = Db e a (IntMap.delete (symId n) m)
retractNames (n : ns) (Db e a m) =
  case IntMap.lookup (symId n) m of
    Nothing    -> Db e a m
    Just child ->
      let child' = retractNames ns child
      in if prunable child'
           then Db e a (IntMap.delete (symId n) m)
           else Db e a (IntMap.insert (symId n) child' m)
  where prunable (Db _ asserted cm) = not asserted && IntMap.null cm
```

Serializers — asserted interior nodes emit themselves AND their descendants (round-trip
through `insertAll` reconstructs the marks; no format change):

```haskell
-- dbToSentences: expand gains the asserted-with-children arm
expand (k, child@(Db _ a cm))
  | IntMap.null cm = [name]
  | a              = name : map ((name ++ ".") ++) (go child)
  | otherwise      = map ((name ++ ".") ++) (go child)
-- dbToLabeledSentences: the same arm, with the child-exclusion separator logic
-- it already has (an asserted interior node emits its bare labeled path PLUS
-- the descendant paths; adapt inside the existing comprehension).
```

Haddock truth-updates in the same commit: `Db`'s type haddock (the second Bool);
`retract`'s ambiguity paragraph REWRITTEN (the ambiguity is gone; state the invariant);
`dbToSentences`' phantom-emission caveat and its banked-item pointer DELETED (now false);
`Prax.Emotion.feeling`/`feelingSomeone` haddocks rewritten — the residue trap is retired,
`feelingSomeone` stays recommended FOR ITS PER-TARGET SEMANTICS (−8 per grudge, the v38
reviewer's note), not for safety.

`Prax.EL`: extend `meet`/`leq` (and any other Db-structural ops) pointwise over the new
bit, consistent with the module's stated lattice laws — the implementer reads EL's
haddocks and chooses the extension WITH a stated argument (meet of assertedness is
conjunction or disjunction — whichever preserves the laws; pin the choice in ELSpec both
ways: a law that would fail under the wrong choice).

**DbSpec (the RED is real behavior change — update-then-observe):**
- INSTANCE PERSISTENCE pin: the instance-survival assertion STAYS (`exists instanceFact
  db4 @?= True` — the v32 requirement, now BY MARKING); the two ghost assertions FLIP
  (`exists ".customer.you" @?= False`; `dbToSentences db4 @?= [instanceFact]`) and its
  long comment rewrites to tell the completed story (pruning was wrong, marking is right,
  cite the spec). Run BEFORE the Db change: the flipped assertions FAIL against the ghost
  — that is the observed RED.
- The v38 repro at Db level (new): insert `carol.feels.angry.toward.bob`, retract it,
  `exists "carol.feels.angry"` and a prefix-Match both report absence; `dbToSentences`
  emits nothing.
- Re-asserted scaffold (new): insert the deep path, THEN insert `carol.feels.angry` as
  its own fact, retract the deep leaf — the prefix survives (asserted) and serializes.
- Sibling survival pin: unchanged (still passes — `carol` keeps a child).
- Serialization round-trips marks (new): a db with an asserted-interior-with-children
  fact — `insertAll (dbToLabeledSentences db) emptyDb @?= db` (full Eq, marks included);
  same for the plain-sentence path via a rebuild comparison.
- Eviction pins unchanged.

**PersistSpec:** the round-trip pin gains the asserted-interior case (save/load a state
whose db has one — e.g. a spawned practice instance with transient children — assert full
db equality including marks).

**THE ADJUDICATION FLAG:** the bar bell (`tendBarP`'s "Ring the bell" Subquery counting
`customer.C`) may TODAY count phantom drained customers toward its ≥2 threshold. If the
bar golden or any bar pin moves: BLOCK, trace whether the old behavior counted a ghost
(that is the bug being fixed — report the trace; the controller will direct an itemized
re-capture), and do NOT weaken the bell's threshold. Village/intrigue/feud have no known
residue reads (the v38 sweep) — movement there is a defect until traced otherwise.

- [ ] Update the two ghost pins + write the new pins → observed RED recorded verbatim →
  implement → GREEN (`-p "Db"`, `-p "EL"`, `-p "Persist"`) → FULL suite + nets; if all
  green, done; if bar moves, BLOCK per the flag → gates → commit
  `"Db: the trie knows its facts — no unasserted node survives its children"`.
  (Golden re-capture, if adjudicated, lands as its own itemized commit afterward.)

---

### Task 2: Docs

**Files:** `docs/LEDGER.md`, `docs/WALKTHROUGH.md` if it mentions phantom facts,
`README.md` if stale.

- [ ] LEDGER: the v39 legend row (the bug class prioritized by the user over content; the
  two-sided evidence — v32's reverted pruning and v38's inert discharge — resolved by one
  bit and one invariant; queries untouched by design; serialization more principled;
  whatever the bell adjudication showed, stated plainly with the trace). CLOSE the
  banked "asserted-endpoint marking" item — rewrite it as done, pointing at the row (the
  fix that was banked in v32, evidenced in v38, landed in v39). Update the v38 row's
  "safety by convention" sentence to note the convention is now enforcement.
- [ ] Gates; commit `"Docs: v39 — no more ghosts"`.
