-- | Confession & absolution (spec @docs/specs/2026-07-12-v32-confession.md@).
-- Marks convert, never delete; confession self-incriminates through the
-- ordinary hearsay channel; absolution is a refusable second-party grant;
-- an absolver's patience is what they KNOW (per-regarder, permanent by
-- memory, fed by gossip and confession alike).
module Prax.Confession
  ( confess
  , absolve
  , incorrigible
  ) where

import           Prax.Db (isVariable, pathNames, tokens, tokensToSentence)
import           Prax.Sym (intern)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Types (Action, Outcome (..), action)
import           Prax.Derive (Axiom, axiom)
import           Prax.Beliefs (beliefAbout)
import           Prax.Witness (CoPresence, asRole)

segOk :: String -> Bool
segOk n = not (null n) && all (`notElem` (".!" :: String)) n

reservedIn :: String -> [String] -> [String]
reservedIn pat vs = [ v | v <- filter isVariable (pathNames pat), v `elem` vs ]

-- | Confess ONE deed (one mark binding) to a co-present hearer. The lied-mark
-- is the precondition and it CONVERTS — a deed can be confessed once; further
-- hearers learn by gossip. @H@ is the mark's own original-hearer slot and is
-- reserved in the event pattern.
confess :: String -> CoPresence -> String -> String -> Action
confess kind copresence pat label
  | not (segOk kind) =
      error ("confess: mark kind " ++ show kind ++ " must be a single path segment")
  | (v : _) <- reservedIn pat ["H", "Hearer", "Actor"] =
      error ("confess: event pattern " ++ show pat ++ " reserves variable " ++ show v
             ++ " (the mark's hearer slot / the action's own roles)")
  | otherwise = action label conds outs
  where
    liedPath      = "Actor." ++ kind ++ ".H." ++ pat
    confessedPath = "Actor.confessed.H." ++ pat
    conds = Match liedPath
          : asRole "Hearer" copresence
         ++ [ Neq "Hearer" "Actor" ]
    outs  = [ Delete liedPath
            , Insert confessedPath
            , Insert (beliefAbout "Hearer" pat ++ ".heard.Actor") ]

-- | Grant absolution: insert the world's defeater for a deed confessed TO YOU
-- (the belief must be heard from its own doer — gossip does not qualify),
-- unless your patience is spent (the incorrigibility regard).
absolve :: String -> String -> String -> String -> Action
absolve defeater pat incLabel label
  | not (segOk defeater) || not (segOk incLabel) =
      error ("absolve: defeater/label must be single path segments: "
             ++ show (defeater, incLabel))
  | (v : _) <- reservedIn pat ["Actor"] =
      error ("absolve: event pattern " ++ show pat ++ " reserves variable " ++ show v)
  | otherwise = case filter isVariable (pathNames pat) of
      [] -> error ("absolve: event pattern " ++ show pat
                   ++ " names no one (the FIRST variable is the confessor)")
      (confessor : _) -> action label
        [ Match (beliefAbout "Actor" pat ++ ".heard." ++ confessor)
        , Neq "Actor" confessor
        , Not ("regards.Actor." ++ confessor ++ "." ++ incLabel)
        , Not (defeater ++ "." ++ confessor) ]
        [ Insert (defeater ++ "." ++ confessor) ]

-- | Patience as knowledge: W regards the offender @label@ once W believes at
-- least @k@ distinct instances of the deed — however W learned them.
--
-- Mirrors 'Prax.Repute.notoriety''s Count idiom pointed inward, with one
-- correction the mirroring forces: 'notoriety' keeps its outer existence
-- check (@regards.W0.T.label@) and its counting 'Subquery' (@regards.W.T.label@)
-- on DIFFERENT names for the counted role (@W0@ vs @W@) — only the FIXED
-- role (@T@, the subject) is shared text between the two. A literal
-- transliteration that instead points the pattern's own deed variables
-- (@rest@) at both the outer 'Match' and the inner 'Subquery' verbatim binds
-- @rest@ before the subquery ever runs, so the subquery re-checks the single
-- already-known instance and 'Count' is always 1 — the threshold could never
-- fire for any @k > 1@ (checked directly against the engine: a believer of
-- two distinct instances of the same offender's deed failed to derive the
-- regard at @k = 2@ under that transliteration). The outer 'Match' here
-- instead witnesses existence over DUMMY names for @rest@ (the deed's
-- non-offender variables, each suffixed @0@ — 'notoriety''s own @W@\/@W0@
-- convention generalized to every deed variable) so they stay free entering
-- the 'Subquery', which alone does the real per-instance counting under
-- their true names.
incorrigible :: String -> Int -> String -> Axiom
incorrigible pat k label
  | not (segOk label) =
      error ("incorrigible: label " ++ show label ++ " must be a single path segment")
  | (v : _) <- reservedIn pat ["W", "Ds", "N"] =
      error ("incorrigible: pattern " ++ show pat ++ " reserves variable " ++ show v)
  | otherwise = case filter isVariable (pathNames pat) of
      [] -> error ("incorrigible: pattern " ++ show pat ++ " names no offender")
      [_] -> error ("incorrigible: pattern " ++ show pat
                    ++ " has no deed variables to count (a single-variable pattern"
                    ++ " admits only one possible instance; a k > 1 threshold could"
                    ++ " never fire, and k <= 1 needs no threshold at all)")
      (offender : rest)
        | (v : _) <- [ d | d <- witnessNames, d `elem` (offender : rest) ] ->
            error ("incorrigible: pattern " ++ show pat ++ " variable " ++ show v
                   ++ " collides with the outer witness-naming convention"
                   ++ " (a deed variable named <name>0 shadowing another"
                   ++ " variable named <name>)")
        | otherwise -> axiom
            [ Match ("W.believes." ++ witnessPat)
            , Subquery "Ds" rest [ Match ("W.believes." ++ pat) ]
            , Count "N" "Ds"
            , Cmp Gte "N" (show k) ]
            [ "regards.W." ++ offender ++ "." ++ label ]
        where
          witnessNames = map (++ "0") rest
          witnessPat = tokensToSentence
            [ (intern (if name `elem` rest then name ++ "0" else name), op)
            | (name, op) <- tokens pat ]
