# Plan v54 — coercion time + the credibility net

Governing spec: `docs/specs/2026-07-19-v54-coercion-time.md` (panel-rewritten;
the no-fiat ruling recorded at the gate). One implementer task, two commits
(code+tests, then docs — the v53 pattern). RED-first per behavior; every v49
pin and golden UNMOVED.

## T1 commit 1 — the fields, the net, the fixtures

**Why each piece:** the two permanences are the mechanism's only remaining
fiat-adjacent decisions the paradigm assigns to authors; the lint closes the
one silent state (unregistered = inert threats) so the bluff/genuine pair are
both positively authored world states; the fixtures prove the two
compositional fictions instead of asserting them.

- `Prax.Coerce`:

  ```haskell
  data Coercion = Coercion
    { ...                            -- unchanged fields
    , coThreatLasts     :: Maybe Int
    , coComplianceLasts :: Maybe Int
    }

  -- the marker-insert selector (used for BOTH markers):
  lasting :: Maybe Int -> String -> Outcome
  lasting Nothing  s = Insert s
  lasting (Just n) s = InsertFor n s
  ```

  threaten's marker insert becomes `lasting (coThreatLasts co) (threatPath …)`;
  comply's becomes `lasting (coComplianceLasts co) (compliedPath …)`. The
  deposit and the extorted mark stay `Insert` (records of the attempt — spec
  §1). The comply comment's "PERMANENT … one purchase ever" rewrites to the
  two-case truth. EXPORT `punitivePrefix :: String` (`"punishes-"`) and build
  `punitiveName = punitivePrefix ++ sid` through it — the one home the lint
  imports (v51/v53 checker-imports-vocabulary precedent). Module haddock:
  three-state credibility (genuine = registered+held; bluff =
  registered-not-held, the fear is identical because the deposit is
  mechanism-owned and `believedDesires` resolves on `desires st`; unregistered
  = the accident, netted).
- `Prax.Blackmail` (:107): the literal gains `coThreatLasts = Nothing,
  coComplianceLasts = Nothing` — the stated hardcode; the :17 permanence prose
  gains its because-of-the-hardcode clause.
- `Prax.TypeCheck`: `CoercionUnmotivated String` TypeError + check after
  `deonticUnclosedErrors`, importing `Prax.Coerce (punitivePrefix)`:

  ```haskell
  -- Check 8: a deposited punitive belief names a registered desire ---------
  coercionUnmotivatedErrors :: PraxState -> [TypeError]
  coercionUnmotivatedErrors st =
    [ CoercionUnmotivated seg
    | (_, os) <- writeSites st, o <- os, s <- writesOf o
    , seg <- pathNames s, punitivePrefix `isPrefixOf` seg
    , seg `notElem` registered ]
    where registered = map desireName (desires st)
  ```

  `writesOf` is today a `where`-local of `reservedFamilyErrors`
  (TypeCheck.hs:327-333) — HOIST it to a top-level binding shared by both
  checks [plan review I1]; behavior unchanged.

  (Rides `writeSites` — authored definitions only, engine rules exempt by
  v53's provenance, which is correct: `coerce`'s output IS authored practice
  content and IS scanned. The prefix key is the mechanism's generated-name
  convention with one exported home, not a heuristic. `-Wall`: the new
  constructor forces the `app/Main.hs` describe arm — the v51 precedent.)
- CoerceSpec: the two FULL literals (racket :26, blackmailShaped :104) gain
  Nothing/Nothing; twoQuantRacket (:43) is a record UPDATE of racket and
  inherits the fields [plan review M — the first draft miscounted it].

**New fixtures/pins (all in CoerceSpec unless noted; RED observed per
behavior; the expiry harness DRIVES `roundBoundary` explicitly — the
ScheduleSpec idiom — or the pins are vacuous [C-C2]):**

1. **Nothing is today** (structural): the racket's compiled threaten/comply
   carry `Insert` for both markers (direct assertion on the generated
   `Action` values — Eq).
2. **The racket cycles** (`coComplianceLasts = Just n` variant of the racket):
   threaten→comply extracts (the demand fact lands); re-threat within n —
   comply BLOCKED (`Not complied`); drive n boundaries; complied expires;
   re-threaten; comply available and extracts AGAIN. RED: neuter `lasting` to
   always-`Insert` — the cycle pin's second extraction fails.
3. **Deal economics, pinned as observed** (the no-fiat ruling): during the
   bought period, the extorter's `pickAction` punish-vs-wait — record which
   way the racket's vengeance-kernel utilities actually point, both the
   choice and the score comparison in the assertion comment; no guarantee
   language.
4. **A stale threat is spent** (`coThreatLasts = Just n`): drive n boundaries
   past an unanswered threat; punish's standing arm gone, comply unpressured;
   a DEFIED threat's punish survives expiry (the defied marker is permanent).
   RED via the same `lasting` neuter.
5. **The bluff pair** (the existing `holdsWant` scaffold — v49 built it):
   registered-not-held vs registered-and-held — the victim's comply/defy
   decision IDENTICAL (same deposit, same believed desire); defied, the
   genuine extorter picks punish, the bluffer does not (pickAction both).
   RED: the parity half by asserting before wiring the bluff fixture's world;
   the divergence half is v49's vengeance pin pattern re-aimed.
6. **The accident is loud** (TypeCheckSpec): a practice depositing a
   `punishes-ghost` belief with no registered desire flags
   `CoercionUnmotivated "punishes-ghost"`; the genuine, bluff, and
   coercion-free worlds are clean; every shipped world stays
   `typeCheck == []`. RED: pins pre-wire, observe absent-flag, wire, GREEN.
7. **The table turns** (new small world in CoerceSpec): mob extorts vic
   (sid1); vic's counter-Coercion (sid2) has `coVictim = "M"` and
   `coTrigger = [ Match ("M.extorted.vic." ++ sid1) ]` — the extorter slot is
   a VARIABLE so the trigger BINDS the counter's victim to whoever extorted
   vic [plan review I2: a literal "mob" there binds nothing and the
   counter-threaten is malformed — the trigger must bind coVictim, per the
   primitive's own contract]; kernel + registered + held `punishes-<sid2>`;
   vic's pickAction reaches counter-threaten; both threats stand. Composition
   blocked anywhere = BLOCK and surface.
8. **Mid-racket save/resume** (PersistSpec or CoerceSpec — implementer's
   call, note where): save with the complied-expiry due pending, reload,
   drive boundaries, the cycle resumes on schedule.

Suite green (baseline 700; report delta); `-Wall` clean; BlackmailSpec +
ConfessionSpec + the v49 six-property pins re-observed UNCHANGED.

## T1 commit 2 — docs

LEDGER: the v54 row (the challenge's disposition: two fields, two proofs, the
bank EMPTIED; the panel's [D-C1] reversal-of-my-reversal story; the no-fiat
ruling) + status updates at the existing bank sites per the DONE-banked-vN
convention (:2695-2704 serial-extortion entry + v30 pointer, :1924-1934 v49
cross-ref). WALKTHROUGH :1510-1519 (serial-extortion bank paragraph —
discharged) and :1656. Coerce/Blackmail haddocks verified against shipped
behavior (commit-1 wrote them; docs pass re-reads).

## Exactness ledger

UNMOVED: every v49 pin, BlackmailSpec, ConfessionSpec, all goldens, all
analyses, all shipped worlds' typeCheck. Moves: the four Coercion literals
(+2 fields each), CoerceSpec's new fixtures/pins, TypeCheckSpec's net pins,
the enumerated doc sites. Anything else = BLOCK and trace.
