-- | Factions: one membership spine (spec docs/specs/2026-07-12-v31-faction-kin.md §1).
-- Membership is a base, single-slot fact — @member.\<who\>!\<faction\>@ — and the
-- @!@ IS the semantics: joining, defecting, and marrying-in are all the same
-- exclusion overwrite. Base @allied.*@ facts remain legal vocabulary (not every
-- alliance is a membership); 'comrades' derives additional ones.
module Prax.Faction
  ( memberPath
  , joins
  , comrades
  , factionStanding
  ) where

import           Prax.Query (Condition (..))
import           Prax.Types (Outcome (..))
import           Prax.Derive (Axiom, axiom)
import           Prax.Db (isVariable, pathNames)

-- | @member.\<who\>!\<faction\>@ (single-slot: the primary allegiance).
memberPath :: String -> String -> String
memberPath who faction
  | bad who || bad faction =
      error ("Faction: names must be nonempty single path segments (no '.' or '!'): "
             ++ show (who, faction))
  | otherwise = "member." ++ who ++ "!" ++ faction
  where bad n = null n || any (`elem` (".!" :: String)) n

-- | Join (or defect to, or marry into) a faction: one exclusion overwrite.
joins :: String -> String -> Outcome
joins who faction = Insert (memberPath who faction)

-- | Shared membership derives alliance — the feud's old base facts, generalized.
-- The derived name stays @allied@ so every downstream consumer (mutuality,
-- enemy-of-my-ally, affordances) is unchanged.
comrades :: Axiom
comrades = axiom
  [ Match "member.X!F", Match "member.Y!F", Neq "X" "Y" ]
  [ "allied.X.Y" ]

-- | Belief-gated faction standing for K-discipline worlds: an offense against
-- my faction-mate, THAT I BELIEVE HAPPENED, makes me regard the offender.
-- @factionStanding pat label@: @pat@'s FIRST variable is the offender, SECOND
-- the victim (loud error otherwise) — e.g. @"struck.A.V"@ ⇒
-- @W.believes.struck.A.V ∧ member.V!F ∧ member.W!F ∧ W≠A ⇒ regards.W.A.\<label\>@.
-- Intra-faction offenders are condemned by their own co-members: nothing
-- exempts an offender who shares the victim's faction (a co-member who
-- believes the deed regards the offender all the same, offender included in
-- the population of possible believers — only the offender's OWN belief of
-- their own act is excluded, by @W≠A@).
factionStanding :: String -> String -> Axiom
factionStanding pat label =
  case filter isVariable (pathNames pat) of
    (offender : victim : _)
      | (bad : _) <- reservedClash ->
          error ("factionStanding: pattern " ++ show pat ++ " uses " ++ show bad
                 ++ ", but W (the believer) and F (the shared faction) are the"
                 ++ " axiom's own join variables — pick a different name")
      | otherwise -> axiom
          [ Match ("W.believes." ++ pat)
          , Match ("member." ++ victim ++ "!F")
          , Match "member.W!F"
          , Neq "W" offender ]
          [ "regards.W." ++ offender ++ "." ++ label ]
    _ -> error ("factionStanding: pattern " ++ show pat
                ++ " must name an offender and a victim variable, in that order")
  where
    reservedClash = [ v | v <- filter isVariable (pathNames pat), v `elem` ["W", "F"] ]
