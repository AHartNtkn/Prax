-- | Core data types for the simulation: practices, actions, outcomes,
-- characters, and the bundled interpreter state.
--
-- Practices/actions are authored as ordinary Haskell values (the eDSL). Smart
-- defaults ('practice', 'action') keep definitions terse — override fields with
-- record syntax, e.g.
--
-- > greet = practice
-- >   { practiceId = "greet", roles = ["Greeter", "Greeted"]
-- >   , actions = [ action "[Actor]: Greet [Other]"
-- >                   [Eq "Actor" "Greeter", Eq "Other" "Greeted"]
-- >                   [Delete "practice.greet.Actor.Other"] ] }
module Prax.Types
  ( Practice(..)
  , practice
  , Action(..)
  , action
  , Outcome(..)
  , outcomeVars
  , authoredVarClash
  , authoredPatClash
  , isPraxVar
  , Function(..)
  , FnCase(..)
  , ScheduleRule(..)
  , CookedScheduleRule(..)
  , turnPath
  , CookedOutcome(..)
  , CookedAction(..)
  , CookedPractice(..)
  , Liveness(..)
  , Character(..)
  , character
  , Want(..)
  , Desire(..)
  , GroundedAction(..)
  , MotiveSignature(..)
  , Intention(..)
  , PraxState(..)
  , emptyState
  , deadSentence
  , livingCharacters
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Db (Bindings, Db, emptyDb, exists, insert, isVariable, pathNames)
import           Prax.Query (Condition, CookedCondition, conditionVars)
import           Prax.Derive (Axiom, CookedRule)
import           Prax.Sym (Sym, intern)

-- | A social practice: a role-parameterized bundle of affordances.
data Practice = Practice
  { practiceId   :: String        -- ^ unique id; the DB key under @practice.@
  , practiceName :: String        -- ^ display template (may contain @[Role]@s)
  , roles        :: [String]      -- ^ role variables; the instance key
  , actions      :: [Action]      -- ^ affordances offered to participants
  , dataFacts    :: [String]      -- ^ static facts, inserted under @practiceData.<id>.@
  , initOutcomes :: [Outcome]     -- ^ run once when an instance first spawns
  }
  deriving (Eq, Show)

-- | A practice with everything empty; override fields with record syntax.
practice :: Practice
practice = Practice
  { practiceId = "", practiceName = "", roles = []
  , actions = [], dataFacts = [], initOutcomes = [] }

-- | An affordance: a named, conditioned bundle of outcomes.
data Action = Action
  { actionName       :: String       -- ^ display template; also the action's id
  , actionConditions :: [Condition]  -- ^ preconditions (a conjunctive query)
  , actionOutcomes   :: [Outcome]    -- ^ effects applied when performed
  }
  deriving (Eq, Show)

-- | Convenience constructor: @action name conditions outcomes@.
action :: String -> [Condition] -> [Outcome] -> Action
action = Action

-- | An effect on the world. Postconditions never need an explicit remove for
-- single-valued slots — the @!@ exclusion in 'Insert' handles that.
data Outcome
  = Insert String            -- ^ assert a sentence (may spawn a practice)
  | Delete String            -- ^ retract a subtree
  | InsertFor Int String
    -- ^ assert a sentence that EXPIRES: the engine retracts it (whole
    -- subtree, so lifetimes belong on leaf facts) @n@ round boundaries
    -- after this insert (spec @docs/specs/2026-07-16-v44-the-schedule.md@).
    -- Re-inserting the exact path with a lifetime refreshes the due;
    -- re-inserting it bare cancels it (v44's supersession law). Rides the
    -- whole cooked pipeline exactly as 'Insert' — the deferred retract is
    -- ENVIRONMENT, not a mover effect.
  | Call String [String]     -- ^ invoke a practice 'Function' by name with args
  | ForEach [Condition] [Outcome]
    -- ^ Quantified effect: for /every/ binding of the conditions (evaluated
    -- against the closed view, snapshot at entry), apply the sub-outcomes.
  deriving (Eq, Show)

-- | Every name an outcome /mentions/ — a total walk over every constructor,
-- 'ForEach' recursing through both its guard conditions ('conditionVars')
-- and its nested outcomes. Declared here (not alongside 'conditionVars' in
-- "Prax.Query") because 'Outcome' lives here and "Prax.Types" already
-- depends on "Prax.Query" — the reverse dependency would cycle. The shared
-- home for reserved-variable guards ('Prax.Engine.setSchedule',
-- 'Prax.Rng.draw') and similar checks that need every name an outcome could
-- touch.
outcomeVars :: Outcome -> [String]
outcomeVars o = case o of
  Insert s       -> pathNames s
  Delete s       -> pathNames s
  InsertFor _ s  -> pathNames s
  Call _ as      -> as
  ForEach cs os  -> concatMap conditionVars cs ++ concatMap outcomeVars os

-- | Names an author-supplied fragment MUST NOT use when a combinator splices
-- it into generated conditions (the v40 hygiene boundary). Two sources of
-- capture, one check: the @Prax@ namespace (ALL machinery variables live
-- there — see the spec; authors can never collide with them by accident) and
-- @forbiddenSplices@, names the combinator ITSELF also binds in the SAME
-- generated query (e.g. 'Prax.Confession.confess' grounds @Actor@\/@H@ — an
-- authored mark pattern reusing either would capture, so both are listed
-- here). @forbiddenSplices@ is a BLOCKLIST, not an allowlist: a name that is
-- neither Prax-prefixed nor in it is unrestricted — do not pass a
-- combinator's legitimate CONTRACT variables (e.g. 'Prax.Schedule.sightRule'\'s
-- @Seer@\/@Seen@\/@Spot@) expecting them to be exempted; the empty list @[]@
-- is how "nothing extra is forbidden" is spelled. Returns the offenders;
-- each combinator raises its own contextual error.
--
-- This walker and its siblings here are the AUTHORING-BOUNDARY side of the
-- v41 one-surface rule: they run at combinator boundaries, before cooking
-- exists, so they read the authored strings — authoring boundary = string;
-- world model = cooked ("Prax.Relevance").
authoredVarClash :: [String]      -- ^ forbiddenSplices: the combinator's OTHER bound names in this splice
                 -> [Condition] -> [Outcome] -> [String]
authoredVarClash forbiddenSplices conds outs =
  [ v | v <- vars, isPraxVar v || v `elem` forbiddenSplices ]
  where
    vars = filter isVariable (concatMap conditionVars conds
                              ++ concatMap outcomeVars outs)

-- | 'authoredVarClash' for string-pattern arguments that are not
-- 'Condition's (e.g. 'Prax.Confession.confess'\'s mark\/deposit patterns,
-- 'Prax.Faction.factionStanding'\'s, 'Prax.Blackmail.shakedown'\'s, and
-- 'Prax.Repute.standing'\'s evidence\/deed patterns): same two-source,
-- @forbiddenSplices@-is-a-blocklist check, over already-split
-- 'Prax.Db.pathNames' instead of 'conditionVars'\/'outcomeVars' — callers
-- extract the names themselves (and drop any they mean to exempt, e.g.
-- 'Prax.Blackmail.shakedown'\'s own victim variable) before calling.
authoredPatClash :: [String] -> [String] -> [String]
authoredPatClash forbiddenSplices names =
  [ v | v <- filter isVariable names, isPraxVar v || v `elem` forbiddenSplices ]

isPraxVar :: String -> Bool
isPraxVar ('P':'r':'a':'x':_:_) = True
isPraxVar _                     = False

-- | A named function: guarded conditional effects (used for e.g. win-condition
-- checks). The first case whose conditions hold runs; the rest are skipped.
data Function = Function
  { fnName   :: String
  , fnParams :: [String]
  , fnCases  :: [FnCase]
  }
  deriving (Eq, Show)

data FnCase = FnCase
  { caseConditions :: [Condition]
  , caseOutcomes   :: [Outcome]
  }
  deriving (Eq, Show)

-- | The engine's turn-counter path family — the one spelling of @turn@ in the
-- tree (spec @docs/specs/2026-07-16-v44-the-schedule.md@). The engine maintains
-- @turn!N@; schedule rule bodies may read it as a fact (@Match "turn!Now"@).
-- Seeded @turn!0@ in 'emptyState' itself, so every path (worlds, Script
-- compile, fixtures) has a clock before anything reads it.
turnPath :: String
turnPath = "turn"

-- | One recurring engine-schedule rule (spec v44): every 'srPeriod' round
-- boundaries, ground each clause's conditions against the world and apply its
-- outcomes per binding. Bodies may read the clock ('turnPath') as a fact.
data ScheduleRule = ScheduleRule
  { srName   :: String   -- ^ single segment; the persist re-association key
  , srPeriod :: Int      -- ^ round boundaries between firings (authored meaning)
  , srBody   :: [([Condition], [Outcome])]
  }
  deriving (Eq, Show)

-- | The cooked mirror of 'Outcome' (see @docs/specs/2026-07-12-v28-cooked-world.md@,
-- @docs/specs/2026-07-12-v29-interning.md@ and 'Prax.Cooked.cookOutcome', which
-- builds these): 'Insert'/'Delete' carry the sentence already split into
-- @(symbol, punctuationAfterName)@ tokens ('Prax.Db.internTokens'); 'CCall'\'s
-- @fn@ stays a String (a @cookedFns@ registry lookup key, never unified) while its @args@
-- are interned; 'CForEach' recurses. Declared here rather than in "Prax.Cooked"
-- because 'PraxState' below embeds 'CookedPractice' (built from these), and
-- "Prax.Cooked" depends on this module for 'Outcome'\/'Practice' — these are
-- pure mirror shapes with no dependency on "Prax.Cooked"; the conversion
-- functions ('Prax.Cooked.cookOutcome', 'Prax.Cooked.cookPractice') live there.
data CookedOutcome
  = CInsert [(Sym, Maybe Char)]
  | CDelete [(Sym, Maybe Char)]
  | CInsertFor Int [(Sym, Maybe Char)]
    -- ^ the cooked mirror of 'InsertFor': assert the tokens and enqueue a
    -- retract @n@ round boundaries out ('Prax.Engine.performCooked').
  | CCall String [Sym]
  | CForEach [CookedCondition] [CookedOutcome]
  deriving (Eq, Show)

-- | The cooked mirror of 'Action': 'actionConditions'\/'actionOutcomes' precooked;
-- 'caName' carries 'actionName' unchanged (a display template, not a pattern —
-- never re-parsed by the query/unify machinery, so it needs no cooking).
data CookedAction = CookedAction
  { caName  :: String
  , caConds :: [CookedCondition]
  , caOuts  :: [CookedOutcome]
  }
  deriving (Eq, Show)

-- | The cooked mirror of 'Practice': everything 'Prax.Engine.possibleActions'
-- and 'Prax.Engine.performAction' need, precompiled once by
-- 'Prax.Cooked.cookPractice' and cached in 'PraxState''s 'cookedDefs'.
data CookedPractice = CookedPractice
  { cpInstanceNames :: [Sym]
    -- ^ Interned 'Prax.Db.pathNames' of @practice.\<pid\>.\<Role1\>...\<RoleN\>@,
    -- precomputed once per world instead of re-split (and re-interned) every
    -- 'possibleActions' call.
  , cpActions :: [CookedAction]
  , cpInits   :: [CookedOutcome]
  }
  deriving (Eq, Show)

-- | The cooked mirror of 'ScheduleRule' ('Prax.Cooked.cookScheduleRule'):
-- body clauses precooked, the name carried unchanged (the persist
-- re-association key and the string-side 'srPeriod' lookup key). Lives on its
-- own cooked surface ('PraxState''s 'cookedSchedule'), NOT in 'cookedDefs':
-- movers cannot take schedule rules, they appear in no candidate set.
data CookedScheduleRule = CookedScheduleRule
  { csrName :: String
  , csrBody :: [([CookedCondition], [CookedOutcome])]
  }
  deriving (Eq, Show)

-- | Per-desire state-check recipe for the planner's dead-now test (spec
-- @docs/specs/2026-07-13-v33-live-relevance.md@), computed once per world by
-- 'Prax.Relevance.livenessOf' (declared here, alongside 'CookedPractice',
-- because 'PraxState' embeds it — the same rationale as that type's own
-- haddock). 'FloorCheck': a negative want-kind is at its floor (unimprovable
-- by anything) when its own Owner-grounded conditions have zero bindings —
-- sound unconditionally, count zero is the minimum. 'GateCheck': a positive
-- want-kind cannot gain a binding while any environment-gated conjunct (a
-- base-fact family no authored outcome inserts and no axiom derives) is
-- empty. 'AlwaysLive': no cheap state test applies — the static verdict
-- stands alone.
--
-- Conservativity note: gates only ever REMOVE work when provably safe;
-- @Or@\/@Absent@\/@Exists@ conjuncts are never gates (only top-level
-- positive @Match@es), and any uncertainty (an axiom-derivable candidate, an
-- unresolvable outcome, a @Subquery@\/@Count@\/@Calc@-bearing want) keeps
-- the desire 'AlwaysLive'.
data Liveness
  = FloorCheck                    -- ^ negative: check the desire's own conditions
  | GateCheck [[CookedCondition]] -- ^ positive: each inner list is ONE gate
                                   -- conjunct (cooked, Owner-templated)
  | AlwaysLive
  deriving (Eq, Show)

-- | A character/agent. Wants drive autonomous choice; a practice-bound character
-- only acts within its bound practice (e.g. an ambient jukebox).
data Character = Character
  { charName    :: String
  , charWants   :: [Want]
  , charDesires :: [String]       -- ^ names of vocabulary 'Desire's this character holds
  , charBoundTo :: Maybe String   -- ^ restrict actions to this practice id
  }
  deriving (Eq, Show)

-- | A character with no wants, no desires, and no binding.
character :: String -> Character
character n = Character
  { charName = n, charWants = [], charDesires = [], charBoundTo = Nothing }

-- | A desire: a query whose every satisfying instantiation adds 'wantUtility'
-- to the utility of a candidate future world (Versu §IX-A).
data Want = Want
  { wantConditions :: [Condition]
  , wantUtility    :: Int
  }
  deriving (Eq, Show)

-- | A nameable desire: a 'Want' whose conditions may use the reserved variable
-- @Owner@, instantiated per character ('Prax.Minds.wantFor'). Naming a desire is
-- what makes it a possible object of belief.
data Desire = Desire
  { desireName :: String
  , desireWant :: Want
  }
  deriving (Eq, Show)

-- | A fully grounded, performable action produced by the engine.
data GroundedAction = GroundedAction
  { gaPracticeId :: String
  , gaInstanceId :: String
  , gaActionId   :: String     -- ^ the originating 'actionName' (lookup key)
  , gaBindings   :: Bindings   -- ^ Actor + role + query bindings for grounding
  , gaLabel      :: String     -- ^ rendered display text
  }
  deriving (Eq, Show)

-- | The inputs to /wanting to reconsider/ (spec
-- @docs/specs/2026-07-13-v35-intentions.md@, as amended): what I can do
-- THAT I CARE ABOUT (want-bearing templates — full-grounding equality made
-- the mechanism inert, measured; staleness is prevented by npcAct's
-- standing-offered check instead), how I am doing (per-want satisfaction
-- counts, 'selfWants' order — counts, not their weighted sum, so two
-- profiles cannot mask each other), what is driving me (own live desires:
-- statically improvable and not dead-now), and what motives I know of
-- (believed-motive facts, the family "Prax.Minds" reads). Compared for
-- equality at the character's own turn against their last deliberation.
data MotiveSignature = MotiveSignature
  { msBearing      :: [String]
    -- ^ the /want-bearing/ affordance templates currently offered: action
    -- identities both grounded now ('Prax.Planner.candidateActions') and
    -- listed in the character's 'bearing' table. Opportunities that touch
    -- what I care about interrupt me; irrelevant comings and goings do not.
    -- (Standing-action validity is checked separately by 'Prax.Loop.npcAct'
    -- — a movement pick is rarely bearing but must still expire when acted.)
  , msSatisfaction :: [Int]
  , msLiveDesires  :: [String]
  , msKnownMotives :: [(String, String)]  -- (other, desire name)
  }
  deriving (Eq, Show)

-- | A standing intention: the action chosen at the last deliberation (or
-- the choice to do nothing) and the motive signature it was based on.
data Intention = Intention
  { intentAct   :: Maybe GroundedAction
  , intentBasis :: MotiveSignature
  }
  deriving (Eq, Show)

-- | All state a running simulation needs.
data PraxState = PraxState
  { db           :: Db
  , practiceDefs :: Map String Practice
  , cookedDefs   :: Map String CookedPractice
    -- ^ 'practiceDefs' compiled to cooked/token form
    -- ('Prax.Cooked.cookPractice'), rebuilt in lockstep by the same Engine
    -- helper ('Prax.Engine.retable') that maintains 'improvables'\/'footprint'.
  , worldFns  :: [Function]                 -- ^ the authored registry (build-time vocabulary)
  , cookedFns :: Map String ([String], [([CookedCondition], [CookedOutcome])])
    -- ^ cooked once by 'Prax.Engine.defineFunctions'; the ONE home 'Call'
    -- resolution reads ('Prax.Engine.lookupCookedFn'). Keyed by 'fnName';
    -- uniqueness is the setter's loud guard, so the Map never silently
    -- collapses a duplicate. Functions carry no practice locality — resolution
    -- was always global (spec 2026-07-17-v47).
  , characters   :: [Character]
  , cookedWants :: Map String [[CookedCondition]]
    -- ^ Each character's 'charWants' conditions precooked, one entry per
    -- want, same order as 'charWants' — keyed by 'charName'. Maintained by
    -- 'Prax.Engine.setCharacters' (retable); paired with 'charWants''
    -- utilities by construction (same source list, same order — never
    -- re-sorted or filtered independently).
  , cookedDesires :: Map String [CookedCondition]
    -- ^ Each vocabulary 'Desire''s (Owner-templated) conditions precooked
    -- once — keyed by 'desireName', independent of which characters hold it.
    -- Maintained by 'Prax.Engine.retable' alongside 'cookedDefs'.
  , cursor       :: Int          -- ^ round-robin index of the last actor
  , caresAbout :: Map String [String]
    -- ^ Per character, the affordance templates whose authored outcomes
    -- could touch their own wants or desires
    -- ('Prax.Relevance.bearingTemplates') — the opportunity-relevance half
    -- of 'MotiveSignature'. Maintained by 'Prax.Engine.retable'.
  , intentions :: Map String Intention
    -- ^ Each character's standing intention ('Prax.Loop.npcAct' — the
    -- reconsideration semantics, spec 2026-07-13-v35). Runtime state like
    -- 'cursor': starts empty (every character's first turn deliberates),
    -- never derived, never touched by 'Prax.Engine.retable'.
  , axioms       :: [Axiom]       -- ^ domain rules; reads see their forward-chained closure (default none)
  , cookedRules  :: [CookedRule]
    -- ^ 'axioms' precompiled ('Prax.Derive.cookAxioms') — bodies pattern-
    -- split, heads pre-tokenized, □-lifted forms included. Maintained by
    -- 'Prax.Engine.setAxioms', consumed by 'Prax.Derive.runCooked' in
    -- 'Prax.Engine.reclose'\/'Prax.Engine.applyGrowToks' so the closure
    -- loop's ~5,400 calls\/round never re-cook the axiom set.
  , sorts        :: [(String, [String])]  -- ^ sort → member constants, for the type checker (default none)
  , desires      :: [Desire]      -- ^ the vocabulary of nameable desires (default none)
  , schedule       :: [ScheduleRule]
    -- ^ Authored recurring-rule declarations (spec v44); declaration order is
    -- firing order. Default none. Cooked into 'cookedSchedule' by
    -- 'Prax.Engine.retable'.
  , cookedSchedule :: [CookedScheduleRule]
    -- ^ 'schedule' compiled ('Prax.Cooked.cookScheduleRule'), maintained by
    -- 'Prax.Engine.retable'. Its own cooked surface, NOT in 'cookedDefs'.
  , scheduleDues   :: Map String Int
    -- ^ Per recurring rule, the next round boundary it fires at (keyed by
    -- 'csrName'\/'srName'). Runtime state like 'cursor'; re-associated by name
    -- on load ('Prax.Persist').
  , expiries       :: Map [(Sym, Maybe Char)] Int
    -- ^ The one-shot expiry queue (spec v44): exact labeled path → the round
    -- boundary at which the engine retracts it. Keyed by EXACT LABELED PATH so
    -- refresh and cancel find entries by path. Runtime state.
  , predictionScope :: [Condition]  -- ^ conditions the planner predicts over (default none)
  , improvables :: [String]
    -- ^ Names of desires some authored action may improve
    -- ('Prax.Relevance.improvableDesires') — rebuilt with the vocabulary
    -- ('Prax.Engine.definePractices' / 'setAxioms' / 'setDesires'); the
    -- planner skips predictions over models with no improvable desire.
  , liveness :: Map String Liveness
    -- ^ Each named desire's dead-now recipe ('Liveness') —
    -- ('Prax.Relevance.livenessOf') — rebuilt with the vocabulary alongside
    -- 'improvables' by 'Prax.Engine.retable'; the planner consults it to
    -- skip state-dead predictions for desires the static filter has already
    -- let through.
  , footprint :: [[Sym]]
    -- ^ Pre-tokenized ('pathNames'), pre-interned patterns the axioms read
    -- or write; a ground delta unifying none of them commutes with closure
    -- (fast path).
  , axiomHeads :: [[Sym]]
    -- ^ Interned anchors of every axiom head that can fire
    -- ('Prax.Derive.axiomHeadPatterns') plus the @contradiction@ witness
    -- ('Prax.Engine.reclose' inserts it when a delta trips ⊥). Maintained by
    -- 'Prax.Engine.retable'; consumed by the planner's prediction-reuse cone:
    -- a path delta that feeds any axiom ('footprint') can change derived
    -- facts only in these families.
  , negFootprint :: [[Sym]]
    -- ^ Pre-tokenized, pre-interned negated body interiors: a '!'-free
    -- insert unifying none of these (in a 'contMonotone' world) only ADDS
    -- derived facts and takes the continuation tier.
  , contMonotone :: Bool
    -- ^ 'Prax.Derive.monotoneAxioms' of this world's axioms.
  , readView     :: Db
    -- ^ The db closed under the axioms — established (lazily) whenever the
    -- state is built, so reads share one closure per state. Change 'db' or
    -- 'axioms' ONLY through 'Prax.Engine.withDb' / 'Prax.Engine.setAxioms',
    -- which rebuild it; a raw record update of either leaves this stale.
  }

-- | An empty interpreter state (cursor before the first actor). The engine's
-- clock is seeded @turn!0@ here — construction, not world setup — so every
-- path (worlds, Script compile, fixtures) carries a clock before anything
-- reads it (spec v44).
emptyState :: PraxState
emptyState = PraxState
  { db = insert (turnPath ++ "!0") emptyDb
  , practiceDefs = Map.empty, cookedDefs = Map.empty
  , worldFns = [], cookedFns = Map.empty, characters = []
  , cookedWants = Map.empty, cookedDesires = Map.empty, cursor = -1
  , caresAbout = Map.empty, intentions = Map.empty
  , axioms = [], cookedRules = [], sorts = [], desires = [], predictionScope = []
  , schedule = [], cookedSchedule = [], scheduleDues = Map.empty, expiries = Map.empty
  , improvables = [], liveness = Map.empty, footprint = []
  , axiomHeads = [[intern "contradiction"]]
  , negFootprint = [], contMonotone = True
  , readView = insert (turnPath ++ "!0") emptyDb }

-- | Death (and eviction) are represented by the fact @dead.\<name\>@. A dead
-- character stays in the cast list but is skipped in turn-taking and lookahead.
deadSentence :: String -> String
deadSentence name = "dead." ++ name

-- | The characters still in play (not marked dead). Used by the turn loop and
-- the planner so a removed character neither acts nor is planned around.
livingCharacters :: PraxState -> [Character]
livingCharacters st =
  [ c | c <- characters st, not (exists (deadSentence (charName c)) (db st)) ]
