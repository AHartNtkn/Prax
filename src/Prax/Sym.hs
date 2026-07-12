{-# LANGUAGE BangPatterns #-}

-- | Interned path segments (spec @docs/specs/2026-07-12-v29-interning.md@).
--
-- A process-global intern pool in the FastString style: within a run,
-- 'intern' is observably a pure function. No 'Sym' numeric identity may ever
-- be serialized, rendered into output, or compared across processes — all
-- observable text goes through 'symName'. Ids are first-encounter ordered
-- and therefore run-dependent by design.
--
-- Variable-ness is packed into id parity — a variable segment (uppercase
-- first letter, the engine's existing convention) interns ODD, a constant
-- EVEN — so 'symIsVar', the hottest predicate in unification, is a bit test.
module Prax.Sym
  ( Sym
  , intern
  , symName
  , symIsVar
  , symId
  , symOfId
  ) where

import           Data.Char (isUpper)
import           Data.IORef (IORef, newIORef, readIORef, atomicModifyIORef')
import qualified Data.IntMap.Strict as IntMap
import qualified Data.Map.Strict as Map
import           System.IO.Unsafe (unsafePerformIO)

newtype Sym = Sym Int
  deriving (Eq, Ord)

-- Debug rendering only; never parsed, never in goldens.
instance Show Sym where
  show s = "\8249" ++ symName s ++ "\8250"

data Pool = Pool !(Map.Map String Sym) !(IntMap.IntMap String) !Int !Int

poolRef :: IORef Pool
poolRef = unsafePerformIO (newIORef (Pool Map.empty IntMap.empty 0 1))
{-# NOINLINE poolRef #-}

-- | Intern a segment: total, idempotent, observably pure within a run.
--
-- __@name@ is forced to WHNF before the pool is touched__: a name built
-- from 'symName' (e.g. a re-interned rendered binding) is a thunk that
-- itself reads the pool; producing even its first character requires
-- 'symName'\'s whole @unsafePerformIO@ action to run to completion (an
-- already-known string comes back, not a further pool-dependent thunk).
-- Left lazy, that forcing would happen from /inside/ the
-- 'atomicModifyIORef'' callback below, nesting a second pool access inside
-- the first while the first's update is still in flight; forcing WHNF here
-- keeps every pool access strictly ordered, one at a time.
intern :: String -> Sym
intern !name =
  unsafePerformIO (atomicModifyIORef' poolRef step)
  where
    step pool@(Pool fwd rev nextE nextO) =
      case Map.lookup name fwd of
        Just s  -> (pool, s)
        Nothing ->
          let varish = case name of { (c : _) -> isUpper c; [] -> False }
              i  = if varish then nextO else nextE
              s  = Sym i
          in ( Pool (Map.insert name s fwd) (IntMap.insert i name rev)
                    (if varish then nextE else nextE + 2)
                    (if varish then nextO + 2 else nextO)
             , s )
{-# NOINLINE intern #-}

-- | The segment a symbol names. Loud error on a foreign id (impossible for
-- any 'Sym' produced by 'intern').
symName :: Sym -> String
symName (Sym i) = unsafePerformIO $ do
  Pool _ rev _ _ <- readIORef poolRef
  case IntMap.lookup i rev of
    Just n  -> pure n
    Nothing -> error ("symName: unknown symbol id " ++ show i)
{-# NOINLINE symName #-}

-- | Is this segment a logic variable? A bit test — see the module header.
symIsVar :: Sym -> Bool
symIsVar (Sym i) = odd i

-- | The raw interned id, for 'Prax.Db', which needs a machine 'Int' to key
-- its trie's 'Data.IntMap.Strict.IntMap'. __Map-keying escape hatch only__ —
-- never compare, serialize, or render this id; it is first-encounter ordered
-- and run-dependent, exactly like the 'Sym' it came from. Pair with
-- 'symOfId' to recover the 'Sym' when reading a key back out of the map.
symId :: Sym -> Int
symId (Sym i) = i

-- | Reconstruct a 'Sym' from a raw id — the read side of the map-keying
-- escape hatch (see 'symId'). The id must have originated from 'symId' on a
-- real 'Sym' (every id 'Prax.Db' ever stores as an 'IntMap' key satisfies
-- this); loud error on a foreign id, exactly as 'symName'.
--
-- __Bang pattern is load-bearing__: @i@ must be forced to WHNF before
-- 'poolRef' is read. Without it, @i@ is a lazy variable pattern, so a caller
-- passing an unforced @i@ derived from a fresh, not-yet-run 'intern' call
-- (e.g. @symOfId (symId (intern name))@) can have 'readIORef' race ahead of
-- that 'intern' call's pool write, observing a stale pool and failing on an
-- id that was, at that very moment, being assigned.
symOfId :: Int -> Sym
symOfId !i = unsafePerformIO $ do
  Pool _ rev _ _ <- readIORef poolRef
  case IntMap.lookup i rev of
    Just _  -> pure (Sym i)
    Nothing -> error ("symOfId: unknown symbol id " ++ show i)
{-# NOINLINE symOfId #-}
