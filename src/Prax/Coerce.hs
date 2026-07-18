-- | Coercion: leverage as a content-agnostic primitive.
--
-- __A threat is conditional intent made credible by a desire.__ 'coerce'
-- generates the four-action protocol — threaten\/comply\/defy\/punish — and
-- the punitive 'Desire' that professes the intent, exactly as
-- "Prax.Blackmail" does for its exposure instance, but with the /content/
-- (what makes the threat available, what compliance costs, what punishment
-- does, and what the extorter VALUES about the punished state) lifted out into
-- the 'Coercion' record. Blackmail is one instance ("Prax.Blackmail"); a
-- protection racket burning a barn is another (see @Prax.CoerceSpec@).
--
-- __Evidence is optional.__ Unlike 'Prax.Blackmail.shakedown' (whose threaten
-- hardcodes an evidence belief and whose motivation counts believers), the
-- primitive owns none of that: the trigger (including how the victim is
-- reached — co-presence for an in-person threat, a letter in absentia) is
-- author content, and the punitive kernel — the state the extorter values —
-- is whatever the instance names. Exposure of held evidence and vengeance for
-- a burned barn are the same skeleton with different kernels.
--
-- __The kernel variable law.__ The author writes the kernel ('coKernel') with
-- NO @Prax@-namespaced variables (the v40 splice guards forbid them on every
-- authored field). The kernel references the victim by 'coVictim'\'s name and
-- any fresh quantifier (a believer, a witness) by an ordinary plain name.
-- 'coerce' then alpha-renames the kernel INTO the @Prax@ namespace,
-- op-preservingly ('namespaceKernel'): the victim → @PraxD@, and every other
-- author-introduced free variable that is not the mechanism interface name
-- @Owner@ → @PraxW@, @PraxW2@, … in first-appearance order. Only the
-- post-rename OUTPUT is @Prax@-namespaced; no author ever writes one.
--
-- __Registration contract.__ The returned 'Desire' is what makes the threat
-- credible: it must be REGISTERED in the world's vocabulary
-- ('Prax.Engine.setDesires') and HELD by the extorter
-- ('Prax.Types.charDesires'). The name-identity half of credibility is
-- structural — the deposited motive-belief and the desire share one generated
-- name (@punishes-\<id\>@) by construction — but an OMITTED registration still
-- yields a silently non-credible threat: the victim's round-walk
-- ("Prax.Planner") can only foresee a punishment the extorter is actually
-- modeled as wanting.
module Prax.Coerce
  ( Coercion(..)
  , coerce
  , namespaceKernel
  ) where

import           Data.List (nub)
import qualified Data.Map.Strict as Map

import           Prax.Db (isVariable, tokens, tokensToSentence)
import           Prax.Sym (intern)
import           Prax.Query (Condition (..), conditionVars)
import           Prax.Types (Action, Desire (..), Outcome (..), Want (..), action,
                             authoredVarClash, isPraxVar)
import           Prax.Beliefs (beliefAbout)

-- | A coercion: the leverage skeleton with its content in named fields.
data Coercion = Coercion
  { coId          :: String       -- ^ single path segment; scopes this coercion's facts
  , coVictim      :: String       -- ^ the victim role variable (see the reserved set)
  , coTrigger     :: [Condition]  -- ^ what makes threatening available, INCLUDING how
                                  -- the victim is reached (co-presence, a letter, …).
                                  -- Must BIND the victim.
  , coDemandLabel :: String       -- ^ display template for the comply action
  , coDemand      :: [Outcome]    -- ^ what compliance does
  , coPunishLabel :: String       -- ^ display template for the punish action
  , coPunishWhen  :: [Condition]  -- ^ EXTRA punish availability (the core gate is
                                  -- mechanism-owned: a standing threat or a defiance)
  , coPunishOuts  :: [Outcome]    -- ^ what punishment does
  , coKernel      :: [Condition]  -- ^ what the extorter VALUES about the punished
                                  -- state, authored with plain variable names
  , coWeight      :: Int          -- ^ the extorter's punitive weight
  }

-- | @coerce coercion@ generates the threaten\/comply\/defy\/punish protocol
-- and the punitive 'Desire' it professes. The 'Desire' must be registered and
-- held (see the module haddock's registration contract).
--
-- Guards, all loud:
--
--   * 'coId' is a single path segment (no @.@\/@!@).
--   * 'coVictim' is not a reserved name — @Actor@\/@E@ (the generated actions'
--     own extorter\/victim roles), @Owner@ (the punitive desire's extorter),
--     @Hearer@, or the @Prax@ namespace. This also CLOSES a latent hole a
--     derive-and-filter design left open: a victim named @Actor@ would have
--     produced an unsatisfiable @Neq Actor Actor@ threaten, silently.
--   * the v40 splice guards on every authored field, each forbidding the
--     mechanism names that would capture in that field's own generated query.
coerce :: Coercion -> (Desire, [Action])
coerce co
  | any (`elem` (".!" :: String)) sid =
      error ("coerce: id " ++ show sid
             ++ " must be a single path segment (no '.' or '!')")
  | victim `elem` ["Actor", "E", "Owner", "Hearer"] || isPraxVar victim =
      error ("coerce: victim variable " ++ show victim
             ++ " is reserved — Actor and E (the generated actions' own extorter"
             ++ " and victim roles), Owner (the punitive desire's extorter),"
             ++ " Hearer, and the Prax namespace are all mechanism-owned;"
             ++ " pick another name for the victim")
  | (bad : _) <- triggerClash =
      error ("coerce: trigger names " ++ show bad
             ++ ", but Actor (the threatening extorter) and E (the victim's"
             ++ " frame in comply/defy) are mechanism-owned in the threaten"
             ++ " query; the trigger may name the victim (" ++ show victim
             ++ ") and its own variables only")
  | (bad : _) <- demandClash =
      error ("coerce: demand names " ++ show bad
             ++ ", but in the comply query the victim is Actor and the extorter"
             ++ " is E; the victim variable " ++ show victim
             ++ " is unbound there — refer to the victim as Actor")
  | (bad : _) <- punishClash =
      error ("coerce: punish names " ++ show bad
             ++ ", but E is the victim's frame (comply/defy); the punish query's"
             ++ " extorter is Actor and its victim is " ++ show victim)
  | (bad : _) <- kernelClash =
      error ("coerce: kernel names " ++ show bad
             ++ ", but the kernel's frame is the punitive desire — its extorter"
             ++ " is Owner and its victim is renamed to PraxD; Actor/E and the"
             ++ " Prax namespace are not the kernel's to write")
  | otherwise = (punitive, [threaten, comply, defy, punish])
  where
    sid    = coId co
    victim = coVictim co

    -- Each authored field forbids exactly the mechanism names that would
    -- capture in its OWN generated query: threaten binds Actor (extorter),
    -- comply/defy bind Actor (victim) and E (extorter), punish binds Actor
    -- (extorter) and the literal victim, the desire binds Owner and PraxD.
    triggerClash = authoredVarClash ["Actor", "E"] (coTrigger co) []
    demandClash  = authoredVarClash [victim] [] (coDemand co)
    punishClash  = authoredVarClash ["E"] (coPunishWhen co) (coPunishOuts co)
    kernelClash  = authoredVarClash ["Actor", "E"] (coKernel co) []

    -- Fact conventions, id-scoped so multiple coercions coexist.
    threatPath  extorter v = "threatened." ++ sid ++ "." ++ extorter ++ "." ++ v
    defiedPath  v extorter = "defied." ++ sid ++ "." ++ v ++ "." ++ extorter
    compliedPath extorter v = "complied." ++ sid ++ "." ++ extorter ++ "." ++ v

    punitiveName = "punishes-" ++ sid

    -- Actor is the extorter. The threat IS the communication of conditional
    -- intent: the ordinary marker, the motive-belief deposit (over the same
    -- channel confiding and lying ride), and the extorter's own mark.
    threaten = action ("[Actor]: threaten [" ++ victim ++ "]")
      ( coTrigger co
        ++ [ Neq victim "Actor"
           , Not (threatPath "Actor" victim) ] )
      [ Insert (threatPath "Actor" victim)
      , Insert (beliefAbout victim ("desires.Actor." ++ punitiveName) ++ ".heard.Actor")
      , Insert ("Actor.extorted." ++ victim ++ "." ++ sid) ]

    -- The victim buys off the threat: they are Actor, the extorter is E. The
    -- complied marker is PERMANENT per (id, extorter, victim) — one purchase
    -- per coercion, ever — so a renewed threat after compliance extracts
    -- nothing (the planner cannot rediscover repeat extraction).
    comply = action (coDemandLabel co)
      [ Match (threatPath "E" "Actor"), Not (compliedPath "E" "Actor") ]
      ( coDemand co
        ++ [ Insert (compliedPath "E" "Actor")
           , Delete (threatPath "E" "Actor") ] )

    defy = action "[Actor]: defy [E]"
      [ Match (threatPath "E" "Actor") ]
      [ Insert (defiedPath "Actor" "E")
      , Delete (threatPath "E" "Actor") ]

    -- The extorter punishes a STANDING threat or a defiance — gating on
    -- defiance alone would make stalling safe forever. Actor is the extorter.
    punish = action (coPunishLabel co)
      ( Or [ [ Match (threatPath "Actor" victim) ]
           , [ Match (defiedPath victim "Actor") ] ]
        : coPunishWhen co )
      (coPunishOuts co)

    -- The punitive desire pays coWeight per (victim, valued-state) pair: the
    -- Or clause (victim = PraxD, extorter = Owner) joins the renamed kernel on
    -- PraxD. Owner-templated, instantiated per holder ('Prax.Minds.wantFor').
    punitive = Desire punitiveName
      (Want ( Or [ [ Match (defiedPath "PraxD" "Owner") ]
                 , [ Match (threatPath "Owner" "PraxD") ] ]
              : namespaceKernel victim (coKernel co) )
            (coWeight co))

-- | Alpha-rename an author-written kernel into the @Prax@ namespace,
-- op-preservingly (generalizing 'Prax.Blackmail'\'s @renameVictim@ from one
-- victim to the whole free-variable frame). The @victim@ variable → @PraxD@;
-- every other free variable, in first-appearance order and excluding the
-- mechanism interface name @Owner@, → @PraxW@, @PraxW2@, …. Renaming is by
-- NAME, applied uniformly through every 'Condition' constructor — so a
-- binder ('Subquery'\'s set\/find variables) and its interior uses move
-- together, and 'Match'\/'Not' pattern segments round-trip through 'tokens'
-- so each segment's following @.@\/@!@ operator is preserved.
namespaceKernel :: String -> [Condition] -> [Condition]
namespaceKernel victim conds = map (renameCond subst) conds
  where
    freeVars = nub (filter isVariable (concatMap conditionVars conds))
    others   = filter (\v -> v /= victim && v /= "Owner") freeVars
    m = Map.fromList ((victim, "PraxD") : zip others praxNames)
    praxNames = "PraxW" : [ "PraxW" ++ show n | n <- [2 :: Int ..] ]
    subst n = Map.findWithDefault n n m

-- | Apply a name substitution through every constructor of a 'Condition'.
-- 'Match'\/'Not' sentences are split via 'tokens', each segment name mapped,
-- and rejoined via 'tokensToSentence' — never a naive string substitution
-- that could corrupt @.@\/@!@ punctuation.
renameCond :: (String -> String) -> Condition -> Condition
renameCond f = go
  where
    go c = case c of
      Match s          -> Match (renameSentence s)
      Not s            -> Not (renameSentence s)
      Eq x y           -> Eq (f x) (f y)
      Neq x y          -> Neq (f x) (f y)
      Cmp op x y       -> Cmp op (f x) (f y)
      Calc r op x y    -> Calc (f r) op (f x) (f y)
      Count r s        -> Count (f r) (f s)
      Subquery s fnd w -> Subquery (f s) (map f fnd) (map go w)
      Or clauses       -> Or (map (map go) clauses)
      Absent cs        -> Absent (map go cs)
      Exists cs        -> Exists (map go cs)
    renameSentence s = tokensToSentence [ (intern (f name), op) | (name, op) <- tokens s ]
