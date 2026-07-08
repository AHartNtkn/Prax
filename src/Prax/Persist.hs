-- | Persistence: save/load a running session (Versu — "it is trivial to
-- serialize and deserialize the world state").
--
-- The whole mutable state is the fact database plus the turn cursor. Practices,
-- characters, and their wants are code (the world's rules), so a save captures
-- only @db@ + @cursor@ and is reloaded onto a freshly-constructed world of the
-- same kind. Serialization is just the fact sentences (one per line): because
-- @dbToSentences@ enumerates every leaf and @insertAll@ rebuilds the trie, the
-- round trip is exact (the @!@/@.@ distinction only matters at insert time, and
-- the exclusion has already been applied in the saved structure).
module Prax.Persist
  ( serializeState
  , deserializeState
  , saveState
  , loadState
  ) where

import           Text.Read (readMaybe)

import           Prax.Db (dbToSentences, emptyDb, insertAll)
import           Prax.Types (PraxState (..))

-- | Serialize the mutable state (@cursor@ + all facts) to text.
serializeState :: PraxState -> String
serializeState st = unlines (("cursor " ++ show (cursor st)) : dbToSentences (db st))

-- | Rebuild a saved state onto @world@ (a fresh world of the same kind, which
-- supplies the practice definitions and cast). Crashes loudly on malformed input.
deserializeState :: String -> PraxState -> PraxState
deserializeState text world =
  case lines text of
    (hd : rest)
      | ["cursor", n] <- words hd, Just c <- readMaybe n ->
          world { db = insertAll (filter (not . null) rest) emptyDb, cursor = c }
    _ -> error "Prax.Persist.deserializeState: malformed save (expected a 'cursor <n>' header)"

-- | Save a session to a file.
saveState :: FilePath -> PraxState -> IO ()
saveState path = writeFile path . serializeState

-- | Load a session from a file onto a fresh @world@.
loadState :: FilePath -> PraxState -> IO PraxState
loadState path world = deserializeState <$> readFile path <*> pure world
