-- | Blackmail: a threat is a motive-belief deposit (spec
-- @docs/specs/2026-07-12-v30-blackmail-debt.md@, §"the leverage model").
--
-- __Threatening communicates conditional intent.__ 'shakedown' generates the
-- four-action protocol a session probe validated live at depth 2: the
-- extorter, holding evidence of some deed, threatens a co-present victim.
-- The threat inserts the ordinary fact @threatened.\<id\>.\<extorter\>.\<victim\>@
-- /and/ a motive-belief deposit — @victim.believes.desires.\<extorter\>.punishes-\<id\>.heard.\<extorter\>@
-- — over the same channel confiding and lying already ride ("Prax.Beliefs",
-- "Prax.Deceit"). No new epistemics: the victim's own round-walk
-- ("Prax.Planner") predicts the professed exposure and weighs it.
--
-- __The threat is credible because the extorter genuinely holds the punitive
-- desire it professes.__ The generated 'Desire' (@punishes-\<id\>@) pays @w@
-- per believer of the evidence once the target has been threatened or has
-- defied — a real want, world-registered like any other, carried via
-- 'Prax.Types.charDesires'. It is what motivates /threatening/ in the first
-- place (via self-recursion — exposure is one ply away); a myopically
-- unmotivated character's 'Prax.Planner.predictMove' correctly won't foresee
-- it.
--
-- __A standing threat is exposable__, not just a defied one — gating
-- exposure on defiance alone would make stalling safe forever. With
-- exposure available against silence too, waiting only ties with defiance
-- and never dominates it.
--
-- __Blackmail leaves a mark__ (v25's idiom, "Prax.Persona"): threatening also
-- deposits @\<extorter\>.extorted.\<victim\>.\<pat\>@ — the extorter's own
-- memory, priceable by a trait that values it negatively.
--
-- The world slots the four generated actions into its own practice (the
-- shape mirrors "Prax.Project"'s 'Prax.Project.endeavor'); a world wanting
-- bespoke label text wraps the actions itself.
module Prax.Blackmail
  ( shakedown
  ) where

import           Prax.Db (isVariable, pathNames, tokens, tokensToSentence)
import           Prax.Sym (intern)
import           Prax.Query (Condition (..))
import           Prax.Types (Action, Desire (..), Outcome (..), Want (..), action)
import           Prax.Beliefs (beliefAbout)
import           Prax.Witness (CoPresence, asRole)
import           Prax.Debt (debtPath, owe)

-- | @shakedown id copresence pat price w@ generates the threaten\/comply\/
-- defy\/expose protocol and the punitive 'Desire' it professes.
--
-- * @id@ — a single path segment scoping this shakedown's facts (loud error
--   otherwise), so more than one may coexist in a world.
-- * @copresence@ — the world's co-presence template ("Prax.Witness"),
--   relating @Actor@ to whichever role it is retargeted onto.
-- * @pat@ — the evidence pattern, e.g. @\"stole.V.loaf\"@; its __first__
--   variable is the victim (mirrors 'Prax.Deceit.lie'\'s convention — loud
--   error if @pat@ names no one).
-- * @price@ — the debt content the victim pays to buy silence (e.g.
--   @\"favor\"@; "Prax.Debt" composes it into a real debt/obligation).
-- * @w@ — the extorter's punitive weight: utility per believer of @pat@
--   once the threat is live or has been defied.
shakedown :: String        -- ^ id (single path segment)
          -> CoPresence    -- ^ co-presence template
          -> String        -- ^ evidence pattern; first variable = victim
          -> String        -- ^ the price (debt content)
          -> Int           -- ^ punitive weight
          -> (Desire, [Action])
shakedown sid copresence pat price w
  | any (`elem` (".!" :: String)) sid =
      error ("shakedown: id " ++ show sid
             ++ " must be a single path segment (no '.' or '!')")
  | (bad : _) <- reservedClash =
      error ("shakedown: evidence pattern " ++ show pat
             ++ " names a secondary variable " ++ show bad
             ++ ", but the punitive desire quantifies over D (the victim), W"
             ++ " (the believer) and Owner (the extorter) — pick a different"
             ++ " variable name for anyone besides the victim")
  | otherwise = (punitive, [threaten, comply, defy, expose])
  where
    victim = case filter isVariable (pathNames pat) of
      (v : _) -> v
      []      -> error ("shakedown: evidence pattern " ++ show pat
                        ++ " names no one (a threat needs a victim)")

    -- Beyond the victim, 'patD' reuses D/W/Owner as the punitive desire's own
    -- variables (below); a secondary evidence variable bearing one of those
    -- names would silently merge with them under 'renameVictim' or grounding.
    reservedClash =
      [ v | v <- pathNames pat, isVariable v, v /= victim, v `elem` ["D", "W", "Owner"] ]

    -- Fact conventions, id-scoped so multiple shakedowns coexist.
    threatPath extorter v = "threatened." ++ sid ++ "." ++ extorter ++ "." ++ v
    defiedPath v extorter = "defied." ++ sid ++ "." ++ v ++ "." ++ extorter

    punitiveName = "punishes-" ++ sid

    threaten = action ("[Actor]: threaten [" ++ victim ++ "] with what you know")
      ( [ Match (beliefAbout "Actor" pat)
        , Neq victim "Actor" ]
        ++ asRole victim copresence
        ++ [ Not (threatPath "Actor" victim) ] )
      [ Insert (threatPath "Actor" victim)
        -- the threat IS the communication of conditional intent:
      , Insert (beliefAbout victim ("desires.Actor." ++ punitiveName) ++ ".heard.Actor")
        -- the mark: the extorter's own memory of having extorted
      , Insert ("Actor.extorted." ++ victim ++ "." ++ pat) ]

    -- Victim buys silence: they are Actor, the extorter is the variable E.
    -- Guarded against paying twice for the same standing debt (the probe's
    -- own guard) — without it, a renewed threat after compliance would let
    -- the planner discover repeat extraction through its own lookahead.
    comply = action "[Actor]: buy [E]'s silence"
      [ Match (threatPath "E" "Actor"), Not (debtPath "E" "Actor" price) ]
      (owe "E" "Actor" price ++ [ Delete (threatPath "E" "Actor") ])

    defy = action "[Actor]: defy [E]"
      [ Match (threatPath "E" "Actor") ]
      [ Insert (defiedPath "Actor" "E")
      , Delete (threatPath "E" "Actor") ]

    -- The extorter exposes a standing threat OR a defiance, to a co-present
    -- hearer who doesn't already believe — Rumor's sourced-hearsay shape.
    expose = action ("[Actor]: expose [" ++ victim ++ "] to [Hearer]")
      ( [ Or [ [ Match (threatPath "Actor" victim) ]
             , [ Match (defiedPath victim "Actor") ] ]
        , Match (beliefAbout "Actor" pat) ]
        ++ asRole "Hearer" copresence
        ++ [ Neq "Hearer" "Actor"
           , Neq "Hearer" victim
           , Not (beliefAbout "Hearer" pat) ] )
      [ Insert (beliefAbout "Hearer" pat ++ ".heard.Actor") ]

    -- The punitive desire's evidence clause quantifies over a fresh victim
    -- (D, distinct from any one shakedown's grounded victim); patD is pat
    -- with its victim variable renamed to D, op-preservingly.
    patD = renameVictim victim "D" pat

    punitive = Desire punitiveName
      (Want [ Or [ [ Match (defiedPath "D" "Owner") ]
                 , [ Match (threatPath "Owner" "D") ] ]
            , Match (beliefAbout "W" patD) ]
            w)

-- | Substitute @victim@ for @newName@ throughout @pat@, preserving each
-- segment's following operator (@.@\/@!@): split via 'tokens', rename the
-- matching segment, rejoin via 'tokensToSentence' — never a naive string
-- substitution that could corrupt @.@\/@!@ punctuation.
renameVictim :: String -> String -> String -> String
renameVictim victim newName pat =
  tokensToSentence
    [ (intern (if name == victim then newName else name), op)
    | (name, op) <- tokens pat ]
