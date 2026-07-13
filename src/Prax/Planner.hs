-- | Utility-based action selection (Versu §IX) with a beyond-source lookahead
-- extension, redesigned in v23 (spec: docs/specs/2026-07-10-v23-planner-realism-design.md).
--
-- Selection is the paper's apply-and-evaluate: score each candidate by the
-- world it produces. The lookahead is a __round-walk over believed minds__:
-- one imagined round in which each other character within the actor's
-- 'predictionScope' takes one /motivated/ move chosen from the actor's
-- __believed model__ of them ("Prax.Minds" — which can be wrong), followed by
-- the actor's own next choice, recursively. Discounts: 0.9 own future move,
-- 0.5 another's. Accumulation is a discounted stream of absolute utilities
-- over the imagined round. Unknown minds and out-of-scope characters are
-- modeled as still — never as helpful.
module Prax.Planner
  ( evaluate
  , evaluateCooked
  , candidateActions
  , predictMove
  , scoreActions
  , pickAction
  ) where

import           Data.List (sortOn)
import qualified Data.Map.Strict as Map
import           Data.Maybe (listToMaybe)
import           Data.Ord (Down(..))

import           Prax.Db (Val (..), exists)
import           Prax.Query (countSatisfying, groundCondition, query, CookedCondition, queryCooked)
import           Prax.Sym (Sym, intern)
import           Prax.Types
import           Prax.Engine (possibleActions, performAction, groundedDeltaAnchors)
import           Prax.Minds (believedDesires, cookedSelfWants, cookedDesiresFor)
import           Prax.Relevance (moverReadAnchors, mayUnifySyms)

-- | Total utility of a world to a set of wants: @Σ utility × #satisfying@.
evaluate :: PraxState -> [Want] -> Int
evaluate st wants =
  sum [ wantUtility w * countSatisfying view (wantConditions w) Map.empty
      | w <- wants ]
  where view = readView st

-- | 'evaluate''s cooked mirror, fed by 'Prax.Minds.cookedSelfWants'\/
-- 'Prax.Minds.cookedDesiresFor' — the Planner's internal scoring path
-- ('scoreActions'\/'predictMove'\/'pickAction'). Case-for-case with
-- 'evaluate': same sum-of-utility-times-satisfying-count, over
-- 'queryCooked' instead of 'countSatisfying'.
evaluateCooked :: PraxState -> [([CookedCondition], Int)] -> Int
evaluateCooked st wants =
  sum [ u * length (queryCooked view cs Map.empty) | (cs, u) <- wants ]
  where view = readView st

-- | The actions a character may actually take (practice-bound filtering).
-- The dead act in no one's plans, including their own: a character marked
-- dead by the time this is consulted has no candidates, so neither
-- 'predictMove' nor the actor's own 'selfNext' recursion will plan around or
-- through a corpse.
candidateActions :: PraxState -> Character -> [GroundedAction]
candidateActions st c
  | exists (deadSentence (charName c)) (db st) = []
  | otherwise =
      let as = possibleActions st (charName c)
      in case charBoundTo c of
           Nothing  -> as
           Just pid -> filter ((== pid) . gaPracticeId) as

-- | Is the mover within the actor's prediction scope? The world's template
-- (over @Actor@/@Witness@) is grounded to the pair and queried against the
-- view; the empty template means everyone.
inScope :: PraxState -> Character -> Character -> Bool
inScope st actor m =
  not (null (query (readView st) grounded Map.empty))
  where
    grounded = map (groundCondition binds) (predictionScope st)
    binds = Map.fromList [ (intern "Actor",   VSym (intern (charName actor)))
                         , (intern "Witness", VSym (intern (charName m))) ]

-- | Is a believed desire dead RIGHT NOW: at its floor (a negative want-kind's
-- own conditions have zero bindings) or gated shut (a positive want-kind's
-- environment gate has zero bindings)? ('Prax.Relevance.livenessOf'; Owner
-- grounds to the mover @m@, matching 'Prax.Minds.cookedDesiesFor'\/
-- 'cookedSelfWants'.) A desire with no 'liveness' recipe, or classified
-- 'AlwaysLive', is never dead-now — only 'FloorCheck'\/'GateCheck' fire.
--
-- The Owner binding is passed to 'queryCooked' as a seed binding rather than
-- substituted into the conditions up front: 'Prax.Db.unifySyms' consults an
-- already-bound variable at every occurrence, so one seed binding grounds
-- @Owner@ everywhere it appears in the (possibly multi-condition) conjunct.
--
-- A 'FloorCheck' desire absent from 'cookedDesires' (@conds = []@) queries
-- via the empty-conjunction identity ('Prax.Query.queryCookedWith'\'s fold
-- over @[]@ returns @[b0]@ unchanged) — one binding, non-null, so it counts
-- LIVE, not dead: an unrecorded desire must never silently read as at-floor.
deadNow :: PraxState -> Character -> Desire -> Bool
deadNow st m d = case Map.lookup (desireName d) (liveness st) of
  Just FloorCheck     -> null (queryCooked v conds owner)
    where conds = Map.findWithDefault [] (desireName d) (cookedDesires st)
  Just (GateCheck gs) -> any (\g -> null (queryCooked v g owner)) gs
  _                   -> False
  where
    v     = readView st
    owner = Map.singleton (intern "Owner") (VSym (intern (charName m)))

-- | The predictor's guess at the mover's next move: the mover's best candidate
-- under the predictor's believed model of them — and only if it strictly
-- improves that model over doing nothing (unmotivated moves are noise, not
-- plan). 'Nothing' when the mind is unreadable or unmotivated.
predictMove :: PraxState -> Character -> Character -> Maybe GroundedAction
predictMove st p m =
  case believedDesires st p m of
    [] -> Nothing
    ds
      -- every believed desire is DEAD: statically dead (no authored action
      -- could ever improve it) OR dead-now (improvable in principle, but not
      -- in THIS state) — no candidate can strictly beat standing still, so
      -- don't ground or evaluate any (Prax.Relevance; exact — a single LIVE
      -- desire keeps the FULL model, unimprovable/dead-now costs included,
      -- so deterrents still deter)
      | all (\d -> desireName d `notElem` improvables st || deadNow st m d) ds -> Nothing
      | otherwise ->
          let model  = cookedDesiresFor st (charName m) ds
              still  = evaluateCooked st model
              scored = sortOn (\(ga, s) -> (Down s, gaLabel ga))
                         [ (a, evaluateCooked (performAction st a) model)
                         | a <- candidateActions st m ]
          in case scored of
               ((a, s) : _) | s > still -> Just a
               _                        -> Nothing

-- The other living characters, one full cycle in cast order starting after
-- the actor (the loop's round-robin order).
othersAfter :: PraxState -> Character -> [Character]
othersAfter st actor =
  filter ((/= charName actor) . charName) (drop (i + 1) cs ++ take (i + 1) cs)
  where
    cs = livingCharacters st
    i  = case [ k | (k, c) <- zip [0 :: Int ..] cs, charName c == charName actor ] of
           (k : _) -> k
           []      -> length cs - 1   -- an actor outside the cast walks everyone

-- | One imagined path's accumulated effect on the pick's root state, as
-- anchor families with the derived-fact cone folded in: the moment any
-- extension feeds an axiom ('footprint'), every fireable head ('axiomHeads')
-- joins the delta — and stays, because heads are themselves in the
-- footprint. 'Nothing' is the opaque path: some applied outcome could not
-- be bounded ('Prax.Engine.groundedDeltaAnchors'), so nothing at or below
-- it may reuse. Spec: docs/specs/2026-07-13-v34-prediction-reuse.md.
type PathDelta = Maybe [[Sym]]

extendDelta :: PraxState -> PathDelta -> Maybe [[Sym]] -> PathDelta
extendDelta st (Just old) (Just new) =
  Just (old ++ new ++ [ h | feeds, h <- axiomHeads st, h `notElem` old ])
  where feeds = any (\n -> any (mayUnifySyms n) (footprint st)) new
extendDelta _ _ _ = Nothing

-- | Score each candidate by the imagined round it opens (best first; ties
-- broken by label for determinism). Within one pick, every prediction is
-- either the root state's — reused when the path delta provably cannot
-- reach anything that (actor, mover) prediction reads — or computed live,
-- exactly as before; the reused value is EQUAL to the live one (the spec's
-- soundness argument), so decisions are bit-for-bit unchanged.
scoreActions :: Int -> PraxState -> Character -> [(GroundedAction, Double)]
scoreActions depth st0 actor = go depth (Just []) st0
  where
    -- The root memo: each mover's step decision (scope gate + prediction)
    -- at the PICK's root state. Map values are lazy — a mover whose pairs
    -- never reuse never computes its root prediction.
    rootStep = Map.fromList
      [ (charName m, stepPredict st0 m) | m <- othersAfter st0 actor ]
    rootReads = Map.fromList
      [ (charName m, moverReadAnchors st0 actor m) | m <- othersAfter st0 actor ]
    stepPredict s m
      | inScope s actor m = predictMove s actor m
      | otherwise         = Nothing

    -- Reuse the root's decision when sound; live otherwise (opaque path,
    -- a mover the root never enumerated, or a delta/read intersection).
    predictAt :: PathDelta -> PraxState -> Character -> Maybe GroundedAction
    predictAt (Just delta) s m
      | Just rs <- Map.lookup (charName m) rootReads
      , not (any (\d -> any (mayUnifySyms d) rs) delta)
      = Map.findWithDefault (stepPredict s m) (charName m) rootStep
    predictAt _ s m = stepPredict s m

    go d delta st =
      sortOn (\(ga, s) -> (Down s, gaLabel ga))
        [ (a, valueAfter d
                (extendDelta st0 delta (groundedDeltaAnchors st a))
                (performAction st a))
        | a <- candidateActions st actor ]

    valueAfter d delta st1 = base + rest
      where
        base = fromIntegral (evaluateCooked st1 (cookedSelfWants st1 actor))
        rest
          | d <= 0    = 0
          | otherwise = othersScore + selfNext
          where
            (afterRound, afterDelta, othersScore) =
              foldl step (st1, delta, 0) (othersAfter st1 actor)
            step (s, dlt, acc) m = case predictAt dlt s m of
              Nothing -> (s, dlt, acc)
              Just ga ->
                let s'   = performAction s ga
                    dlt' = extendDelta st0 dlt (groundedDeltaAnchors s ga)
                in (s', dlt', acc + 0.5 * fromIntegral (evaluateCooked s' (cookedSelfWants s' actor)))
            selfNext = case go (d - 1) afterDelta afterRound of
              ((_, v) : _) -> 0.9 * v
              []           -> 0

-- | The actor's best action (deterministic), if any.
pickAction :: Int -> PraxState -> Character -> Maybe GroundedAction
pickAction depth st actor = fst <$> listToMaybe (scoreActions depth st actor)
