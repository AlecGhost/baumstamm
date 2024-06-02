module Common exposing (..)

import Element exposing (..)
import Element.Background as Background
import Element.Border as Border


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


modal : Element msg -> Attribute msg
modal element =
    inFront <|
        margin 0.8
            0.8
            (el
                [ Background.color palette.fg
                , width fill
                , height fill
                , paddingXY 30 30
                , Border.rounded 15
                ]
                element
            )
