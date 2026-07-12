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
intern :: String -> Sym
intern name = unsafePerformIO $ atomicModifyIORef' poolRef $
  \pool@(Pool fwd rev nextE nextO) ->
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
