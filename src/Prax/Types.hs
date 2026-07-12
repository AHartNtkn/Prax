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
import           Prax.Query (Condition)
import           Prax.Derive (Axiom)

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
  , characters   :: [Character]
  , cursor       :: Int          -- ^ round-robin index of the last actor
  , axioms       :: [Axiom]       -- ^ domain rules; reads see their forward-chained closure (default none)
  , sorts        :: [(String, [String])]  -- ^ sort → member constants, for the type checker (default none)
  , desires      :: [Desire]      -- ^ the vocabulary of nameable desires (default none)
  , predictionScope :: [Condition]  -- ^ conditions the planner predicts over (default none)
  , improvables :: [String]
    -- ^ Names of desires some authored action may improve
    -- ('Prax.Relevance.improvableDesires') — rebuilt with the vocabulary
    -- ('Prax.Engine.definePractices' / 'setAxioms' / 'setDesires'); the
    -- planner skips predictions over models with no improvable desire.
  , footprint :: [[String]]
    -- ^ Pre-tokenized ('pathNames') patterns the axioms read or write; a
    -- ground delta unifying none of them commutes with closure (fast path).
  , negFootprint :: [[String]]
    -- ^ Pre-tokenized negated body interiors: a '!'-free insert unifying
    -- none of these (in a 'contMonotone' world) only ADDS derived facts and
    -- takes the continuation tier.
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
  { db = emptyDb, practiceDefs = Map.empty, characters = [], cursor = -1
  , axioms = [], sorts = [], desires = [], predictionScope = []
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
