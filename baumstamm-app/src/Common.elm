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
            if percentileX == 1 then
                1000000

            else
                round (2 / ((1 / percentileX) - 1))

        portionY =
            if percentileY == 1 then
                1000000

            else
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


flip : (a -> b -> c) -> (b -> a -> c)
flip f =
    \b a -> f a b



{-
   Create an instance of `Color` from hsl values.

   See: https://www.baeldung.com/cs/convert-color-hsl-rgb
-}


hsl : Float -> Float -> Float -> Color
hsl h s l =
    let
        c =
            (1 - abs (2 * l - 1)) * s

        h1 =
            round (h / 60)

        x =
            c * toFloat (1 - abs (modBy 2 h1 - 1))

        m =
            l - c / 2

        ( r1, g1, b1 ) =
            if h1 <= 1 then
                ( c, x, 0 )

            else if h1 <= 2 then
                ( x, c, 0 )

            else if h1 <= 3 then
                ( 0, c, x )

            else if h1 <= 4 then
                ( 0, x, c )

            else if h1 <= 5 then
                ( x, 0, c )

            else if h1 <= 6 then
                ( c, 0, x )

            else
                ( 0, 0, 0 )

        ( r, g, b ) =
            ( r1 + m, g1 + m, b1 + m )
    in
    rgb r g b


toRgbString : Color -> String
toRgbString color =
    let
        { red, green, blue, alpha } =
            toRgb color
    in
    "rgba("
        ++ String.fromFloat (red * 100)
        ++ "%,"
        ++ String.fromFloat (green * 100)
        ++ "%,"
        ++ String.fromFloat (blue * 100)
        ++ "%,"
        ++ String.fromFloat (alpha * 100)
        ++ "%"
        ++ ")"
