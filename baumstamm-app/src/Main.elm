module Main exposing (..)

import Browser
import File exposing (File)
import File.Select as Select
import Html exposing (Html, button, div, text)
import Html.Events exposing (onClick)
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


type alias Model =
    { flags : Flags, file : String, tree : String }


init : Value -> ( Model, Cmd Msg )
init flags =
    ( Model
        (decodeFlags flags)
        ""
        "Empty"
    , Cmd.none
    )



-- UPDATE


type Msg
    = SendRpc Rpc.Outgoing
    | ReceiveRcp Rpc.Incoming
    | SelectFile
    | LoadFile File


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



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ div [] [ text model.tree ]
        , button [ onClick SelectFile ] [ text "Open file" ]
        , div []
            [ text
                (if model.flags.isTauri then
                    "Tauri!"

                 else
                    "Browser!"
                )
            ]
        ]
