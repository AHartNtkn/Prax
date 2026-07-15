-- | Readable JSON (de)serialization for a "Prax.Script" play-script.
--
-- Rather than maintain a bespoke @.prompter@ grammar and parser, a play-script
-- round-trips through JSON: an author edits a @.json@ file and 'loadScript's it,
-- or dumps an existing 'Script' with 'encodeScript'. The schema is hand-written
-- (aeson's generic sum encoding is noisy) to stay legible — each condition and
-- effect is a single-key tagged object:
--
-- > { "start": "confidence",
-- >   "cast": [ { "name": "cassia", "playable": false,
-- >               "desires": [ { "utility": 100, "when": [ { "match": "dead.artus" } ] } ] } ],
-- >   "scenes": [ { "id": "confidence", "opening": "...",
-- >                 "beats": [ { "speaker": "cassia", "label": "...",
-- >                              "when": [ { "not": "confided" } ],
-- >                              "effects": [ { "insert": "confided" } ] } ],
-- >                 "junctions": [ { "name": "toBanquet", "to": "banquet",
-- >                                  "when": [ { "match": "confided" } ] } ] } ] }
--
-- The instances are (deliberately) orphans: keeping the JSON schema in one place
-- avoids coupling the core logic modules ("Prax.Query", "Prax.Types") to aeson.
{-# LANGUAGE OverloadedStrings #-}
{-# OPTIONS_GHC -Wno-orphans #-}
module Prax.Script.Json
  ( encodeScript
  , decodeScript
  , loadScript
  , saveScript
  ) where

import           Control.Applicative ((<|>))
import qualified Data.ByteString.Lazy as BL
import           Data.Aeson
import           Data.Aeson.Types (Parser)

import           Prax.Query (Condition (..), CmpOp (..), CalcOp (..))
import           Prax.Types (Outcome (..), Want (..))
import           Prax.Script

-- Public API ------------------------------------------------------------------

-- | Serialize a play-script to JSON (compact; pipe through @jq@ to pretty-print).
encodeScript :: Script -> BL.ByteString
encodeScript = encode

-- | Parse a play-script from JSON, reporting the error on failure.
decodeScript :: BL.ByteString -> Either String Script
decodeScript = eitherDecode

-- | Load and parse a play-script from a @.json@ file.
loadScript :: FilePath -> IO (Either String Script)
loadScript path = decodeScript <$> BL.readFile path

-- | Write a play-script to a @.json@ file.
saveScript :: FilePath -> Script -> IO ()
saveScript path = BL.writeFile path . encodeScript

-- Enum tags -------------------------------------------------------------------

cmpTag :: CmpOp -> String
cmpTag Lt = "lt"; cmpTag Lte = "lte"; cmpTag Gt = "gt"; cmpTag Gte = "gte"

parseCmp :: String -> Parser CmpOp
parseCmp "lt" = pure Lt; parseCmp "lte" = pure Lte
parseCmp "gt" = pure Gt; parseCmp "gte" = pure Gte
parseCmp s    = fail ("unknown comparison operator " ++ show s)

calcTag :: CalcOp -> String
calcTag Add = "add"; calcTag Sub = "sub"; calcTag Mul = "mul"; calcTag Mod = "mod"

parseCalc :: String -> Parser CalcOp
parseCalc "add" = pure Add; parseCalc "sub" = pure Sub; parseCalc "mul" = pure Mul
parseCalc "mod" = pure Mod
parseCalc s     = fail ("unknown calc operator " ++ show s)

-- Conditions ------------------------------------------------------------------

instance ToJSON Condition where
  toJSON c = case c of
    Match s               -> object [ "match"  .= s ]
    Not s                 -> object [ "not"    .= s ]
    Eq a b                -> object [ "eq"     .= [a, b] ]
    Neq a b               -> object [ "neq"    .= [a, b] ]
    Cmp op a b            -> object [ "cmp"    .= [cmpTag op, a, b] ]
    Calc r op a b         -> object [ "calc"   .= [r, calcTag op, a, b] ]
    Count r s             -> object [ "count"  .= [r, s] ]
    Subquery set find whr -> object [ "subquery" .=
                               object [ "set" .= set, "find" .= find, "where" .= whr ] ]
    Or clauses            -> object [ "or"     .= clauses ]
    Absent cs             -> object [ "absent" .= cs ]
    Exists cs             -> object [ "exists" .= cs ]

instance FromJSON Condition where
  parseJSON = withObject "Condition" $ \o ->
        (Match  <$> o .: "match")
    <|> (Not    <$> o .: "not")
    <|> triadOr2 o "eq"  Eq
    <|> triadOr2 o "neq" Neq
    <|> (do [op, a, b] <- o .: "cmp"; (\op' -> Cmp op' a b) <$> parseCmp op)
    <|> (do [r, op, a, b] <- o .: "calc"; (\op' -> Calc r op' a b) <$> parseCalc op)
    <|> triadOr2 o "count" Count
    <|> (o .: "subquery" >>= withObject "subquery"
           (\s -> Subquery <$> s .: "set" <*> s .: "find" <*> s .: "where"))
    <|> (Or     <$> o .: "or")
    <|> (Absent <$> o .: "absent")
    <|> (Exists <$> o .: "exists")
    where
      -- a two-element string array under key @k@, applied to a binary constructor
      triadOr2 o k f = do
        xs <- o .: k
        case xs of
          [a, b] -> pure (f a b)
          _      -> fail ("expected [a, b] under " ++ show k)

-- Outcomes --------------------------------------------------------------------

instance ToJSON Outcome where
  toJSON (Insert s)     = object [ "insert" .= s ]
  toJSON (Delete s)     = object [ "delete" .= s ]
  toJSON (Call fn args) = object [ "call" .= object [ "fn" .= fn, "args" .= args ] ]
  toJSON (ForEach conds outs) =
    object [ "forEach" .= object [ "when" .= conds, "do" .= outs ] ]

instance FromJSON Outcome where
  parseJSON = withObject "Outcome" $ \o ->
        (Insert <$> o .: "insert")
    <|> (Delete <$> o .: "delete")
    <|> (o .: "call" >>= withObject "call"
           (\c -> Call <$> c .: "fn" <*> c .: "args"))
    <|> (o .: "forEach" >>= withObject "forEach"
           (\f -> ForEach <$> f .: "when" <*> f .: "do"))

-- Wants -----------------------------------------------------------------------

instance ToJSON Want where
  toJSON (Want conds u) = object [ "when" .= conds, "utility" .= u ]

instance FromJSON Want where
  parseJSON = withObject "Want" $ \o -> Want <$> o .: "when" <*> o .: "utility"

-- Script AST ------------------------------------------------------------------

instance ToJSON CastMember where
  toJSON (CastMember n p ds ts) =
    object [ "name" .= n, "playable" .= p, "desires" .= ds, "traits" .= ts ]

instance FromJSON CastMember where
  parseJSON = withObject "CastMember" $ \o ->
    CastMember <$> o .: "name"
               <*> o .:? "playable" .!= False
               <*> o .:? "desires"  .!= []
               <*> o .:? "traits"   .!= []

instance ToJSON Beat where
  toJSON (Beat lbl spk cs es) =
    object $ [ "label" .= lbl, "when" .= cs, "effects" .= es ]
             ++ maybe [] (\s -> [ "speaker" .= s ]) spk

instance FromJSON Beat where
  parseJSON = withObject "Beat" $ \o ->
    Beat <$> o .:  "label"
         <*> o .:? "speaker"
         <*> o .:? "when"    .!= []
         <*> o .:? "effects" .!= []

instance ToJSON Junction where
  toJSON (Junction name to whn) =
    object $ [ "name" .= name, "when" .= whn ]
             ++ maybe [] (\t -> [ "to" .= t ]) to

instance FromJSON Junction where
  parseJSON = withObject "Junction" $ \o ->
    Junction <$> o .:  "name"
             <*> o .:? "to"
             <*> o .:? "when" .!= []

instance ToJSON Memory where
  toJSON (Memory t whn) = object [ "text" .= t, "when" .= whn ]

instance FromJSON Memory where
  parseJSON = withObject "Memory" $ \o -> Memory <$> o .: "text" <*> o .:? "when" .!= []

instance ToJSON Scene where
  toJSON (Scene sid op setup beats juncs mems) =
    object [ "id" .= sid, "opening" .= op, "setup" .= setup
           , "beats" .= beats, "junctions" .= juncs, "memories" .= mems ]

instance FromJSON Scene where
  parseJSON = withObject "Scene" $ \o ->
    Scene <$> o .:  "id"
          <*> o .:? "opening"   .!= ""
          <*> o .:? "setup"     .!= []
          <*> o .:? "beats"     .!= []
          <*> o .:? "junctions" .!= []
          <*> o .:? "memories"  .!= []

instance ToJSON Script where
  toJSON (Script cast scenes start) =
    object [ "start" .= start, "cast" .= cast, "scenes" .= scenes ]

instance FromJSON Script where
  parseJSON = withObject "Script" $ \o ->
    Script <$> o .: "cast" <*> o .: "scenes" <*> o .: "start"
