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
  , Function(..)
  , FnCase(..)
  , CookedOutcome(..)
  , CookedAction(..)
  , CookedPractice(..)
  , Character(..)
  , character
  , Want(..)
  , Desire(..)
  , GroundedAction(..)
  , PraxState(..)
  , emptyState
  , deadSentence
  , livingCharacters
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Db (Bindings, Db, emptyDb, exists)
import           Prax.Query (Condition, CookedCondition)
import           Prax.Derive (Axiom, CookedRule)
import           Prax.Sym (Sym)

-- | A social practice: a role-parameterized bundle of affordances.
data Practice = Practice
  { practiceId   :: String        -- ^ unique id; the DB key under @practice.@
  , practiceName :: String        -- ^ display template (may contain @[Role]@s)
  , roles        :: [String]      -- ^ role variables; the instance key
  , actions      :: [Action]      -- ^ affordances offered to participants
  , dataFacts    :: [String]      -- ^ static facts, inserted under @practiceData.<id>.@
  , initOutcomes :: [Outcome]     -- ^ run once when an instance first spawns
  , functions    :: [Function]    -- ^ named guarded effect bundles, invoked by 'Call'
  }
  deriving (Eq, Show)

-- | A practice with everything empty; override fields with record syntax.
practice :: Practice
practice = Practice
  { practiceId = "", practiceName = "", roles = []
  , actions = [], dataFacts = [], initOutcomes = [], functions = [] }

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
  | Call String [String]     -- ^ invoke a practice 'Function' by name with args
  | ForEach [Condition] [Outcome]
    -- ^ Quantified effect: for /every/ binding of the conditions (evaluated
    -- against the closed view, snapshot at entry), apply the sub-outcomes.
  deriving (Eq, Show)

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

-- | The cooked mirror of 'Outcome' (see @docs/specs/2026-07-12-v28-cooked-world.md@,
-- @docs/specs/2026-07-12-v29-interning.md@ and 'Prax.Cooked.cookOutcome', which
-- builds these): 'Insert'/'Delete' carry the sentence already split into
-- @(symbol, punctuationAfterName)@ tokens ('Prax.Db.internTokens'); 'CCall'\'s
-- @fn@ stays a String (a @cpFns@ lookup key, never unified) while its @args@
-- are interned; 'CForEach' recurses. Declared here rather than in "Prax.Cooked"
-- because 'PraxState' below embeds 'CookedPractice' (built from these), and
-- "Prax.Cooked" depends on this module for 'Outcome'\/'Practice' — these are
-- pure mirror shapes with no dependency on "Prax.Cooked"; the conversion
-- functions ('Prax.Cooked.cookOutcome', 'Prax.Cooked.cookPractice') live there.
data CookedOutcome
  = CInsert [(Sym, Maybe Char)]
  | CDelete [(Sym, Maybe Char)]
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
  , cpFns     :: Map String ([String], [([CookedCondition], [CookedOutcome])])
    -- ^ Cooked 'Function' cases, keyed by 'fnName', paired with 'fnParams' —
    -- so the cooked hot path ('Prax.Engine.performCooked') never falls back
    -- to a string-side 'fnParams' lookup. First-wins on a duplicate 'fnName'
    -- within one practice (built via a fold that keeps the first occurrence,
    -- not @Map.fromList@'s last-wins), matching the search order
    -- 'lookupCookedFn' uses across practices.
  }
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

-- | All state a running simulation needs.
data PraxState = PraxState
  { db           :: Db
  , practiceDefs :: Map String Practice
  , cookedDefs   :: Map String CookedPractice
    -- ^ 'practiceDefs' compiled to cooked/token form
    -- ('Prax.Cooked.cookPractice'), rebuilt in lockstep by the same Engine
    -- helper ('Prax.Engine.retable') that maintains 'improvables'\/'footprint'.
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
  , axioms       :: [Axiom]       -- ^ domain rules; reads see their forward-chained closure (default none)
  , cookedRules  :: [CookedRule]
    -- ^ 'axioms' precompiled ('Prax.Derive.cookAxioms') — bodies pattern-
    -- split, heads pre-tokenized, □-lifted forms included. Maintained by
    -- 'Prax.Engine.setAxioms', consumed by 'Prax.Derive.runCooked' in
    -- 'Prax.Engine.reclose'\/'Prax.Engine.applyGrowToks' so the closure
    -- loop's ~5,400 calls\/round never re-cook the axiom set.
  , sorts        :: [(String, [String])]  -- ^ sort → member constants, for the type checker (default none)
  , desires      :: [Desire]      -- ^ the vocabulary of nameable desires (default none)
  , predictionScope :: [Condition]  -- ^ conditions the planner predicts over (default none)
  , improvables :: [String]
    -- ^ Names of desires some authored action may improve
    -- ('Prax.Relevance.improvableDesires') — rebuilt with the vocabulary
    -- ('Prax.Engine.definePractices' / 'setAxioms' / 'setDesires'); the
    -- planner skips predictions over models with no improvable desire.
  , footprint :: [[Sym]]
    -- ^ Pre-tokenized ('pathNames'), pre-interned patterns the axioms read
    -- or write; a ground delta unifying none of them commutes with closure
    -- (fast path).
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

-- | An empty interpreter state (cursor before the first actor).
emptyState :: PraxState
emptyState = PraxState
  { db = emptyDb, practiceDefs = Map.empty, cookedDefs = Map.empty, characters = []
  , cookedWants = Map.empty, cookedDesires = Map.empty, cursor = -1
  , axioms = [], cookedRules = [], sorts = [], desires = [], predictionScope = []
  , improvables = [], footprint = [], negFootprint = [], contMonotone = True
  , readView = emptyDb }

-- | Death (and eviction) are represented by the fact @dead.\<name\>@. A dead
-- character stays in the cast list but is skipped in turn-taking and lookahead.
deadSentence :: String -> String
deadSentence name = "dead." ++ name

-- | The characters still in play (not marked dead). Used by the turn loop and
-- the planner so a removed character neither acts nor is planned around.
livingCharacters :: PraxState -> [Character]
livingCharacters st =
  [ c | c <- characters st, not (exists (deadSentence (charName c)) (db st)) ]
