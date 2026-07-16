-- | Persistence: save/load a running session (Versu — "it is trivial to
-- serialize and deserialize the world state").
--
-- The whole mutable state is the fact database, the turn cursor, and the
-- standing intentions. Practices, characters, and their wants are code (the
-- world's rules), so a save captures only @db@ + @cursor@ + @intentions@ and
-- is reloaded onto a freshly-constructed world of the same kind. Facts
-- serialize as sentences (one per line, via @dbToLabeledSentences@ /
-- @insertAll@); intentions serialize on their own labelled lines (below),
-- so the round trip is exact.
module Prax.Persist
  ( serializeState
  , deserializeState
  , saveState
  , loadState
  , formatVersion
  ) where

import           Data.List (isPrefixOf)
import qualified Data.Map.Strict as Map
import           Text.Read (readMaybe)

import           Prax.Db (Val (..), dbToLabeledSentences, emptyDb, insertAll)
import           Prax.Types (GroundedAction (..), Intention (..),
                              MotiveSignature (..), PraxState (..))
import           Prax.Engine (withDb)
import           Prax.Sym (intern, symName)

-- | A 'Val' crossing the file boundary as text, since interned symbols are
-- process-local (spec @docs/specs/2026-07-12-v29-interning.md@) — every
-- 'Prax.Sym.Sym' is written by name and re-interned on load.
data ValRepr = RS String | RN Integer | RT [[String]] deriving (Show, Read)

reprVal :: Val -> ValRepr
reprVal (VSym s)  = RS (symName s)
reprVal (VNum n)  = RN n
reprVal (VSet xs) = RT (map (map symName) xs)

unreprVal :: ValRepr -> Val
unreprVal (RS s)  = VSym (intern s)
unreprVal (RN n)  = VNum n
unreprVal (RT xs) = VSet (map (map intern) xs)

type GaRepr = (String, String, String, [(String, ValRepr)], String)
type IntentRepr = (Maybe GaRepr, ([String], [Int], [String], [(String, String)]))

reprIntention :: Intention -> IntentRepr
reprIntention (Intention mga sig) =
  ( fmap (\ga -> ( gaPracticeId ga, gaInstanceId ga, gaActionId ga
                 , [ (symName k, reprVal v) | (k, v) <- Map.toList (gaBindings ga) ]
                 , gaLabel ga )) mga
  , (msBearing sig, msSatisfaction sig, msLiveDesires sig, msKnownMotives sig) )

unreprIntention :: IntentRepr -> Intention
unreprIntention (mga, (b, s, l, m)) =
  Intention
    (fmap (\(pid, iid, aid, bs, lbl) -> GroundedAction pid iid aid
             (Map.fromList [ (intern k, unreprVal v) | (k, v) <- bs ]) lbl) mga)
    (MotiveSignature b s l m)

-- | The save-format tag, first line of every serialized state. Bump it when
-- the line format below changes; 'deserializeState' rejects anything else
-- loudly — no silent misparse of a save from another era.
formatVersion :: String
formatVersion = "prax-state v1"

-- | Serialize the mutable state (@cursor@ + standing intentions + all facts)
-- to text, with @!@/@.@ labels so the reload rebuilds the exclusion structure
-- exactly. Intention lines are prefixed @"intention "@; since fact sentences
-- are dot-paths with no spaces, that prefix cannot collide with a fact line
-- (the same disambiguation the @"cursor "@ header already relies on).
serializeState :: PraxState -> String
serializeState st =
  unlines
    ( formatVersion
    : ("cursor " ++ show (cursor st))
    : [ "intention " ++ name ++ " " ++ show (reprIntention i)
      | (name, i) <- Map.toList (intentions st) ]
    ++ dbToLabeledSentences (db st)
    )

-- | Rebuild a saved state onto @world@ (a fresh world of the same kind, which
-- supplies the practice definitions and cast). Crashes loudly on malformed
-- input, including a malformed intention line, and on a save from another
-- format era (a version tag other than 'formatVersion').
deserializeState :: String -> PraxState -> PraxState
deserializeState text world =
  case lines text of
    (v : hd : rest)
      | v == formatVersion, ["cursor", n] <- words hd, Just c <- readMaybe n ->
          let intentionLines = filter ("intention " `isPrefixOf`) rest
              factLines      = filter (not . ("intention " `isPrefixOf`)) rest
              newIntentions  = Map.fromList (map parseIntentionLine intentionLines)
          in (withDb (const (insertAll (filter (not . null) factLines) emptyDb)) world)
               { cursor = c, intentions = newIntentions }
    (v : _)
      | v /= formatVersion ->
          error ("Prax.Persist.deserializeState: unsupported save format "
                 ++ show v ++ " (expected " ++ show formatVersion ++ ")")
    _ -> error "Prax.Persist.deserializeState: malformed save (expected the format header, then 'cursor <n>')"
  where
    parseIntentionLine line =
      case break (== ' ') (drop (length "intention ") line) of
        (name, ' ' : reprText)
          | Just repr <- readMaybe reprText -> (name, unreprIntention repr)
        _ -> error ("Prax.Persist.deserializeState: malformed intention line: " ++ line)

-- | Save a session to a file.
saveState :: FilePath -> PraxState -> IO ()
saveState path = writeFile path . serializeState

-- | Load a session from a file onto a fresh @world@.
loadState :: FilePath -> PraxState -> IO PraxState
loadState path world = deserializeState <$> readFile path <*> pure world
