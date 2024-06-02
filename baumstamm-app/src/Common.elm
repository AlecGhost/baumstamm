module Common exposing (..)

import Dict exposing (Dict)
import Element exposing (..)
import Element.Border as Border
import Element.Font as Font
import Element.Input exposing (button)
import FeatherIcons exposing (withSize)
import Json.Decode exposing (Value)


palette : { bg : Color, fg : Color, action : Color, marker : Color }
palette =
    { bg = rgb255 48 56 65
    , fg = rgb255 58 71 80
    , action = rgb255 0 173 181
    , marker = rgb255 238 238 238
    }


buttonStyles : List (Attribute msg)
buttonStyles =
    [ Border.rounded 15
    , Border.width 2
    , Border.color palette.action
    , paddingXY 2 3
    , pointer
    , mouseOver [ Border.color palette.marker ]
    ]


type alias TreeData =
    { persons : List Person, relationships : List Relationship, grid : Grid }


type alias Person =
    { id : Pid
    , info : Dict String String
    }


getFirstName : Person -> Maybe String
getFirstName person =
    person.info
        |> Dict.get "@firstName"


getMiddleNames : Person -> Maybe String
getMiddleNames person =
    person.info
        |> Dict.get "@middleNames"


getLastName : Person -> Maybe String
getLastName person =
    person.info
        |> Dict.get "@lastName"


getPerson : Pid -> TreeData -> Maybe Person
getPerson pid treeData =
    treeData.persons
        |> List.filter (\person -> person.id == pid)
        |> List.head


type alias Relationship =
    { id : Rid
    , parents : ( Maybe Pid, Maybe Pid )
    , children : List Pid
    }


type alias Grid =
    List (List GridItem)


type GridItem
    = PersonItem Pid
    | ConnectionsItem Connections


type alias Connections =
    { orientation : Orientation
    , totalX : Int
    , totalY : Int
    , passing : List Passing
    , ending : List Ending
    , crossing : List Crossing
    }


type alias Passing =
    { connection : Cid
    , color : Color
    , yIndex : Int
    }


type alias Ending =
    { connection : Cid
    , color : Color
    , origin : Origin
    , xIndex : Int
    , yIndex : Int
    }


type alias Crossing =
    { connection : Cid
    , color : Color
    , origin : Origin
    , xIndex : Int
    , yIndex : Int
    }


type Orientation
    = Up
    | Down


type Origin
    = Left
    | Right
    | None


type alias Pid =
    String


type alias Rid =
    String


type alias Cid =
    Int


margin : Float -> Float -> Element msg -> Element msg
margin percentileX percentileY element =
    let
        portionX =
            round (2 / ((1 / percentileX) - 1))

        portionY =
            round (2 / ((1 / percentileY) - 1))
    in
    row
        [ width fill
        , height fill
        ]
        [ el [ width (fillPortion 1) ] none
        , column [ width (fillPortion portionX), height fill ]
            [ el [ height (fillPortion 1) ] none
            , el
                [ width fill
                , height (fillPortion portionY)
                ]
                element
            , el [ height (fillPortion 1) ] none
            ]
        , el [ width (fillPortion 1) ] none
        ]


navIcon : List (Attribute msg) -> { icon : FeatherIcons.Icon, onPress : Maybe msg } -> Element msg
navIcon attributes { icon, onPress } =
    el ([ centerX, Element.paddingXY 0 5 ] |> List.append attributes) <|
        button
            [ pointer
            , Font.color palette.action
            , mouseOver [ Font.color palette.marker ]
            ]
            { label =
                icon
                    |> withSize 40
                    |> FeatherIcons.toHtml []
                    |> Element.html
            , onPress = onPress
            }


select : (a -> Bool) -> (a -> b) -> (a -> b) -> a -> b
select condition f g x =
    if condition x then
        f x

    else
        g x
