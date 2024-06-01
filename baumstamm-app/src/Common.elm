module Common exposing (..)

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
    { persons : Value, relationships : Value, grid : Value }


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
