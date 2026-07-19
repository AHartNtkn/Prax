module Prax.ProjectSpec (tests) where

import           Control.Exception (ErrorCall, try)
import qualified Control.Exception as Exc (evaluate)
import           Data.Either (isLeft)
import           Data.List (isInfixOf)
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, assertBool, (@?=))

import           Prax.Db (exists)
import           Prax.Query (Condition (..), CmpOp (..))
import           Prax.Types
import           Prax.Engine (definePractices, performOutcome, possibleActions, performAction, setDesires, setCharacters)
import           Prax.Planner (pickAction, predictMove, evaluate)
import           Prax.Project

--------------------------------------------------------------------------------
-- Fixtures: endeavors as part-SETS (no cursor). Each names its parts, the
-- ledger keys; topology is authored two ways -- 'partAfter' (validated sibling
-- edges) and 'partNeeds' (world resources + threshold gates).
--------------------------------------------------------------------------------

-- A 4-part CHAIN: mia builds an oven, part after part; pat looks on. The edges
-- (fetch after sweep, shape after fetch, fire after shape) make it a chain
-- again, so it re-pins v24's horizon theorem on the new machinery.
chainParts :: [Part]
chainParts =
  [ Part "sweep" "[Actor]: sweep the hearth" [] [] []
  , Part "fetch" "[Actor]: fetch the clay"   ["sweep"]
      [ Match "clay.available" ] [ Insert "carrying.Owner.clay" ]
  , Part "shape" "[Actor]: shape the oven"   ["fetch"]
      [ Match "carrying.Owner.clay" ] [ Delete "carrying.Owner.clay" ]
  , Part "fire"  "[Actor]: fire it"          ["shape"] [] [ Insert "oven.standing" ]
  ]

ovenEnd :: (Action, Practice, Desire)
ovenEnd = endeavor "oven" 3 "[Actor]: resolve to build an oven" [] chainParts

ovenPursuit :: Desire
ovenPursuit = let (_, _, d) = ovenEnd in d

-- Two edge-free parts: nothing chains them, so both are live at once.
choresParts :: [Part]
choresParts =
  [ Part "dishes" "[Actor]: wash the dishes" [] [] [ Insert "washed.Owner" ]
  , Part "mop"    "[Actor]: mop the floor"   [] [] [ Insert "mopped.Owner" ]
  ]

choresEnd :: (Action, Practice, Desire)
choresEnd = endeavor "chores" 3 "[Actor]: set to the chores" [] choresParts

-- A culmination ('finish') that requires only 'gather'; 'flourish' hangs off
-- the side -- optional, never blocking, +w when taken.
feastParts :: [Part]
feastParts =
  [ Part "gather"   "[Actor]: gather the harvest"  []         [] [ Insert "gathered.Owner" ]
  , Part "flourish" "[Actor]: garnish the platter" []         [] [ Insert "garnished.Owner" ]
  , Part "finish"   "[Actor]: serve the feast"     ["gather"] [] [ Insert "served.Owner" ]
  ]

feastEnd :: (Action, Practice, Desire)
feastEnd = endeavor "feast" 3 "[Actor]: throw a feast" [] feastParts

-- Five parts; the culmination ('close') carries a THRESHOLD gate -- a Count
-- over the ledger family, fired at 3 of the workers done (not 2).
quotaParts :: [Part]
quotaParts =
  [ Part "a" "[Actor]: file report a" [] [] []
  , Part "b" "[Actor]: file report b" [] [] []
  , Part "c" "[Actor]: file report c" [] [] []
  , Part "d" "[Actor]: file report d" [] [] []
  , Part "close" "[Actor]: close the quota" []
      [ Subquery "Done" ["P"] [ Match "practice.quota.Owner.did.P" ]
      , Count "N" "Done"
      , Cmp Gte "N" "3" ]
      [ Insert "quota.met.Owner" ]
  ]

quotaEnd :: (Action, Practice, Desire)
quotaEnd = endeavor "quota" 3 "[Actor]: take on the quota" [] quotaParts

--------------------------------------------------------------------------------
-- The world scaffold: mia carries the pursuit; a bare "yard" practice hosts the
-- undertake action and an idle. pat is a bystander who owns no instance.
--------------------------------------------------------------------------------

buildWorld :: (Action, Practice, Desire) -> [Character] -> [Outcome] -> PraxState
buildWorld (take_, prac, pursuit) chars extra =
  foldl (flip performOutcome) base (Insert "practice.yard.here" : extra)
  where
    base = setDesires [ pursuit ]
             (setCharacters chars
                (definePractices [prac, yardP] emptyState))
    yardP = practice { practiceId = "yard", roles = ["R"]
                     , actions = [ take_, action "[Actor]: Idle about" [] [] ] }

miaFor :: String -> Character
miaFor pid = (character "mia") { charDesires = ["pursues-" ++ pid] }

ovenWorld :: PraxState
ovenWorld = buildWorld ovenEnd [ miaFor "oven", character "pat" ] [ Insert "clay.available" ]

choresWorld :: PraxState
choresWorld = buildWorld choresEnd [ miaFor "chores" ] []

feastWorld :: PraxState
feastWorld = buildWorld feastEnd [ miaFor "feast" ] []

quotaWorld :: PraxState
quotaWorld = buildWorld quotaEnd [ miaFor "quota" ] []

doAct :: String -> String -> PraxState -> PraxState
doAct who needle st =
  case [ ga | ga <- possibleActions st who, needle `isInfixOf` gaLabel ga ] of
    (ga : _) -> performAction st ga
    []       -> error ("no action for " ++ who ++ " matching " ++ show needle
                       ++ "; had: " ++ show (map gaLabel (possibleActions st who)))

offered :: String -> String -> PraxState -> Bool
offered who needle st =
  any ((needle `isInfixOf`) . gaLabel) (possibleActions st who)

-- Force a construction error to a Bool (did it error?), for the guard pins.
errored :: (Action, Practice, Desire) -> IO Bool
errored e = do
  r <- try (Exc.evaluate (length (show e)))
  pure (isLeft (r :: Either ErrorCall Int))

erroredWith :: String -> (Action, Practice, Desire) -> IO Bool
erroredWith needle e = do
  r <- try (Exc.evaluate (length (show e)))
  pure $ case r :: Either ErrorCall Int of
    Left err -> needle `isInfixOf` show err
    Right _  -> False

tests :: TestTree
tests = testGroup "Prax.Project"
  [ -- Property 1: local reward carries the horizon (v24's theorem, on edges).
    testCase "the horizon regression: a four-part chain pursued to completion at depth 2" $ do
      let mia = miaFor "oven"
          step st = case pickAction 2 st mia of
                      Just ga -> performAction st ga
                      Nothing -> error "mia has no move"
          st5 = iterate step ovenWorld !! 5   -- undertake + 4 parts
      assertBool "the chain completed to its culminating part"
        (exists "practice.oven.mia.did.fire" (db st5))

    -- Property 2: parallel parts are genuinely parallel.
  , testCase "parallel parts: both edge-free parts are offered; the un-chosen one survives" $ do
      let st0 = doAct "mia" "set to the chores" choresWorld
      assertBool "both edge-free parts offered at once"
        (offered "mia" "wash the dishes" st0 && offered "mia" "mop the floor" st0)
      let st1 = doAct "mia" "wash the dishes" st0
      assertBool "the completed part is not re-offered"
        (not (offered "mia" "wash the dishes" st1))
      assertBool "the un-chosen sibling is STILL offered (parts do not suppress siblings)"
        (offered "mia" "mop the floor" st1)

    -- Property 3: optional parts are optional.
  , testCase "an optional part: the culmination fires with it undone, and it still pays +w" $ do
      let st0 = doAct "mia" "gather the harvest" (doAct "mia" "throw a feast" feastWorld)
      assertBool "the culmination is offered though the optional part is undone"
        (offered "mia" "serve the feast" st0
         && not (exists "practice.feast.mia.did.flourish" (db st0)))
      let stFinish = doAct "mia" "serve the feast" st0
      assertBool "the culmination fired" (exists "practice.feast.mia.did.finish" (db stFinish))
      -- the optional still hangs off the side, and performing it pays +w
      assertBool "the optional part is still offered after the culmination"
        (offered "mia" "garnish the platter" stFinish)
      let want       = Want [ Match "practice.feast.Owner.did.P" ] 3
          before     = evaluate stFinish [ want ]
          stFlourish = doAct "mia" "garnish the platter" stFinish
          after      = evaluate stFlourish [ want ]
      after @?= before + 3

    -- Property 4: threshold success is authorable.
  , testCase "a threshold culmination fires at three of five, not two" $ do
      let st2 = doAct "mia" "file report b"
                  (doAct "mia" "file report a"
                    (doAct "mia" "take on the quota" quotaWorld))
      assertBool "two done: the threshold gate blocks the culmination"
        (not (offered "mia" "close the quota" st2))
      let st3 = doAct "mia" "file report c" st2
      assertBool "three done: the threshold gate opens the culmination"
        (offered "mia" "close the quota" st3)

    -- Property 5: dependencies gate loudly and correctly.
  , testCase "dependency gating: an unmet edge blocks; the met edge offers" $ do
      let st0 = doAct "mia" "resolve to build an oven" ovenWorld
      assertBool "the edge-free root is offered" (offered "mia" "sweep the hearth" st0)
      assertBool "the dependent part is blocked while its edge is unmet"
        (not (offered "mia" "fetch the clay" st0))
      let st1 = doAct "mia" "sweep the hearth" st0
      assertBool "the dependent part is offered once its edge is met"
        (offered "mia" "fetch the clay" st1)

    -- Property 6: each part once; teardown re-opens the whole endeavor.
  , testCase "each part fires once; the subtree delete re-opens the endeavor" $ do
      let st0 = doAct "mia" "sweep the hearth" (doAct "mia" "resolve to build an oven" ovenWorld)
      assertBool "the completed part is not re-offered within the instance"
        (not (offered "mia" "sweep the hearth" st0))
      assertBool "and undertaking is not re-offered while the instance stands"
        (not (offered "mia" "resolve to build an oven" st0))
      -- teardown: a subtree delete on the instance path reaps the ledger and
      -- re-opens undertake (the eat-cycle contract, in miniature)
      let torn = performOutcome (Delete "practice.oven.mia") st0
      assertBool "the ledger subtree is gone" (not (exists "practice.oven.mia.did.sweep" (db torn)))
      assertBool "undertaking is offered again" (offered "mia" "resolve to build an oven" torn)

    -- Property 7: dormancy and theory-of-mind survive.
  , testCase "the pursuit: exact shape, dormant (zero utility, no prediction), live once undertaken" $ do
      ovenPursuit @?= Desire "pursues-oven"
                        (Want [ Match "practice.oven.Owner.did.P" ] 3)
      -- instanceless: no ledger facts, so the pursuit scores zero
      evaluate ovenWorld [ let (Desire _ w) = ovenPursuit in w ] @?= 0
      -- dormant: pat believes mia pursues it, but with no instance the believed
      -- model gains nothing from any move -- no prediction.
      let mia  = miaFor "oven"
          told = performOutcome
                   (Insert "pat.believes.desires.mia.pursues-oven.heard.mia") ovenWorld
      predictMove told (character "pat") mia @?= Nothing
      -- undertaken: the same belief now predicts the next available part.
      let live = doAct "mia" "resolve to build an oven" told
      fmap gaLabel (predictMove live (character "pat") mia)
        @?= Just "mia: sweep the hearth"

  , testCase "a believed pursuit predicts among PARALLEL parts (v52 T1 review M1)" $ do
      -- both edge-free parts are live predictions; scoring ties (+3 each), so
      -- the deterministic label tiebreak picks the alphabetically-first label
      -- ("mop" < "wash") -- the point is that prediction needs no chain:
      -- whichever part the planner would take is the prediction, and BOTH are
      -- candidates.
      let mia  = miaFor "chores"
          w    = buildWorld choresEnd [ mia, character "pat" ] []
          told = performOutcome
                   (Insert "pat.believes.desires.mia.pursues-chores.heard.mia") w
          live = doAct "mia" "set to the chores" told
      fmap gaLabel (predictMove live (character "pat") mia)
        @?= Just "mia: mop the floor"
      -- and once the tiebreak winner is done, the prediction moves to the
      -- OTHER parallel part -- the un-chosen sibling was a real candidate.
      let after1 = doAct "mia" "mop the floor" live
      fmap gaLabel (predictMove after1 (character "pat") mia)
        @?= Just "mia: wash the dishes"

    -- Property 8: loud construction guards, one pin each.
  , testCase "an endeavor with no parts errors loudly" $
      errored (endeavor "idle" 1 "[Actor]: do nothing much" [] []) >>= assertBool "an endeavor is work"

  , testCase "a dotted project id errors loudly" $
      errored (endeavor "my.oven" 1 "[Actor]: x" [] chainParts)
        >>= assertBool "id must be a single path segment"

  , testCase "a dotted part name errors loudly" $
      errored (endeavor "oven" 1 "[Actor]: x" []
                 [ Part "sw.eep" "[Actor]: sweep" [] [] [] ])
        >>= assertBool "part name must be a single path segment"

  , testCase "duplicate part names error loudly" $
      errored (endeavor "oven" 1 "[Actor]: x" []
                 [ Part "dup" "[Actor]: one" [] [] []
                 , Part "dup" "[Actor]: two" [] [] [] ])
        >>= assertBool "part names must be distinct"

  , testCase "a dangling dependency edge errors loudly" $
      errored (endeavor "oven" 1 "[Actor]: x" []
                 [ Part "real" "[Actor]: real" ["ghost"] [] [] ])
        >>= assertBool "an edge must name an actual part"

  , testCase "a self-edge errors loudly (unreachable by the reachability fixpoint)" $
      erroredWith "unreachable" (endeavor "oven" 1 "[Actor]: x" []
                 [ Part "a" "[Actor]: a" ["a"] [] [] ])
        >>= assertBool "a self-dependent part is unreachable"

  , testCase "a two-cycle errors loudly (both participants unreachable)" $
      erroredWith "unreachable" (endeavor "oven" 1 "[Actor]: x" []
                 [ Part "a" "[Actor]: a" ["b"] [] []
                 , Part "b" "[Actor]: b" ["a"] [] [] ])
        >>= assertBool "a two-cycle has no edge-free root, so both are unreachable"

  , testCase "a Prax-namespace variable in partNeeds errors loudly" $
      errored (endeavor "oven" 1 "[Actor]: x" []
                 [ Part "x" "[Actor]: x" [] [ Match "PraxFoo.bar" ] [] ])
        >>= assertBool "the Prax namespace is reserved"
  ]
