module Main exposing (..)

import Browser
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Font as Font
import Element.Input exposing (button)
import FeatherIcons exposing (withSize)
import File exposing (File)
import File.Select as Select
import Html exposing (Html)
import Json.Decode as Decode exposing (Value)
import Rpc
import Task



-- MAIN


main : Program Value Model Msg
main =
    Browser.element { init = init, subscriptions = subscriptions, update = update, view = view }


subscriptions : Model -> Sub Msg
subscriptions _ =
    Rpc.receive (Rpc.decodeIncoming >> ReceiveRcp)



-- MODEL


type alias Flags =
    { isTauri : Bool }


decodeFlags : Value -> Flags
decodeFlags value =
    let
        flagDecoder =
            Decode.map Flags
                (Decode.field "isTauri" Decode.bool)
    in
    Result.withDefault
        { isTauri = False }
        (Decode.decodeValue flagDecoder value)


type Frame
    = TreeFrame
    | SettingsFrame


type alias Model =
    { flags : Flags
    , file : String
    , tree : String
    , frame : Frame
    , modal : Maybe (Element Msg)
    }


init : Value -> ( Model, Cmd Msg )
init flags =
    ( Model
        (decodeFlags flags)
        ""
        "Empty"
        TreeFrame
        (Just <| text "Hello")
    , Cmd.none
    )



-- UPDATE


type Msg
    = SendRpc Rpc.Outgoing
    | ReceiveRcp Rpc.Incoming
    | SelectFile
    | LoadFile File
    | ToggleSettings
    | ShowModal (Element Msg)
    | HideModal


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SendRpc data ->
            ( model, Rpc.encodeOutgoing data |> Rpc.send )

        ReceiveRcp data ->
            ( { model | tree = Debug.toString data }, Cmd.none )

        SelectFile ->
            ( model, Select.file [ "application/json" ] LoadFile )

        LoadFile file ->
            ( model
            , Task.perform (Rpc.Load >> SendRpc) (File.toString file)
            )

        ToggleSettings ->
            ( { model
                | frame =
                    case model.frame of
                        SettingsFrame ->
                            TreeFrame

                        TreeFrame ->
                            SettingsFrame
              }
            , Cmd.none
            )

        ShowModal element ->
            ( { model | modal = Just element }, Cmd.none )

        HideModal ->
            ( { model | modal = Nothing }, Cmd.none )



-- VIEW


palette : { bg : Color, fg : Color, action : Color, marker : Color }
palette =
    { bg = rgb255 48 56 65
    , fg = rgb255 58 71 80
    , action = rgb255 0 173 181
    , marker = rgb255 238 238 238
    }


view : Model -> Html Msg
view model =
    Element.layoutWith
        { options =
            [ focusStyle
                { backgroundColor = Nothing
                , shadow = Nothing
                , borderColor = Just palette.marker
                }
            ]
        }
        [ Background.color palette.bg ]
    <|
        row [ height fill, width fill ]
            [ navBar
            , body model
            ]


navBar : Element Msg
navBar =
    column [ Background.color palette.fg, height fill, width (px 80) ]
        [ el [] <| text "Baumstamm"
        , el [ alignBottom, centerX, Element.paddingXY 0 5 ] <|
            button
                [ pointer
                , Font.color palette.action
                , mouseOver [ Font.color palette.marker ]
                ]
                { label = FeatherIcons.settings |> withSize 40 |> FeatherIcons.toHtml [] |> Element.html
                , onPress = Just ToggleSettings
                }
        ]


buttonStyles : List (Attribute msg)
buttonStyles =
    [ Border.rounded 15
    , Border.width 2
    , Border.color palette.action
    , paddingXY 2 3
    , pointer
    , mouseOver [ Border.color palette.marker ]
    ]


body : Model -> Element Msg
body model =
    el
        (List.append
            (case model.modal of
                Just element ->
                    [ modal <|
                        el [ centerX, centerY, width fill, height fill ] <|
                            element
                    ]

                Nothing ->
                    []
            )
            [ width fill
            , height fill
            ]
        )
    <|
        case model.frame of
            SettingsFrame ->
                el [ centerX, centerY ] <| text "Settings"

            TreeFrame ->
                el [ centerX, centerY ] <| text "Tree"


modal : Element Msg -> Attribute Msg
modal element =
    margin 0.8
        0.8
        (el
            [ Background.color palette.fg
            , width fill
            , height fill
            , paddingXY 30 30
            , Border.rounded 15
            , inFront <|
                button [ alignTop, alignRight, Font.color palette.action ]
                    { label = FeatherIcons.x |> withSize 40 |> FeatherIcons.toHtml [] |> Element.html
                    , onPress = Just HideModal
                    }
            ]
            element
        )


margin : Float -> Float -> Element msg -> Attribute msg
margin percentileX percentileY element =
    let
        portionX =
            round (2 / ((1 / percentileX) - 1))

        portionY =
            round (2 / ((1 / percentileY) - 1))
    in
    inFront
        (row
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
        )
