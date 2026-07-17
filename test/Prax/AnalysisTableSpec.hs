module Prax.AnalysisTableSpec (tests) where

import           Data.List (intercalate)
import qualified Data.Map.Strict as Map
import           Test.Tasty (TestTree, testGroup)
import           Test.Tasty.HUnit (testCase, (@?=))

import           Prax.Query (CookedCondition (..))
import           Prax.Sym (symName)
import           Prax.Types
import           Prax.Worlds.Audience (audienceWorld)
import           Prax.Worlds.Bar (barDirectorWorld, barWorld)
import           Prax.Worlds.Feud (feudWorld)
import           Prax.Worlds.Intrigue (intrigueWorld)
import           Prax.Worlds.Play (playWorld)
import           Prax.Worlds.Village (villageWorld)

-- | Render every derived analysis table a world state carries, one line per
-- entry, in the exact order the state holds them — order is part of the pin
-- (the v41 rewrite must reproduce the old walkers' emission order, not just
-- their sets). Sym paths render dot-joined via 'symName'. 'GateCheck' renders
-- its gates' 'CMatch' paths — the only shape 'Prax.Relevance.livenessOf'
-- emits; anything else crashes loudly here, deliberately.
analysisTable :: PraxState -> [String]
analysisTable st =
  [ "contMonotone: " ++ show (contMonotone st) ]
  ++ [ "improvable: " ++ n | n <- improvables st ]
  ++ [ "liveness: " ++ n ++ " " ++ renderL l
     | (n, l) <- Map.toList (liveness st) ]
  ++ [ "caresAbout: " ++ n ++ " -> " ++ intercalate "; " as
     | (n, as) <- Map.toList (caresAbout st) ]
  ++ [ "footprint: " ++ path p | p <- footprint st ]
  ++ [ "negFootprint: " ++ path p | p <- negFootprint st ]
  ++ [ "axiomHead: " ++ path p | p <- axiomHeads st ]
  where
    path = intercalate "." . map symName
    renderL l = case l of
      FloorCheck      -> "FloorCheck"
      AlwaysLive      -> "AlwaysLive"
      GateCheck gates -> "GateCheck " ++ intercalate " | " (map gate gates)
    gate [CMatch p] = path p
    gate g = error ("AnalysisTableSpec: unexpected gate shape: " ++ show g)

-- Captured from the live pre-v41 analyses (string-side walkers). These lines
-- ARE the analyses' contract across the v41 representation switch: the cooked
-- computation must reproduce every classification AND its order. Never edit
-- them to match new output — a failure means the rewrite is wrong.
tests :: TestTree
tests = testGroup "Prax.AnalysisTable"
  [ testCase "village"      $ analysisTable villageWorld     @?= villagePin
  , testCase "bar"          $ analysisTable barWorld         @?= barPin
  , testCase "bar-director" $ analysisTable barDirectorWorld @?= barDirectorPin
  , testCase "intrigue"     $ analysisTable intrigueWorld    @?= intriguePin
  , testCase "feud"         $ analysisTable feudWorld        @?= feudPin
  , testCase "audience"     $ analysisTable audienceWorld    @?= audiencePin
  , testCase "play"         $ analysisTable playWorld        @?= playPin
  ]

villagePin :: [String]
villagePin =
  [ "contMonotone: True"
  , "improvable: pursues-earnBread"
  , "improvable: spites-carol"
  , "improvable: punishes-whisper"
  , "improvable: suffers-hunger"
  , "improvable: drawn-to-market"
  , "improvable: smoulders"
  , "improvable: clean-conscience"
  , "liveness: clean-conscience FloorCheck"
  , "liveness: conscience-remembers FloorCheck"
  , "liveness: drawn-to-market GateCheck marketDay.square"
  , "liveness: punishes-whisper AlwaysLive"
  , "liveness: pursues-earnBread AlwaysLive"
  , "liveness: smoulders FloorCheck"
  , "liveness: spites-carol AlwaysLive"
  , "liveness: suffers-hunger FloorCheck"
  , "caresAbout: bob -> [Actor]: sweep the square; [Actor]: fetch flour from the mill; [Actor]: bake and earn the loaf; [Actor]: steal the loaf from the stall; [Actor]: tell [Hearer] that [Culprit] stole the loaf; [Actor]: return the loaf with apologies; [Actor]: whisper to [Hearer] that [Culprit] stole the loaf; [Actor]: take up honest work at the stall; [Actor]: eat the loaf; [Actor]: Go to [Place]"
  , "caresAbout: carol -> [Actor]: steal the loaf from the stall; [Actor]: confront [Thief] about the theft; [Actor]: tell [Hearer] that [Culprit] stole the loaf; [Actor]: shun [T]; [Actor]: relent toward [T]; [Actor]: whisper to [Hearer] that [Culprit] stole the loaf; [Actor]: confess to [Hearer] about framing [C]; [Actor]: threaten [V] with what you know; [Actor]: buy [E]'s silence; [Actor]: defy [E]; [Actor]: expose [V] to [Hearer]; [Actor]: Go to [Place]"
  , "caresAbout: dana -> [Actor]: confront [Thief] about the theft; [Actor]: eye [Thief] with suspicion; [Actor]: shun [T]; [Actor]: relent toward [T]; [Actor]: Go to [Place]"
  , "caresAbout: eve -> [Actor]: Go to [Place]"
  , "caresAbout: gale -> [Actor]: whisper to [Hearer] that [Culprit] stole the loaf; [Actor]: confess to [Hearer] about framing [C]; [Actor]: Go to [Place]"
  , "caresAbout: you -> "
  , "footprint: Regarder.believes.stole.Culprit.loaf"
  , "footprint: atoned.Culprit"
  , "footprint: regards.Regarder.Culprit.thief"
  , "footprint: regards.W0.T.thief"
  , "footprint: regards.W.T.thief"
  , "footprint: notorious.T.thief"
  , "footprint: Regarder.believes.swept.bob"
  , "footprint: Regarder.believes.desires.bob.pursues-earnBread.presumed"
  , "footprint: trait.M.T"
  , "footprint: traitDesire.T.D"
  , "footprint: character.P"
  , "footprint: P.believes.desires.M.D.presumed"
  , "footprint: Regarder.believes.whispered.V.H"
  , "footprint: recanted.V"
  , "footprint: regards.Regarder.V.slanderer"
  , "footprint: regards.W0.T.slanderer"
  , "footprint: regards.W.T.slanderer"
  , "footprint: notorious.T.slanderer"
  , "footprint: PraxW.believes.whispered.V.H0"
  , "footprint: PraxW.believes.whispered.V.H"
  , "footprint: regards.PraxW.V.incorrigible"
  , "footprint: obliged.Obligor.Regarder.believes.swept.bob"
  , "footprint: obliged.Obligor.Regarder.believes.desires.bob.pursues-earnBread.presumed"
  , "footprint: obliged.Obligor.trait.M.T"
  , "footprint: obliged.Obligor.traitDesire.T.D"
  , "footprint: obliged.Obligor.character.P"
  , "footprint: obliged.Obligor.P.believes.desires.M.D.presumed"
  , "negFootprint: atoned.Culprit"
  , "negFootprint: recanted.V"
  , "axiomHead: regards.Regarder.Culprit.thief"
  , "axiomHead: notorious.T.thief"
  , "axiomHead: Regarder.believes.desires.bob.pursues-earnBread.presumed"
  , "axiomHead: P.believes.desires.M.D.presumed"
  , "axiomHead: regards.Regarder.V.slanderer"
  , "axiomHead: notorious.T.slanderer"
  , "axiomHead: regards.PraxW.V.incorrigible"
  , "axiomHead: obliged.Obligor.Regarder.believes.desires.bob.pursues-earnBread.presumed"
  , "axiomHead: obliged.Obligor.P.believes.desires.M.D.presumed"
  , "axiomHead: contradiction"
  ]

barPin :: [String]
barPin =
  [ "contMonotone: True"
  , "caresAbout: ada -> [Actor]: Disapprove of [Offender]; [Actor]: turn [X] against [Y] to stir up the evening; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Go to [Place]"
  , "caresAbout: bex -> [Actor]: settle in, feeling you belong here; [Actor]: give up on the evening, resigning yourself to solitude; [Actor]: Disapprove of [Offender]; [Actor]: turn [X] against [Y] to stir up the evening; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Buy [Other] a drink; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Tip [Bartender]; [Actor]: Leave [Bartender]'s tab unpaid; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Drink the [Beverage]; [Actor]: Go to [Place]"
  , "caresAbout: director -> [Actor]: turn [X] against [Y] to stir up the evening"
  , "caresAbout: you -> "
  , "axiomHead: contradiction"
  ]

barDirectorPin :: [String]
barDirectorPin =
  [ "contMonotone: True"
  , "caresAbout: ada -> [Actor]: stir up a rivalry between [X] and [Y]; [Actor]: Disapprove of [Offender]; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Go to [Place]"
  , "caresAbout: bex -> [Actor]: settle in, feeling you belong here; [Actor]: give up on the evening, resigning yourself to solitude; [Actor]: stir up a rivalry between [X] and [Y]; [Actor]: Disapprove of [Offender]; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Buy [Other] a drink; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Tip [Bartender]; [Actor]: Leave [Bartender]'s tab unpaid; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Drink the [Beverage]; [Actor]: Go to [Place]"
  , "caresAbout: cai -> [Actor]: settle in, feeling you belong here; [Actor]: give up on the evening, resigning yourself to solitude; [Actor]: stir up a rivalry between [X] and [Y]; [Actor]: Disapprove of [Offender]; [Actor]: Greet [Other]; [Actor]: Strike up a conversation with [Other]; [Actor]: Buy [Other] a drink; [Actor]: Greet [Greeter] back; [Actor]: Rebuff [Greeter]; [Actor]: Take offense that [Greeted] ignored your greeting; [Actor]: Order [Beverage]; [Actor]: Fulfill [Customer]'s order; [Actor]: Drink the [Beverage]; [Actor]: Go to [Place]"
  , "caresAbout: director -> "
  , "axiomHead: contradiction"
  ]

intriguePin :: [String]
intriguePin =
  [ "contMonotone: True"
  , "improvable: kill-artus"
  , "liveness: kill-artus AlwaysLive"
  , "caresAbout: artus -> "
  , "caresAbout: cassia -> [Actor]: slip poison into [Target]'s cup; [Actor]: poison [Target] with your own hand"
  , "caresAbout: marcus -> "
  , "axiomHead: contradiction"
  ]

feudPin :: [String]
feudPin =
  [ "contMonotone: True"
  , "caresAbout: alice -> [Actor]: shun [Target]"
  , "caresAbout: bob -> [Actor]: shun [Target]"
  , "caresAbout: carol -> [Actor]: shun [Target]"
  , "caresAbout: dave -> [Actor]: shun [Target]"
  , "caresAbout: esme -> [Actor]: shun [Target]"
  , "footprint: allied.X.Y"
  , "footprint: allied.Y.X"
  , "footprint: wronged.X.Y"
  , "footprint: resents.Y.X"
  , "footprint: resents.A.B"
  , "footprint: allied.A.C"
  , "footprint: resents.C.B"
  , "footprint: member.X.F"
  , "footprint: member.Y.F"
  , "footprint: allied.X.Y"
  , "footprint: married.A.B"
  , "footprint: married.B.A"
  , "footprint: parent.P.X"
  , "footprint: parent.P.Y"
  , "footprint: sibling.X.Y"
  , "footprint: parent.G.P"
  , "footprint: parent.P.C"
  , "footprint: grandparent.G.C"
  , "footprint: married.A.B"
  , "footprint: parent.P.A"
  , "footprint: inLaw.P.B"
  , "footprint: married.A.B"
  , "footprint: sibling.A.S"
  , "footprint: inLaw.S.B"
  , "footprint: obliged.Obligor.allied.X.Y"
  , "footprint: obliged.Obligor.allied.Y.X"
  , "footprint: obliged.Obligor.wronged.X.Y"
  , "footprint: obliged.Obligor.resents.Y.X"
  , "footprint: obliged.Obligor.resents.A.B"
  , "footprint: obliged.Obligor.allied.A.C"
  , "footprint: obliged.Obligor.resents.C.B"
  , "footprint: obliged.Obligor.married.A.B"
  , "footprint: obliged.Obligor.married.B.A"
  , "footprint: obliged.Obligor.parent.G.P"
  , "footprint: obliged.Obligor.parent.P.C"
  , "footprint: obliged.Obligor.grandparent.G.C"
  , "footprint: obliged.Obligor.married.A.B"
  , "footprint: obliged.Obligor.parent.P.A"
  , "footprint: obliged.Obligor.inLaw.P.B"
  , "footprint: obliged.Obligor.married.A.B"
  , "footprint: obliged.Obligor.sibling.A.S"
  , "footprint: obliged.Obligor.inLaw.S.B"
  , "axiomHead: allied.Y.X"
  , "axiomHead: resents.Y.X"
  , "axiomHead: resents.C.B"
  , "axiomHead: allied.X.Y"
  , "axiomHead: married.B.A"
  , "axiomHead: sibling.X.Y"
  , "axiomHead: grandparent.G.C"
  , "axiomHead: inLaw.P.B"
  , "axiomHead: inLaw.S.B"
  , "axiomHead: obliged.Obligor.allied.Y.X"
  , "axiomHead: obliged.Obligor.resents.Y.X"
  , "axiomHead: obliged.Obligor.resents.C.B"
  , "axiomHead: obliged.Obligor.married.B.A"
  , "axiomHead: obliged.Obligor.grandparent.G.C"
  , "axiomHead: obliged.Obligor.inLaw.P.B"
  , "axiomHead: obliged.Obligor.inLaw.S.B"
  , "axiomHead: contradiction"
  ]

audiencePin :: [String]
audiencePin =
  [ "contMonotone: True"
  , "caresAbout: duke -> envoy: flatter the king; duke: flatter the king"
  , "caresAbout: envoy -> "
  , "caresAbout: king -> "
  , "axiomHead: contradiction"
  ]

playPin :: [String]
playPin =
  [ "contMonotone: True"
  , "caresAbout: artus -> "
  , "caresAbout: cassia -> cassia: slip poison into artus's cup; marcus: poison artus with your own hand"
  , "caresAbout: marcus -> "
  , "axiomHead: contradiction"
  ]
