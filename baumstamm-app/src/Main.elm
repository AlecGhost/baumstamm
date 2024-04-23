module Main exposing (..)

import Browser
import Element
    exposing
        ( Color
        , Element
        , column
        , el
        , row
        , text
        )
import Element.Background as Background
import Element.Font as Font
import Element.Input exposing (button)
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
    { flags : Flags, file : String, tree : String, frame : Frame }


init : Value -> ( Model, Cmd Msg )
init flags =
    ( Model
        (decodeFlags flags)
        ""
        "Empty"
        TreeFrame
    , Cmd.none
    )



-- UPDATE


type Msg
    = SendRpc Rpc.Outgoing
    | ReceiveRcp Rpc.Incoming
    | SelectFile
    | LoadFile File
    | ToggleSettings


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



-- VIEW


view : Model -> Html Msg
view model =
    Element.layout [ Background.color palette.bg ] <|
        row [ Element.height Element.fill, Element.width Element.fill ]
            [ navBar
            , body model.frame
            ]


palette : { bg : Color, fg : Color, action : Color, marker : Color }
palette =
    { bg = Element.rgb255 48 56 65
    , fg = Element.rgb255 58 71 80
    , action = Element.rgb255 0 173 181
    , marker = Element.rgb255 238 238 238
    }


navBar : Element Msg
navBar =
    column [ Background.color palette.fg, Element.height Element.fill, Element.alignLeft ]
        [ el [ Element.alignTop ] <| text "Baumstamm"
        , el [ Element.alignBottom, Element.centerX ] <|
            button []
                { label = icon "⚙"
                , onPress = Just ToggleSettings
                }
        ]


body : Frame -> Element msg
body frame =
    el [ Element.centerX, Element.centerY ] <|
        case frame of
            SettingsFrame ->
                text "Settings"

            TreeFrame ->
                text "Tree"


icon : String -> Element msg
icon txt =
    el [ Font.size 50, Element.paddingXY 5 5 ] (text txt)
