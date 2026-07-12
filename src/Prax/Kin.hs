-- | Kinship: base facts + derived closure (spec §2). Marriage moves membership —
-- the fold's payoff in one line.
module Prax.Kin
  ( kinAxioms
  , wed
  , succession
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Action, Outcome (..), action)
import           Prax.Derive (Axiom, axiom)
import           Prax.Faction (joins)

-- | Marriage symmetry, siblings, grandparents, in-laws — all derived, all
-- retraction-safe (dissolve the base fact and the closure forgets with it).
kinAxioms :: [Axiom]
kinAxioms =
  [ axiom [ Match "married.A.B" ]                        [ "married.B.A" ]
  , axiom [ Match "parent.P.X", Match "parent.P.Y"
          , Neq "X" "Y" ]                                [ "sibling.X.Y" ]
  , axiom [ Match "parent.G.P", Match "parent.P.C" ]     [ "grandparent.G.C" ]
  , axiom [ Match "married.A.B", Match "parent.P.A" ]    [ "inLaw.P.B" ]
  , axiom [ Match "married.A.B", Match "sibling.A.S" ]   [ "inLaw.S.B" ]
  ]

-- | @wed joiner faction spouse@: the marriage fact plus the joiner's membership
-- overwrite. WHO moves households (and to which faction) is the author's choice
-- per wedding — world content, not module policy.
wed :: String -> String -> String -> [Outcome]
wed joiner faction spouse =
  [ Insert ("married." ++ joiner ++ "." ++ spouse)
  , joins joiner faction ]

-- | Succession as exclusion: any child of the dead holder may claim; the
-- single-slot office takes one — first motivated claimant wins. No invented
-- primogeniture (age does not exist in the vocabulary).
succession :: String -> Action
succession office
  | null office || any (`elem` (".!" :: String)) office =
      error ("succession: office " ++ show office ++ " must be a single path segment")
  | otherwise = action ("[Actor]: claim the office of " ++ office)
      [ Match ("office." ++ office ++ "!H")
      , Match "dead.H"
      , Match "parent.H.Actor"
      , Neq "Actor" "H" ]
      [ Insert ("office." ++ office ++ "!Actor") ]
