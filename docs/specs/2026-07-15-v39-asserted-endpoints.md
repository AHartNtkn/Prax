# v39 — Asserted endpoints: the trie learns which nodes are facts

User-directed, prioritized as a BUG CLASS over new content. The defect, on the books since
v32 and biting in v38: `Prax.Db`'s trie cannot distinguish an interior node that was
ASSERTED as a fact from one that exists only as scaffolding beneath deeper facts. Retract
deletes a subtree but leaves ancestors standing, and `Match` sees subtrees — so a drained
scaffold keeps answering queries as if the fact existed. Evidence, both directions:

- **v32**: naive pruning (delete drained ancestors) was implemented and REVERTED — the
  Bar's practice instances are asserted interior facts that must survive their sub-facts
  draining (the instance-persistence pin). Some interior nodes ARE facts.
- **v38**: the un-pruned residue made a shipped mechanic silently inert (`unfeelToward`'s
  leaf delete left `feels.angry.toward` standing; the subtree-matching price never lifted)
  behind a pin that couldn't see it. Some interior nodes are NOT facts, and pretending
  they are is a lie the query layer tells.

Both are correct requirements. The representation is what's deficient; this round fixes it.

## Design: one bit, one invariant

`Db` gains an **asserted** flag beside the exclusion flag (`Db !Bool !Bool (IntMap Db)` —
excl, asserted; strict like its sibling). Semantics:

- **Insert marks its endpoint.** `insertToks`' terminal case sets the final node's
  asserted bit. Mid-path traversal PRESERVES existing marks (inserting a deeper fact
  through an asserted ancestor must not unmark it; the exclusion-eviction path likewise
  preserves the surviving child's subtree marks — eviction removes whole siblings, which
  is mark-irrelevant).
- **Retract prunes unasserted childless ancestors, eagerly.** `retractNames`' recursion
  already returns through exactly the ancestor chain: at each level, if the child came
  back unasserted AND childless, delete its entry instead of reinserting it. This
  establishes THE INVARIANT: **the trie never contains an unasserted childless node.**
- **Queries are untouched.** Under the invariant, "node exists" is equivalent to
  "asserted, or has living descendants" — `unifySyms`/`exists`/`childKeys`/`Match`
  semantics need no change and no per-query bit consultation. The whole fix lives in the
  two mutators. (`closure`/derived-fact inserts go through the same insert, so view nodes
  are marked identically; ViewInvariant's recompute-equality carries over.)
- **Serialization becomes MORE principled.** `dbToSentences`/`dbToLabeledSentences`
  currently emit leaf paths only; they gain the asserted interior nodes as their own
  sentences (an asserted node with children emits itself AND its descendants' paths).
  `insertAll` re-asserts each — assertedness round-trips through plain sentences with no
  format change: facts in, facts out. (`Prax.Persist` inherits exactness; the round-trip
  pin gains an asserted-interior-with-children case.)

## Consequences swept, not assumed

- The v32 Bar case: instances are asserted at spawn ⇒ survive drain ⇒ the
  instance-persistence pin keeps passing — this is the proof the marking design is right
  where pruning was wrong.
- The v38 case becomes impossible by construction: `toward` is never asserted, so the
  last leaf's retract prunes it and the subtree pattern stops matching. The Db-level
  repro is this round's defining pin (insert deep, delete the leaf, prefix-Match now 0).
- `feelingSomeone` and the per-target price shape are KEPT — v38's reviewer noted
  per-target pricing (−8 per grudge) is the better semantics, so it stays as a deliberate
  choice; the Emotion haddock's residue-trap warning is RETIRED (rewritten: the engine now
  prunes; the bind-shape remains the recommended pricing pattern for its per-target
  semantics, not for safety). v38's mutation-evidence pin that relied on the residue to
  demonstrate the old bug is reworked to whatever still discriminates (the discharge pin's
  price-lift assertions stay).
- v34 relevance: retract deltas now remove strictly more (the pruned ancestors);
  `mayUnifySyms`' prefix-compatibility already covers ancestors, and removal in the
  conservative direction cannot create an unsound reuse — argued in the plan, gated by
  the nets.
- Every remaining doc/comment describing the ambiguity (Db's haddocks, `dbToSentences`'
  banked-item reference, the LEDGER bank entry) updates to the new truth.

## Verification

- DbSpec pins (RED-first at the unit level): the v38 repro fixed; asserted-interior
  survival under drain (the v32 case, unit-scale); a re-asserted scaffold (insert deep,
  then insert the prefix as its own fact, then delete the deep leaf — the prefix
  survives); eviction unaffected; serialization round-trips assertedness (labeled and
  plain), including asserted-interior-with-children.
- PersistSpec: the round-trip pin extended.
- The nets: goldens expected byte-identical (v38's final review swept shipped patterns
  and found none reading residue in a decision path) — but any golden that DOES move is
  adjudicated as a probable BUGFIX (a decision was leaning on residue), itemized, and
  re-captured deliberately; never rationalized away. ViewInvariant green; full suite;
  the usual gates.
- The frozen-die, hunger, market, and feelings mechanics all re-verified green (their
  pins are the regression net for the mutator changes).

## Out of scope

Any Match-semantics change beyond the invariant (prefix patterns still see subtrees);
compaction/GC of the trie; the Script layer; new content of any kind.
