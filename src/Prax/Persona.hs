-- | Personality as conduct, not goals (spec
-- @docs/specs/2026-07-11-v25-persona-design.md@).
--
-- A trait is a named bundle of (mostly negative) desires over the bearer's
-- /own/ conduct-marks — @honest@ values each of your own @lied@ marks
-- ("Prax.Deceit") at a cost. A bearer can do anything: trait-contrary conduct
-- carries negative utility, never prohibition, and the arithmetic is the
-- meaning ("her honesty outweighs her spite, by exactly the margin written").
-- Because the mark lands in the very state the planner evaluates, deterrence
-- needs no lookahead; and because trait desires are named vocabulary
-- ("Prax.Minds"), a /believed/ temperament nets against believed motives
-- inside prediction — knowing someone honest changes what you expect of them,
-- not just what they do.
--
-- Goal-bundles are deliberately NOT traits: a goal is a plain desire needing
-- no bundle. A trait says how you're willing to act, not what you're after.
module Prax.Persona
  ( Trait (..)
  , personaVocabulary
  , bearing
  , transparent
  , cast
  ) where

import           Data.List (nub, (\\))

import           Prax.Query (Condition (..))
import           Prax.Types
import           Prax.Derive (Axiom, axiom)

-- | A named bundle of conduct-valuations. The name must be a single path
-- segment (it becomes one in @trait.\<who\>.\<name\>@ facts).
data Trait = Trait
  { traitName    :: String
  , traitDesires :: [Desire]  -- ^ valuations over the bearer's own conduct-marks
  }
  deriving (Eq, Show)

-- | The desires a trait list contributes to a world's vocabulary. Loud error
-- on a duplicate desire name (bundles must not collide).
personaVocabulary :: [Trait] -> [Desire]
personaVocabulary traits =
  case names \\ nub names of
    (d : _) -> error ("personaVocabulary: duplicate desire name " ++ show d)
    []      -> ds
  where
    ds    = concatMap traitDesires traits
    names = map desireName ds

-- | Endow a character with a trait: they hold each of its desires by name.
bearing :: Trait -> Character -> Character
bearing t c = c { charDesires = charDesires c ++ map desireName (traitDesires t) }

-- | Temperament is worn on the sleeve: every character presumes a bearer's
-- conduct-valuations. Defeasible (@.presumed@) like all derived belief.
transparent :: Axiom
transparent = axiom
  [ Match "trait.M.T", Match "traitDesire.T.D", Match "character.P" ]
  [ "P.believes.desires.M.D.presumed" ]

-- | Deterministic roster assembly over hand-authored base characters: each
-- member 'bearing' their traits, plus the setup facts 'transparent' reads —
-- @trait.\<who\>.\<name\>@ per bearing, @traitDesire.\<trait\>.\<desire\>@
-- once per trait, @character.\<who\>@ per member. Loud errors: a trait name
-- that is not a single path segment; a duplicate trait; a borne trait missing
-- from the vocabulary list (its valuations would be silently illegible).
cast :: [Trait] -> [(Character, [Trait])] -> ([Character], [Outcome])
cast traits roster
  | (bad : _) <- [ n | n <- tnames, null n || any (`elem` (".!" :: String)) n ] =
      error ("cast: trait name " ++ show bad
             ++ " must be a nonempty single path segment (no '.' or '!')")
  | (dup : _) <- tnames \\ nub tnames =
      error ("cast: duplicate trait " ++ show dup)
  | (stray : _) <- [ traitName t | (_, ts) <- roster, t <- ts
                                 , traitName t `notElem` tnames ] =
      error ("cast: trait " ++ show stray ++ " is borne but not in the trait"
             ++ " list (its valuations would be silently illegible)")
  | otherwise =
      ( [ foldl (flip bearing) c ts | (c, ts) <- roster ]
      , [ Insert ("traitDesire." ++ traitName t ++ "." ++ desireName d)
        | t <- traits, d <- traitDesires t ]
        ++ [ Insert ("character." ++ charName c) | (c, _) <- roster ]
        ++ [ Insert ("trait." ++ charName c ++ "." ++ traitName t)
           | (c, ts) <- roster, t <- ts ] )
  where tnames = map traitName traits
