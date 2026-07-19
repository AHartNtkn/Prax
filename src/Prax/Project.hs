-- | Endeavors: long-horizon behavior as part-sets.
--
-- A project __type__ is authored vocabulary (like practices, deeds, and
-- desires); a project __instance__ emerges when a disposed character's planner
-- chooses the undertake action. The pursuit is a named 'Desire' counting the
-- owner's completed parts — __dormant__ without an instance (zero bindings,
-- zero utility), which is how disposed characters carry it permanently and
-- undertaking switches it on: conditioned wants /are/ injectable wants.
--
-- Progress itself is rewarding (+w per completed part — the authored weight is
-- how invested this character is in this kind of work), so horizon length is
-- irrelevant: every next part is locally visible to the ordinary planner. The
-- agent pursues because pursuing is rewarding, not because the planner derives
-- the end from the means — the truer model of commitment anyway.
--
-- A plan is a SET of parts, not a linear chain: parts complete independently
-- and in parallel, and NOT ALL are required for success. There is no single
-- number encoding progress, so the set of completed-part facts is the primary
-- state — the linear cursor is gone. Topology is authored two ways: 'partAfter'
-- names sibling parts that must finish first (a validated, privately-compiled
-- dependency edge), and 'partNeeds' carries world resources and genuine
-- threshold gates (raw conditions — @Subquery@\/@Count@\/@Cmp@ over the ledger
-- family expresses "fire when any 3 of 5 are done"). A culminating part's
-- 'partAfter' names the REQUIRED parts; optional parts hang off the side, each
-- +w when taken, never blocking.
--
-- Completing a part writes the per-part completion ledger
-- (@practice.\<pid\>.Owner.did.\<partName\>@) — JUSTIFIED infrastructure, not
-- world-visible fiction: with parallel\/optional parts progress is a set, and a
-- uniform ledger family is the only honest countable form for one 'Want'. The
-- ledger entry doubles as the once-guard, so each part fires ONCE per instance
-- (a stated boundary — repeatable\/counted parts are parked). A part whose
-- completion should be world-visible fiction ALSO writes a real deed via
-- 'partYields' (@swept.Actor@, witnessed) — the ledger and the fiction are
-- separate facts with separate jobs.
--
-- Because the pursuit is a named desire, projects are theory-of-mind content
-- ("Prax.Minds"): whoever comes to believe you pursue one predicts your next
-- available part. Make a part public by placing 'Prax.Witness.witnessed' in its
-- 'partYields'.
module Prax.Project
  ( Part (..)
  , endeavor
  ) where

import           Data.List (nub, (\\))

import           Prax.Query (Condition (..))
import           Prax.Types

-- | One moving part of the work: its ledger key, action label, the sibling
-- parts it depends on, what the world must provide, and what completing it
-- does to the world (beyond writing the ledger).
data Part = Part
  { partName   :: String       -- ^ single path segment; the ledger key
  , partLabel  :: String       -- ^ action label
  , partAfter  :: [String]     -- ^ dependency edges: sibling part names
  , partNeeds  :: [Condition]  -- ^ world resources; threshold gates
  , partYields :: [Outcome]
  }

-- | An authored endeavor: the undertake action (for the world to slot into
-- one of its own practices), the part-set practice, and the named pursuit
-- desire (for the world's vocabulary). One instance per owner; a finished
-- instance persists as the record of the work.
--
-- The undertake and the parts live in /different/ practices, so an endeavor
-- owner must not be 'charBoundTo' a single practice — a bound character could
-- undertake but never complete a part (or vice versa). Only bodiless
-- mechanism characters are ever bound; keep it that way for owners.
--
-- Every 'partAfter' name is validated against the actual part set — a dangling
-- or misspelled edge is a LOUD construction error, as is a name that forms or
-- feeds a cycle (the whole endeavor would be silently unreachable). The
-- fact-path convention is PRIVATE: the compiler builds the ledger conditions
-- itself, so a typo'd edge cannot fail silently as a never-available part.
endeavor :: String        -- ^ project id (a single path segment)
         -> Int           -- ^ pursuit weight: +w per completed part
         -> String        -- ^ undertake label
         -> [Condition]   -- ^ undertake gate (may be @[]@)
         -> [Part]
         -> (Action, Practice, Desire)
endeavor pid w ulabel gate parts
  | null parts =
      error ("endeavor: " ++ show pid ++ " has no parts (an endeavor is work)")
  | any (`elem` (".!" :: String)) pid =
      error ("endeavor: id " ++ show pid ++ " must be a single path segment")
  | (n : _) <- [ n | n <- names, any (`elem` (".!" :: String)) n ] =
      error ("endeavor " ++ show pid ++ ": part name " ++ show n
             ++ " must be a single path segment (it keys the ledger)")
  | (n : _) <- names \\ nub names =
      error ("endeavor " ++ show pid ++ ": duplicate part name " ++ show n)
  | ((p, e) : _) <- [ (partName p, e) | p <- parts
                    , e <- partAfter p, e `notElem` names ] =
      error ("endeavor " ++ show pid ++ ": part " ++ show p
             ++ " depends on " ++ show e ++ ", which is not a part")
  | (v : _) <- concat [ authoredVarClash [] (partNeeds p) (partYields p)
                      | p <- parts ] =
      error ("endeavor " ++ show pid ++ ": part authors " ++ show v
             ++ " -- the Prax namespace is reserved")
  | (n : _) <- filter (`notElem` reachable) names =
      error ("endeavor " ++ show pid ++ ": part " ++ show n
             ++ " is unreachable (its dependency edges form or feed a cycle)")
  | otherwise = (undertake, proj, pursuit)
  where
    names = map partName parts
    -- Transitive reachability from the edge-free roots (fixpoint). A part is
    -- reachable iff all its edges point to reachable parts; a graph with no
    -- edge-free node contains a cycle, and every cycle participant or
    -- dependent is unreachable — so reachability from the roots IS complete
    -- cycle detection.
    reachable = go [ partName p | p <- parts, null (partAfter p) ]
      where
        go acc =
          let acc' = nub (acc ++ [ partName p | p <- parts
                                 , all (`elem` acc) (partAfter p) ])
          in if length acc' == length acc then acc else go acc'
    inst suffix = "practice." ++ pid ++ suffix
    ledger n = inst (".Owner.did." ++ n)
    undertake = action ulabel (gate ++ [ Not (inst ".Actor") ])
                  [ Insert (inst ".Actor") ]
    -- No instance-fact Match: instance existence and Owner's binding ride the
    -- practice-instance ENUMERATION (the undertake fact's trie node), exactly
    -- as they always did — the old stage gate never bound Owner either. No
    -- init seed: the linear cursor is dead and nothing reads the family.
    partAction p = action (partLabel p)
      ([ Eq "Actor" "Owner", Not (ledger (partName p)) ]
       ++ [ Match (ledger d) | d <- partAfter p ]
       ++ partNeeds p)
      (Insert (ledger (partName p)) : partYields p)
    proj = practice
      { practiceId   = pid
      , practiceName = "[Owner] pursues " ++ pid
      , roles        = ["Owner"]
      , initOutcomes = []
      , actions      = map partAction parts }
    pursuit = Desire ("pursues-" ++ pid)
                (Want [ Match (inst ".Owner.did.P") ] w)
