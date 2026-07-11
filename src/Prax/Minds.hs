-- | Minds as objects of belief.
--
-- To believe something about a mind, minds must be nameable: a world declares
-- a vocabulary of named, @Owner@-parameterized 'Desire's, and a motive-belief
-- is an ordinary belief over the issue @desires.\<owner\>.\<name\>@ in the v20
-- provenance shape (@.seen@ \/ @.heard.\<src\>@ \/ @.presumed@) — so the whole
-- information stack (gossip, lies, confides, forgetting, derivation) works on
-- minds unchanged. An unnamed 'charWants' want has no name to believe and is
-- therefore inherently unreadable (right for the story manager's metalevel
-- desires). Common knowledge is /derived/, defeasibly: 'professed' spreads a
-- character's openly-held desire; 'conventional' presumes a desire of everyone
-- — even of those who do not actually have it (you expect strangers to be
-- conventional, and can be wrong).
module Prax.Minds
  ( wantFor
  , selfWants
  , believedWants
  , professed
  , conventional
  ) where

import qualified Data.Map.Strict as Map

import           Prax.Db (Val (..), exists)
import           Prax.Query (Condition (..), groundCondition)
import           Prax.Types
import           Prax.Derive (Axiom, axiom)

-- | Instantiate a desire template for its owner (grounds @Owner@).
wantFor :: String -> Desire -> Want
wantFor owner (Desire _ (Want cs u)) =
  Want (map (groundCondition (Map.singleton "Owner" (VStr owner))) cs) u

-- | What a character plans with: their whole mind — unnamed wants plus their
-- own named desires, instantiated.
selfWants :: PraxState -> Character -> [Want]
selfWants st c =
  charWants c
    ++ [ wantFor (charName c) d
       | d <- desires st, desireName d `elem` charDesires c ]

-- | The predictor's believed model of the mover: every vocabulary desire the
-- predictor believes (any provenance) the mover to have. The model can be
-- wrong — it is the predictor's, not the mover's.
believedWants :: PraxState -> Character -> Character -> [Want]
believedWants st p m =
  [ wantFor (charName m) d
  | d <- desires st
  , exists (charName p ++ ".believes.desires." ++ charName m ++ "." ++ desireName d) view ]
  where view = readView st

-- | An openly-held desire is presumed known by everyone:
-- @professes.\<owner\>.\<name\>@ ⇒ every character presumes it.
professed :: Axiom
professed = axiom
  [ Match "professes.Owner.D", Match "character.P" ]
  [ "P.believes.desires.Owner.D.presumed" ]

-- | A conventional desire is presumed of everyone by everyone — even of those
-- who do not actually have it.
conventional :: Axiom
conventional = axiom
  [ Match "conventional.D", Match "character.P", Match "character.M" ]
  [ "P.believes.desires.M.D.presumed" ]
