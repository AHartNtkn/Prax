module Prax.PlannerSpec (tests) where

import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=), assertBool, assertFailure)

import           Prax.Query
import           Prax.Types
import           Prax.Engine
import           Prax.Planner
import           Prax.Derive (axiom)

-- A minimal tend-bar practice (walk up + order) for planner tests.
tendBarP :: Practice
tendBarP = practice
  { practiceId = "tendBar"
  , practiceName = "[Bartender] is tending bar"
  , roles = ["Bartender"]
  , dataFacts =
      [ "beverageType.beer!alcoholic", "beverageType.cider!alcoholic"
      , "beverageType.soda!nonalcoholic" ]
  , actions =
      [ action "[Actor]: Walk up to bar"
          [ Neq "Actor" "Bartender"
          , Not "practice.tendBar.Bartender.customer.Actor" ]
          [ Insert "practice.tendBar.Bartender.customer.Actor" ]
      , action "[Actor]: Order [Beverage]"
          [ Match "practice.tendBar.Bartender.customer.Actor"
          , Not "practice.tendBar.Bartender.customer.Actor!beverage"
          , Match "practiceData.tendBar.beverageType.Beverage" ]
          [ Insert "practice.tendBar.Bartender.customer.Actor!order!Beverage" ]
      ]
  }

-- beth wants, above all, to have a cider on order.
bethWantsCider :: Character
bethWantsCider = (character "beth")
  { charWants = [ Want [ Match "practice.tendBar.Bartender.customer.beth!order!cider" ] 10 ] }

-- World with bar instance (ada tending) and beth present.
barState :: PraxState
barState =
  let st0 = setCharacters [bethWantsCider] (definePractices [tendBarP] emptyState)
  in performOutcome (Insert "practice.tendBar.ada") st0

-- After beth has walked up to the bar.
walkedUp :: PraxState
walkedUp = performOutcome (Insert "practice.tendBar.ada.customer.beth") barState

tests :: TestTree
tests = testGroup "Prax.Planner"
  [ testCase "evaluate sums utility over satisfying instantiations" $ do
      -- No cider order yet: want unsatisfied, utility 0.
      evaluate walkedUp (charWants bethWantsCider) @?= 0
      -- After ordering cider, the want is satisfied once: utility 10.
      let ordered = performOutcome
            (Insert "practice.tendBar.ada.customer.beth!order!cider") walkedUp
      evaluate ordered (charWants bethWantsCider) @?= 10

  , testCase "pickAction chooses the want-satisfying order over the alternatives" $ do
      -- beth can order beer, cider, or soda; only cider satisfies her want.
      let best = pickAction 0 walkedUp bethWantsCider
      fmap gaLabel best @?= Just "beth: Order cider"

  , testCase "top-scoring order beats the others; non-cider orders score 0" $ do
      let scored = scoreActions 0 walkedUp bethWantsCider
      -- Ordering cider is uniquely top with value 10.
      case scored of
        ((ga, s) : _) -> do gaLabel ga @?= "beth: Order cider"; s @?= 10.0
        []            -> assertBool "expected some actions" False
      -- Every non-cider action scores 0.
      let others = [ s | (ga, s) <- scored, gaLabel ga /= "beth: Order cider" ]
      all (== 0.0) others @?= True

  , testCase "lookahead: walking up is worthless immediately but valuable at depth 1" $ do
      -- score(walk) at depth 1 = eval(after walk = 0) + others (none believed: 0)
      --                        + 0.9 × best-next (order → 10) = 9.0   [§4 arithmetic]
      let scored = scoreActions 1 barState bethWantsCider
      lookup "beth: Walk up to bar" [ (gaLabel a, s) | (a, s) <- scored ] @?= Just 9.0
      fmap gaLabel (pickAction 1 barState bethWantsCider) @?= Just "beth: Walk up to bar"

  , testCase "a universally-quantified desire drives the planner to complete it" $ do
      -- A host who wants EVERY guest to have a drink (a ∀ desire), where one guest
      -- still lacks one. The planner should pour a drink for the one who needs it.
      let serveP = practice
            { practiceId = "serve", practiceName = "[Host] hosts", roles = ["Host"]
            , actions =
                [ action "[Actor]: pour a drink for [Guest]"
                    [ Match "guest.Guest", Not "hasDrink.Guest" ]
                    [ Insert "hasDrink.Guest" ]
                , action "[Actor]: rest" [] []   -- a 0-utility alternative
                ] }
          host = (character "host")
            { charWants = [ Want [ forAll [Match "guest.G"] [Match "hasDrink.G"] ] 10 ] }
          st0 = setCharacters [host] (definePractice serveP emptyState)
          st  = foldl (flip performOutcome) st0
                  [ Insert "guest.a", Insert "guest.b", Insert "hasDrink.a"  -- b lacks a drink
                  , Insert "practice.serve.host" ]
      -- the ∀ is currently unsatisfied (b has no drink), so it scores 0…
      evaluate st (charWants host) @?= 0
      -- …and the planner chooses to serve b, which completes "everyone has a drink".
      fmap gaLabel (pickAction 1 st host) @?= Just "host: pour a drink for b"
      pourForB <- case [ ga | ga <- possibleActions st "host"
                             , gaLabel ga == "host: pour a drink for b" ] of
                    (ga : _) -> pure ga
                    []       -> assertFailure "no 'pour a drink for b' action available to host"
      let served = performAction st pourForB
      evaluate served (charWants host) @?= 10   -- now the universal holds

  , testCase "predictMove is belief-relative: no belief, no prediction" $ do
      -- ada (a fresh character) holds no motive-beliefs about beth
      predictMove walkedUp (character "ada") bethWantsCider @?= Nothing

  , testCase "predictMove with a believed motive is the mover's motivated best" $ do
      let vocab = [ Desire "cider-craving"
                      (Want [ Match "practice.tendBar.Bartender.customer.Owner!order!cider" ] 10) ]
          beth' = (character "beth") { charDesires = ["cider-craving"] }
          st    = setDesires vocab (setCharacters [ beth', character "ada" ] walkedUp)
          st'   = performOutcome
                    (Insert "ada.believes.desires.beth.cider-craving.heard.gossip") st
      fmap gaLabel (predictMove st' (character "ada") beth') @?= Just "beth: Order cider"
      -- and motivated-only: a believed mind with nothing to gain predicts still
      let satisfied = performOutcome
                        (Insert "practice.tendBar.ada.customer.beth!order!cider") st'
      predictMove satisfied (character "ada") beth' @?= Nothing

  , testCase "a false belief predicts a move the mover would never take" $ do
      -- ada believes beth craves cider; beth actually wants nothing.
      let vocab = [ Desire "cider-craving"
                      (Want [ Match "practice.tendBar.Bartender.customer.Owner!order!cider" ] 10) ]
          plainBeth = character "beth"
          st  = setDesires vocab (setCharacters [ plainBeth, character "ada" ] walkedUp)
          st' = performOutcome
                  (Insert "ada.believes.desires.beth.cider-craving.presumed") st
      fmap gaLabel (predictMove st' (character "ada") plainBeth) @?= Just "beth: Order cider"
      pickAction 0 st' plainBeth @?= pickAction 0 st plainBeth   -- beth herself is unmoved

  , testCase "the round-walk credits a predicted enabling world (secret coordination)" $ do
      -- Conspirators: 'inge' will grab the relic once the gate is open (her
      -- desire is in the vocabulary); 'olaf' wants the relic grabbed and can
      -- open the gate. Olaf takes the enabling move IFF he is in on her motive.
      let vocab = [ Desire "covet-relic" (Want [ Match "grabbed.Owner" ] 10) ]
          grabP = practice
            { practiceId = "heist", roles = ["R"]
            , actions =
                [ action "[Actor]: grab the relic"
                    [ Match "gate.open", Not "grabbed.inge", Eq "Actor" "inge" ]
                    [ Insert "grabbed.inge" ]
                , action "[Actor]: open the gate"
                    [ Eq "Actor" "olaf", Not "gate.open" ]
                    [ Insert "gate.open" ]
                , action "[Actor]: Wait about"
                    [] [] ]
            }
          inge = (character "inge") { charDesires = ["covet-relic"] }
          olaf = (character "olaf") { charWants = [ Want [ Match "grabbed.inge" ] 6 ] }
          st0  = foldl (flip performOutcome)
                    (setDesires vocab
                       (setCharacters [ olaf, inge ]
                          (definePractices [grabP] emptyState)))
                    [ Insert "practice.heist.here" ]
          told = performOutcome
                   (Insert "olaf.believes.desires.inge.covet-relic.heard.inge") st0
      fmap gaLabel (pickAction 1 told olaf) @?= Just "olaf: open the gate"
      assertBool "not in on it: opening the gate gains him nothing"
        (fmap gaLabel (pickAction 1 st0 olaf) /= Just "olaf: open the gate")

  , testCase "the round is sequential: the second prediction sees the first's effects" $ do
      -- alice signals; bob (believed to want "relayed") relays once "signaled"
      -- holds; carol (believed to want "delivered") delivers once "relayed"
      -- holds — carol's precondition is only satisfiable because bob's move
      -- (the FIRST prediction in the round) was actually applied to the state
      -- the SECOND prediction sees. alice's own want is "delivered". The whole
      -- chain fires in one imagined round only when alice holds both
      -- motive-beliefs.
      let vocab = [ Desire "relay-desire"   (Want [ Match "relayed"   ] 10)
                  , Desire "deliver-desire" (Want [ Match "delivered" ] 10) ]
          chainP = practice
            { practiceId = "chain", roles = ["R"]
            , actions =
                [ action "[Actor]: signal"
                    [ Eq "Actor" "alice", Not "signaled" ]
                    [ Insert "signaled" ]
                , action "[Actor]: relay"
                    [ Eq "Actor" "bob", Match "signaled", Not "relayed" ]
                    [ Insert "relayed" ]
                , action "[Actor]: deliver"
                    [ Eq "Actor" "carol", Match "relayed", Not "delivered" ]
                    [ Insert "delivered" ]
                , action "[Actor]: Wait about"
                    [] [] ]
            }
          alice = (character "alice") { charWants = [ Want [ Match "delivered" ] 10 ] }
          bob   = character "bob"
          carol = character "carol"
          st0   = foldl (flip performOutcome)
                    (setDesires vocab
                       (setCharacters [ alice, bob, carol ]
                          (definePractices [chainP] emptyState)))
                    [ Insert "practice.chain.here" ]
          believeBoth = foldl (flip performOutcome) st0
            [ Insert "alice.believes.desires.bob.relay-desire.heard.gossip"
            , Insert "alice.believes.desires.carol.deliver-desire.heard.gossip" ]
          believeBobOnly = performOutcome
            (Insert "alice.believes.desires.bob.relay-desire.heard.gossip") st0
          scoreOf lbl st = lookup lbl [ (gaLabel a, s) | (a, s) <- scoreActions 1 st alice ]
      -- both beliefs held: bob relays (predicted first), then carol delivers
      -- (predicted second, seeing bob's "relayed") — the chain completes and
      -- "signal" is credited for opening it.
      scoreOf "alice: signal" believeBoth @?= Just 14.0
      fmap gaLabel (pickAction 1 believeBoth alice) @?= Just "alice: signal"
      -- only bob's motive believed: bob relays, but carol is never predicted
      -- (no belief) — delivery never happens, so signal earns no credit.
      scoreOf "alice: signal" believeBobOnly @?= Just 0.0
      -- neither belief: nobody is predicted to move at all.
      scoreOf "alice: signal" st0 @?= Just 0.0

  , testCase "prediction scope gates participation" $ do
      -- Reuse the heist: with a scope template requiring a shared room, and
      -- the conspirators placed in different rooms, olaf no longer credits
      -- inge's move even though he holds the motive-belief. Once they share a
      -- room the coordination fires exactly as in the default (empty, i.e.
      -- everyone-in-scope) case exercised above.
      let vocab = [ Desire "covet-relic" (Want [ Match "grabbed.Owner" ] 10) ]
          grabP = practice
            { practiceId = "heist", roles = ["R"]
            , actions =
                [ action "[Actor]: grab the relic"
                    [ Match "gate.open", Not "grabbed.inge", Eq "Actor" "inge" ]
                    [ Insert "grabbed.inge" ]
                , action "[Actor]: open the gate"
                    [ Eq "Actor" "olaf", Not "gate.open" ]
                    [ Insert "gate.open" ]
                , action "[Actor]: Wait about"
                    [] [] ]
            }
          inge = (character "inge") { charDesires = ["covet-relic"] }
          olaf = (character "olaf") { charWants = [ Want [ Match "grabbed.inge" ] 6 ] }
          sharedRoom = [ Match "at.Actor!Room", Match "at.Witness!Room" ]
          st0  = foldl (flip performOutcome)
                    (setDesires vocab
                       ((setCharacters [ olaf, inge ]
                          (definePractices [grabP] emptyState))
                          { predictionScope = sharedRoom }))
                    [ Insert "practice.heist.here" ]
          told = performOutcome
                   (Insert "olaf.believes.desires.inge.covet-relic.heard.inge") st0
          apart = foldl (flip performOutcome) told
                    [ Insert "at.olaf!gatehouse", Insert "at.inge!vault" ]
          together = foldl (flip performOutcome) told
                    [ Insert "at.olaf!vault", Insert "at.inge!vault" ]
      -- apart: the motive-belief is held, but inge is out of scope — no
      -- credit, so olaf does not bother opening the gate for her.
      assertBool "out of scope: opening the gate gains him nothing"
        (fmap gaLabel (pickAction 1 apart olaf) /= Just "olaf: open the gate")
      -- together: back in scope, the coordination fires as in the default
      -- (empty-scope, everyone-in-scope) case above.
      fmap gaLabel (pickAction 1 together olaf) @?= Just "olaf: open the gate"

  , testCase "the dead are predicted to do nothing" $ do
      -- ada believes beth craves cider; beth is alive with a motivated move
      -- available (order cider), so she is predicted to take it.
      let vocab = [ Desire "cider-craving"
                      (Want [ Match "practice.tendBar.Bartender.customer.Owner!order!cider" ] 10) ]
          beth' = (character "beth") { charDesires = ["cider-craving"] }
          st    = setDesires vocab (setCharacters [ beth', character "ada" ] walkedUp)
          st'   = performOutcome
                    (Insert "ada.believes.desires.beth.cider-craving.heard.gossip") st
      fmap gaLabel (predictMove st' (character "ada") beth') @?= Just "beth: Order cider"
      -- dead: the same belief and precondition hold, but a dead mover has no
      -- candidate actions, so there is nothing to predict.
      let dead = performOutcome (Insert (deadSentence "beth")) st'
      predictMove dead (character "ada") beth' @?= Nothing

  , testCase "a mid-round death silences the rest of the imagined round" $ do
      -- Reviewer's shape: alice's "signal" opens a round in which bob (predicted
      -- first) kills carol, and carol (predicted second) would otherwise
      -- deliver -- a move alice values. A corpse must not be credited.
      let vocab = [ Desire "kill-desire"    (Want [ Match "dead.carol" ] 10)
                  , Desire "deliver-desire" (Want [ Match "delivered"  ] 10) ]
          duelP = practice
            { practiceId = "duel", roles = ["R"]
            , actions =
                [ action "[Actor]: signal"
                    [ Eq "Actor" "alice", Not "signaled" ]
                    [ Insert "signaled" ]
                , action "[Actor]: kill carol"
                    [ Eq "Actor" "bob", Match "signaled", Not "dead.carol" ]
                    [ Insert "dead.carol" ]
                , action "[Actor]: deliver"
                    [ Eq "Actor" "carol", Match "signaled", Not "delivered" ]
                    [ Insert "delivered" ]
                , action "[Actor]: Wait about"
                    [] [] ]
            }
          alice = (character "alice") { charWants = [ Want [ Match "delivered" ] 10 ] }
          bob   = character "bob"
          carol = character "carol"
          st0   = foldl (flip performOutcome)
                    (setDesires vocab
                       (setCharacters [ alice, bob, carol ]
                          (definePractices [duelP] emptyState)))
                    [ Insert "practice.duel.here" ]
          believeBoth = foldl (flip performOutcome) st0
            [ Insert "alice.believes.desires.bob.kill-desire.heard.gossip"
            , Insert "alice.believes.desires.carol.deliver-desire.heard.gossip" ]
          scoreOf lbl st = lookup lbl [ (gaLabel a, s) | (a, s) <- scoreActions 1 st alice ]
      -- Round-walk after "signal": bob is predicted first (killing carol,
      -- which alone gains alice nothing -> +0.5*0), then carol is predicted.
      --
      -- WITH the ghost bug (carol's death not consulted before predicting
      -- her): carol is still predicted to "deliver" against a state that
      -- already has "dead.carol" -- possibleActions/candidateActions never
      -- filter on death. That predicted move inserts "delivered", crediting
      -- +0.5*10 = 5.0 to othersScore, and the resulting (ghost-inflated)
      -- afterRound state also feeds selfNext: scoreActions 0 there sees
      -- "delivered" already true, so selfNext = 0.9*10 = 9.0. Total:
      -- 0 (base after signal) + 5.0 + 9.0 = 14.0.
      --
      -- WITHOUT the ghost (correct): carol is dead by the time she would be
      -- predicted, so candidateActions returns [] for her and predictMove is
      -- Nothing -- she contributes nothing, and afterRound never gets
      -- "delivered". othersScore = 0, selfNext = 0.9*0 = 0. Total:
      -- 0 + 0 + 0 = 0.0.
      scoreOf "alice: signal" believeBoth @?= Just 0.0

  , testCase "deadNow (floor): a markless conscience skips; one lied-mark goes live" $ do
      -- A negative want-kind ("hates-lying") is at its floor whenever the
      -- mover carries no lied-mark at all: zero is the minimum, so no
      -- candidate (confess included) can improve it, and the pair-skip
      -- fires without grounding or scoring beth's candidates.
      let confessP = practice
            { practiceId = "confess", roles = ["R"]
            , actions =
                [ action "[Actor]: confess" [ Match "lied.Actor" ] [ Delete "lied.Actor" ]
                , action "[Actor]: Wait about" [] []
                ]
            }
          vocab = [ Desire "hates-lying" (Want [ Match "lied.Owner" ] (-5)) ]
          priya = character "priya"
          beth' = character "beth"
          st0   = setDesires vocab
                    (setCharacters [priya, beth'] (definePractices [confessP] emptyState))
          st1   = performOutcome (Insert "practice.confess.here") st0
          markless = performOutcome
            (Insert "priya.believes.desires.beth.hates-lying.heard.gossip") st1
      -- markless: "lied.beth" has zero bindings -- already at the floor.
      predictMove markless priya beth' @?= Nothing
      -- one lied-mark lifts the floor: confess strictly improves the desire
      -- (utility rises from -5 to 0), so the FULL model evaluates and finds it.
      let marked = performOutcome (Insert "lied.beth") markless
      fmap gaLabel (predictMove marked priya beth') @?= Just "beth: confess"

  , testCase "deadNow (gate): absent hunger skips; the hunger fact goes live" $ do
      -- A positive want-kind ("wants-food") gates on "hungry.Owner": no
      -- authored outcome inserts "hungry.*" (only "meal.*"), so it is a pure
      -- environment gate -- dead while absent, live once the fact appears.
      let eateryP = practice
            { practiceId = "eatery", roles = ["R"]
            , actions =
                [ action "[Actor]: eat lunch"
                    [ Match "practice.eatery.here" ] [ Insert "meal.Actor" ]
                , action "[Actor]: Wait about" [] []
                ]
            }
          vocab = [ Desire "wants-food" (Want [ Match "hungry.Owner", Match "meal.M" ] 5) ]
          priya = character "priya"
          beth' = character "beth"
          st0   = setDesires vocab
                    (setCharacters [priya, beth'] (definePractices [eateryP] emptyState))
          st1   = performOutcome (Insert "practice.eatery.here") st0
          believed = performOutcome
            (Insert "priya.believes.desires.beth.wants-food.heard.gossip") st1
      -- no hunger fact: the gate is empty, the pair skips.
      predictMove believed priya beth' @?= Nothing
      -- hunger fact present: the gate opens; eating creates the joint
      -- hungry+meal binding the FULL model needed to see the improvement.
      let hungry = performOutcome (Insert "hungry.beth") believed
      fmap gaLabel (predictMove hungry priya beth') @?= Just "beth: eat lunch"

  , testCase "deadNow (conservative): an axiom-derivable gate candidate never skips" $ do
      -- "hungry.Owner" looks environment-gated (no action inserts it
      -- directly), but an axiom derives it from "starving.Owner" -- the
      -- static classifier must exclude it from gating (livenessOf's
      -- conservativity), so the desire stays AlwaysLive and predictMove is
      -- never skipped by the state check, regardless of "hungry.beth"'s
      -- current truth.
      let toilP = practice
            { practiceId = "toil", roles = ["R"]
            , actions =
                [ action "[Actor]: toil" [] [ Insert "starving.Actor" ]
                , action "[Actor]: Wait about" [] []
                ]
            }
          vocab = [ Desire "craves-hunger" (Want [ Match "hungry.Owner" ] 5) ]
          axs   = [ axiom [ Match "starving.Owner" ] [ "hungry.Owner" ] ]
          priya = character "priya"
          beth' = character "beth"
          st0   = setDesires vocab
                    (setAxioms axs
                       (setCharacters [priya, beth'] (definePractices [toilP] emptyState)))
          st1   = performOutcome (Insert "practice.toil.here") st0
          believed = performOutcome
            (Insert "priya.believes.desires.beth.craves-hunger.heard.gossip") st1
      -- "hungry.beth" is absent, but the desire is conservatively AlwaysLive:
      -- toiling (which derives it via the axiom) is still predicted, exactly
      -- as it would be with no dead-now check at all.
      fmap gaLabel (predictMove believed priya beth') @?= Just "beth: toil"

  , testCase "deadNow gates the SKIP, never the model: a mixed model evaluates in FULL, dead deterrent included" $ do
      -- wants-treasure (AlwaysLive: its own condition is action-insertable,
      -- so it never gates) keeps this pair from EVER being skipped -- the
      -- point of this test is what happens once it's NOT skipped: does the
      -- floor-dead "hates-lying" still cost what it should, even though
      -- deadNow currently reads it as dead? "grab openly" (no lie) is
      -- unconditionally available; "grab boldly" (treasure + a NEW lie)
      -- only when clean; "confess" (removes an existing lie) only when
      -- marked -- so the two states offer genuinely different choices.
      let treasureP = practice
            { practiceId = "treasure", roles = ["R"]
            , actions =
                [ action "[Actor]: grab openly" [] [ Insert "has.Actor.treasure" ]
                , action "[Actor]: grab boldly" [ Not "lied.Actor" ]
                    [ Insert "has.Actor.treasure", Insert "lied.Actor" ]
                , action "[Actor]: confess" [ Match "lied.Actor" ] [ Delete "lied.Actor" ]
                , action "[Actor]: Wait about" [] []
                ]
            }
          vocab = [ Desire "wants-treasure" (Want [ Match "has.Owner.treasure" ] 5)
                  , Desire "hates-lying"    (Want [ Match "lied.Owner" ] (-6)) ]
          priya = character "priya"
          beth' = character "beth"
          st0   = setDesires vocab
                    (setCharacters [priya, beth'] (definePractices [treasureP] emptyState))
          st1   = performOutcome (Insert "practice.treasure.here") st0
          believed = foldl (flip performOutcome) st1
            [ Insert "priya.believes.desires.beth.wants-treasure.heard.gossip"
            , Insert "priya.believes.desires.beth.hates-lying.heard.gossip" ]
      -- markless: hates-lying is dead-now (floor), yet the FULL model still
      -- prices "grab boldly"'s NEW lie at -6 against "grab openly"'s clean
      -- +5 -- so the honest option wins (5 > 5-6=-1), not a tie. A buggy
      -- implementation that dropped a dead-now desire from the scored
      -- model (rather than only from the skip check) would score both
      -- grab options at 5 (a tie broken alphabetically toward "grab
      -- boldly") -- wrong. This is the discriminating half for THIS state.
      fmap gaLabel (predictMove believed priya beth') @?= Just "beth: grab openly"
      -- marked: hates-lying is now live (an existing, unrelated lie), which
      -- flips the decision -- confessing (relief worth 6) now beats
      -- grabbing openly (worth only 5), even though "grab boldly" itself is
      -- no longer on the table (Not "lied.Actor" fails). The SAME pair,
      -- the SAME never-skipped mixed model, a different answer because the
      -- deterrent's current state changed -- deadNow gated no skip here at
      -- all; the model's content did all the work, both times.
      let marked = performOutcome (Insert "lied.beth") believed
      fmap gaLabel (predictMove marked priya beth') @?= Just "beth: confess"
  ]
