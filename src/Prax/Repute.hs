-- | Reputation: standing derived from evidence.
--
-- Nobody /stores/ reputation. @regards.\<observer\>.\<subject\>.\<label\>@ is
-- __derived__ ("Prax.Derive") from the observer's event-beliefs
-- ("Prax.Witness" \/ "Prax.Rumor"), so it lives only in the defeasible view:
-- it inherits information asymmetry (only those the news reached hold the
-- regard) and dissolves the moment its support does.
--
-- Standing is defeated by __atonement, not amnesia__: 'standingUnless' guards
-- the derivation with a /base-fact/ defeater (e.g. @atoned.\<culprit\>@) — one
-- insertion dissolves every derived regard at once while every belief (the
-- memory of the deed) persists. The defeater must name only base facts, never
-- derived heads, keeping the closure stratified.
--
-- 'notoriety' turns corroboration into a /global/ derived fact:
-- @notorious.\<subject\>.\<label\>@ holds while at least @k@ distinct observers
-- hold the regard — counting derived facts across fixpoint rounds. The
-- threshold is an authored world parameter with stated meaning ("the whole
-- village knows").
--
-- Conventions (as in "Prax.Rumor"): the deed pattern's __first__ variable is
-- the subject (who the standing attaches to), and the variable @Regarder@ is
-- reserved — deed patterns must not use it (enforced, v40: 'standingWith'
-- runs the deed pattern through the shared 'Prax.Types.authoredPatClash'
-- guard before splicing it beside the axiom's own @Regarder@ join variable —
-- previously a documented-but-unchecked convention, the same latent-capture
-- shape 'Prax.Sight.sightP' had before its own v40 guard). The deed
-- pattern's namespace must not overlap any valued-belief issue path in the
-- same world (the same invariant 'Prax.Rumor.gossip' documents).
module Prax.Repute
  ( standing
  , standingUnless
  , regardedAs
  , notoriety
  ) where

import           Prax.Db (isVariable, pathNames)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Derive (Axiom, axiom)
import           Prax.Types (authoredPatClash)

-- | Every observer with evidence of the deed regards its subject under @label@.
standing :: String -> String -> Axiom
standing pat = standingWith pat []

-- | 'standing', defeated by a base-fact pattern (which may use only the deed
-- pattern's variables — checked, loud error otherwise): derives only while
-- the defeater is absent.
standingUnless :: String -> String -> String -> Axiom
standingUnless pat defeater = standingWith pat [ Not (checkedDefeater pat defeater) ]

-- A defeater may use only the deed pattern's variables: negation-as-failure
-- would silently turn any OTHER variable into a global existential guard
-- (one unrelated fact dissolving everyone's standing).
checkedDefeater :: String -> String -> String
checkedDefeater pat defeater
  | all (`elem` patVars) defVars = defeater
  | otherwise = error ("standingUnless: defeater " ++ show defeater
                       ++ " uses variables outside the deed pattern "
                       ++ show pat ++ " (would defeat globally, not per-subject)")
  where
    patVars = filter isVariable (pathNames pat)
    defVars = filter isVariable (pathNames defeater)

standingWith :: String -> [Condition] -> String -> Axiom
standingWith pat extra label
  | (v : _) <- authoredPatClash ["Regarder"] (pathNames pat) =
      error ("standing: deed pattern " ++ show pat ++ " reserves variable " ++ show v
             ++ " -- the Prax namespace and Regarder (the axiom's own observer"
             ++ " join variable) are both reserved")
  | otherwise = axiom (Match ("Regarder.believes." ++ pat) : extra)
        [ "regards.Regarder." ++ subjectOf pat ++ "." ++ label ]

-- | Condition: @observer@ regards @subject@ under @label@ (a derived fact —
-- usable in preconditions and wants, which read the closed view).
regardedAs :: String -> String -> String -> Condition
regardedAs observer subject label =
  Match ("regards." ++ observer ++ "." ++ subject ++ "." ++ label)

-- | @notorious.\<subject\>.\<label\>@ while at least @k@ distinct observers hold
-- the regard.
notoriety :: String -> Int -> Axiom
notoriety label k =
  axiom [ Match ("regards.W0.T." ++ label)
        , Subquery "Rs" ["W"] [ Match ("regards.W.T." ++ label) ]
        , Count "N" "Rs"
        , Cmp Gte "N" (show k) ]
        [ "notorious.T." ++ label ]

-- The deed pattern's first variable: who the standing is about.
subjectOf :: String -> String
subjectOf pat = case filter isVariable (pathNames pat) of
  (v : _) -> v
  []      -> error ("standing: deed pattern " ++ show pat
                    ++ " names no one (a standing is about someone)")
