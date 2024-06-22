module Data exposing (..)

import Dict exposing (Dict)
import Element exposing (Color)


type alias TreeData =
    { persons : List Person, relationships : List Relationship, grid : Grid }


type alias Relationship =
    { id : Rid
    , parents : ( Maybe Pid, Maybe Pid )
    , children : List Pid
    }


type alias Person =
    { id : Pid
    , info : Dict String String
    }


type alias Pid =
    String


type alias Grid =
    List (List GridItem)


type GridItem
    = PersonItem Pid
    | ConnectionsItem Connections


type alias Rid =
    String


type alias Connections =
    { orientation : Orientation
    , passing : List Passing
    , ending : List Ending
    , crossing : List Crossing
    }


type alias Passing =
    { rid : Rid
    , color : Color
    , yFraction : Fraction
    }


type alias Ending =
    { rid : Rid
    , color : Color
    , origin : Origin
    , xFraction : Fraction
    , yFraction : Fraction
    }


type alias Crossing =
    { rid : Rid
    , color : Color
    , origin : Origin
    , xFraction : Fraction
    , yFraction : Fraction
    }


type Orientation
    = Up
    | Down


type Origin
    = Left
    | Right
    | None


type alias Fraction =
    { numerator : Int
    , denominator : Int
    }
