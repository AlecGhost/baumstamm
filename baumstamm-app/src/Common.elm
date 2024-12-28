module Common exposing (..)

import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Font as Font
import Element.Input exposing (button)
import FeatherIcons
import Html.Events
import Json.Decode as Decode


type alias Settings =
    { showProfilePictures : Bool
    , showMiddleNames : Bool
    , showDates : Bool
    , palette : Palette
    }


type alias Palette =
    { bg : Color
    , fg : Color
    , action : Color
    , marker : Color
    , font : Color
    }


defaultPalette : Palette
defaultPalette =
    { bg = rgb255 48 56 65
    , fg = rgb255 58 71 80
    , action = rgb255 0 173 181
    , marker = rgb255 238 238 238
    , font = rgb255 255 255 255
    }


printPalette : Palette
printPalette =
    { bg = rgb 1 1 1
    , fg = rgb 1 1 1
    , action = rgb 0 0 0
    , marker = rgb 0.5 0.5 0.5
    , font = rgb 0 0 0
    }


type alias ButtonStyles msg =
    { primary : List (Attribute msg)
    , cancel : List (Attribute msg)
    }


buttonStyles : Settings -> ButtonStyles msg
buttonStyles settings =
    { primary =
        [ centerX
        , width (px 100)
        , Border.rounded 15
        , paddingXY 2 3
        , Border.width 2
        , Border.color settings.palette.action
        , pointer
        , mouseOver
            [ Border.color settings.palette.marker ]
        ]
    , cancel =
        [ centerX
        , width (px 100)
        , Border.rounded 15
        , paddingXY 2 3
        , Border.width 2
        , Border.color settings.palette.fg
        , pointer
        , mouseOver
            [ Border.color settings.palette.marker ]
        ]
    }


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


modal : Settings -> Element msg -> Attribute msg
modal settings element =
    inFront <|
        margin 0.8
            0.8
            (el
                [ Background.color settings.palette.fg
                , width fill
                , height fill
                , paddingXY 30 30
                , Border.rounded 15
                ]
                element
            )


toast : Settings -> String -> msg -> Element msg
toast settings message onDismiss =
    el
        [ Background.color settings.palette.fg
        , paddingXY 10 10
        , Border.width 2
        , Border.color settings.palette.action
        , Border.rounded 15
        ]
    <|
        row []
            [ el [ centerX, centerY ] <|
                text message
            , button
                [ pointer
                , Font.color settings.palette.action
                , mouseOver [ Font.color settings.palette.marker ]
                ]
                { onPress = Just onDismiss
                , label =
                    FeatherIcons.x
                        |> FeatherIcons.toHtml []
                        |> Element.html
                }
            ]


type alias KeyboardEvent =
    { key : String
    , ctrl : Bool
    , shift : Bool
    , meta : Bool
    }


onKeyboardEvent : (KeyboardEvent -> Maybe msg) -> Attribute msg
onKeyboardEvent eventHandler =
    let
        decoder =
            Decode.map4 KeyboardEvent
                (Decode.field "key" Decode.string)
                (Decode.field "ctrlKey" Decode.bool)
                (Decode.field "shiftKey" Decode.bool)
                (Decode.field "metaKey" Decode.bool)
                |> Decode.andThen
                    (\event ->
                        case eventHandler (Debug.log "event" event) of
                            Just msg ->
                                Decode.succeed ( msg, True )

                            Nothing ->
                                Decode.fail "No event triggered"
                    )
    in
    Html.Events.stopPropagationOn "keyup"
        decoder
        |> htmlAttribute
