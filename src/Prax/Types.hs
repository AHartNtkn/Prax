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
  , GroundedAction(..)
  , PraxState(..)
  , emptyState
  ) where

import           Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map

import           Prax.Db (Bindings, Db, emptyDb)
import           Prax.Query (Condition)

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
  , charBoundTo :: Maybe String   -- ^ restrict actions to this practice id
  }
  deriving (Eq, Show)

-- | A character with no wants and no binding.
character :: String -> Character
character n = Character { charName = n, charWants = [], charBoundTo = Nothing }

-- | A desire: a query whose every satisfying instantiation adds 'wantUtility'
-- to the utility of a candidate future world (Versu §IX-A).
data Want = Want
  { wantConditions :: [Condition]
  , wantUtility    :: Int
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
  }

-- | An empty interpreter state (cursor before the first actor).
emptyState :: PraxState
emptyState = PraxState
  { db = emptyDb, practiceDefs = Map.empty, characters = [], cursor = -1 }
