-- | Blackmail: the exposure instance of coercion (spec
-- @docs/specs/2026-07-17-v49-coercion.md@; the original leverage model,
-- @docs/specs/2026-07-12-v30-blackmail-debt.md@ §"the leverage model").
--
-- __Blackmail is one 'Prax.Coerce.Coercion', composed not wrapped.__
-- 'shakedown' keeps its v30 signature but builds a 'Prax.Coerce.Coercion'
-- record and returns @'Prax.Coerce.coerce'@ of it. The four-action protocol
-- (threaten\/comply\/defy\/expose), the motive-belief deposit, the extorted
-- mark, the standing-threat punish gate, and the punitive 'Desire' are all
-- the primitive's; blackmail supplies only the /content/:
--
--   * __trigger__ — the extorter holds evidence (@Actor.believes.\<pat\>@) and
--     the victim is co-present (@pat@\'s first variable, retargeted through the
--     world's co-presence template). The primitive appends @Neq victim Actor@
--     and the not-already-threatened gate.
--   * __demand__ — a debt (@Prax.Debt.owe@): the victim buys silence with a
--     favour. The re-buy guard is the primitive's permanent @complied@ marker,
--     not the debt itself.
--   * __punish__ — expose: plant the evidence in a co-present non-believer
--     (Rumor's sourced-hearsay shape). The primitive prepends the
--     standing-threat-or-defiance availability core.
--   * __kernel__ — believers of the evidence. Authored with the PLAIN believer
--     name @Believer@ and @pat@\'s own plain victim variable;
--     'Prax.Coerce.coerce' alpha-renames them into the @Prax@ namespace
--     (@Believer@ → @PraxW@, the victim → @PraxD@), so the punitive want pays
--     @w@ per believer of the evidence about a threatened\/defied victim. The
--     believer is named for its role rather than @W@ so that a same-named
--     secondary evidence variable cannot merge with it under the rename.
--
-- __The threat is credible because the extorter genuinely holds the punitive
-- desire it professes.__ The returned 'Desire' (@punishes-\<id\>@) must be
-- world-registered ('Prax.Engine.setDesires') and carried
-- ('Prax.Types.charDesires'); it is what motivates /threatening/ in the first
-- place (self-recursion — exposure is one ply away), and a myopically
-- unmotivated character's 'Prax.Planner.predictMove' correctly won't foresee
-- it. See "Prax.Coerce"\'s registration contract.
--
-- The world slots the four generated actions into its own practice (the shape
-- mirrors "Prax.Project"'s 'Prax.Project.endeavor'); a world wanting bespoke
-- label text passes it as the record's label fields.
module Prax.Blackmail
  ( shakedown
  ) where

import           Prax.Db (isVariable, pathNames)
import           Prax.Query (Condition (..))
import           Prax.Types (Action, Desire, Outcome (..), authoredPatClash)
import           Prax.Beliefs (beliefAbout)
import           Prax.Witness (CoPresence, asRole)
import           Prax.Debt (owe)
import           Prax.Coerce (Coercion (..), coerce)

-- | @shakedown id copresence pat price w@ builds the exposure 'Coercion' and
-- returns @'coerce'@ of it: the threaten\/comply\/defy\/expose protocol and
-- the punitive 'Desire' it professes.
--
-- * @id@ — a single path segment scoping this shakedown's facts (the
--   primitive errors loudly otherwise), so more than one may coexist.
-- * @copresence@ — the world's co-presence template ("Prax.Witness"),
--   relating @Actor@ to whichever role it is retargeted onto.
-- * @pat@ — the evidence pattern, e.g. @\"stole.V.loaf\"@; its __first__
--   variable is the victim (mirrors 'Prax.Deceit.lie'\'s convention — loud
--   error if @pat@ names no one).
-- * @price@ — the debt content the victim pays to buy silence (e.g.
--   @\"favor\"@; "Prax.Debt" composes it into a real debt\/obligation).
-- * @w@ — the extorter's punitive weight: utility per believer of @pat@ once
--   the threat is live or has been defied.
shakedown :: String        -- ^ id (single path segment)
          -> CoPresence    -- ^ co-presence template
          -> String        -- ^ evidence pattern; first variable = victim
          -> String        -- ^ the price (debt content)
          -> Int           -- ^ punitive weight
          -> (Desire, [Action])
shakedown sid copresence pat price w
  | (bad : _) <- offenders =
      error ("shakedown: evidence pattern " ++ show pat
             ++ " names a secondary variable " ++ show bad
             ++ ", but Owner (the punitive desire's extorter, which the kernel"
             ++ " rename leaves untouched) and Hearer (expose's own audience)"
             ++ " are frame roles the evidence pattern is spliced beside — a"
             ++ " secondary variable bearing either name would silently merge"
             ++ " with the frame's binding; pick a different name for anyone"
             ++ " besides the victim")
  | otherwise = coerce blackmail
  where
    -- The victim is pat's first variable (loud error if pat names no one).
    victim = case filter isVariable (pathNames pat) of
      (v : _) -> v
      []      -> error ("shakedown: evidence pattern " ++ show pat
                        ++ " names no one (a threat needs a victim)")

    -- The instance guard, reasoned against the primitive's own field guards.
    -- pat is spliced into THREE frames: threaten's trigger (Actor = extorter),
    -- expose's punish (Actor = extorter, Hearer = audience), and the punitive
    -- kernel (Owner = extorter, the victim renamed to PraxD). The primitive
    -- already forbids the @Prax@ namespace on every field, and — because the
    -- kernel carries @pat@ — its kernel guard forbids @Actor@ and @E@ there.
    -- What the primitive CANNOT catch for @pat@ is @Owner@ (the kernel rename
    -- deliberately exempts the mechanism interface name, so a secondary
    -- @Owner@ would pass through and capture the desire's extorter) and
    -- @Hearer@ (expose's frame legitimately binds it as the audience, so
    -- naming it there is a merge, not a capture the punish guard flags). Those
    -- two the instance adds; the rest the primitive owns.
    offenders = authoredPatClash ["Owner", "Hearer"]
                  (filter (/= victim) (pathNames pat))

    blackmail = Coercion
      { coId            = sid
      , coVictim        = victim
        -- the extorter holds evidence, the victim is co-present
      , coTrigger       = Match (beliefAbout "Actor" pat) : asRole victim copresence
      , coThreatenLabel = "[Actor]: threaten [" ++ victim ++ "] with what you know"
      , coDemandLabel   = "[Actor]: buy [E]'s silence"
      , coDemand        = owe "E" "Actor" price
      , coPunishLabel   = "[Actor]: expose [" ++ victim ++ "] to [Hearer]"
        -- expose to a co-present hearer who doesn't already believe; the
        -- primitive prepends the standing-threat-or-defiance availability core
      , coPunishWhen    = Match (beliefAbout "Actor" pat)
                          : asRole "Hearer" copresence
                          ++ [ Neq "Hearer" "Actor"
                             , Neq "Hearer" victim
                             , Not (beliefAbout "Hearer" pat) ]
        -- Rumor's sourced-hearsay plant
      , coPunishOuts    = [ Insert (beliefAbout "Hearer" pat ++ ".heard.Actor") ]
        -- believers of the evidence, authored plain; coerce lifts Believer ->
        -- PraxW and the victim -> PraxD (a descriptive name, so a secondary
        -- evidence variable never collides with the believer under the rename)
      , coKernel        = [ Match (beliefAbout "Believer" pat) ]
      , coWeight        = w
      }
