module Common exposing (..)

import Json.Decode exposing (Value)


type alias TreeData =
    { persons : Value, relationships : Value, grid : Value }
