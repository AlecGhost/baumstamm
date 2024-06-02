module Main exposing (..)

import Browser
import Common exposing (modal, palette)
import Connections exposing (view)
import Data exposing (GridItem(..), Pid, TreeData)
import Element exposing (..)
import Element.Background as Background
import Element.Font as Font
import Element.Region as Region
import FeatherIcons
import File exposing (File)
import File.Select as Select
import Html exposing (Html)
import Html.Attributes as HA
import Json.Decode as Decode exposing (Value)
import Nav exposing (navIcon)
import PanZoom
import Rpc
import Task
import Tree
import Utils exposing (..)



-- MAIN


main : Program Value Model Msg
main =
    Browser.element { init = init, subscriptions = subscriptions, update = update, view = view }


subscriptions : Model -> Sub Msg
subscriptions _ =
    Rpc.receive (Rpc.decodeIncoming >> ReceiveRcp)



-- MODEL


type alias Model =
    { flags : Flags
    , file : String
    , treeData : Maybe TreeData
    , activePerson : Maybe Pid
    , frame : Frame
    , modal : Maybe (Element Msg)
    , panzoom : PanZoom.Model Msg
    }


type Frame
    = TreeFrame
    | SettingsFrame


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


type Msg
    = SendRpc Rpc.Outgoing
    | ReceiveRcp Rpc.Incoming
    | SelectFile
    | LoadFile File
    | ToggleSettings
    | ShowModal (Element Msg)
    | HideModal
    | UpdatePanZoom PanZoom.MouseEvent
    | New
    | SelectPerson Pid
    | NoOp


init : Value -> ( Model, Cmd Msg )
init flags =
    ( { flags = decodeFlags flags
      , file = ""
      , treeData = Nothing
      , activePerson = Nothing
      , frame = TreeFrame
      , modal = Nothing
      , panzoom =
            PanZoom.init
                (PanZoom.defaultConfig UpdatePanZoom)
                { scale = 1, position = { x = 600, y = 600 } }
      }
    , Cmd.none
    )



-- UPDATE


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SendRpc data ->
            ( model, Rpc.encodeOutgoing data |> Rpc.send )

        ReceiveRcp (Rpc.TreeData data) ->
            ( { model | frame = TreeFrame, treeData = Just data }, Cmd.none )

        ReceiveRcp _ ->
            ( model, Cmd.none )

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
                        TreeFrame ->
                            SettingsFrame

                        SettingsFrame ->
                            TreeFrame
              }
            , Cmd.none
            )

        ShowModal element ->
            ( { model | modal = Just element }, Cmd.none )

        HideModal ->
            ( { model | modal = Nothing }, Cmd.none )

        UpdatePanZoom event ->
            ( { model | panzoom = PanZoom.update event model.panzoom }, Cmd.none )

        New ->
            update (SendRpc Rpc.New) model

        SelectPerson pid ->
            ( { model | activePerson = Just pid }, Cmd.none )

        NoOp ->
            ( model, Cmd.none )



-- VIEW


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
        [ Background.color palette.bg, width fill, height fill, Font.color (rgb 1 1 1) ]
    <|
        row
            [ height fill
            , width fill
            ]
            [ Nav.navBar
                { onNew = Just New
                , onSettings = Just ToggleSettings
                , onUpload = Just SelectFile
                , onEdit = Just (ShowModal Element.none)
                }
            , body model
            ]


body : Model -> Element Msg
body model =
    el
        ([ Region.mainContent
         , width fill
         , height fill
         , HA.style "overflow" "hidden" |> htmlAttribute
         ]
            |> List.append
                (case model.modal of
                    Just element ->
                        [ modal HideModal <|
                            el [ centerX, centerY, width fill, height fill ] <|
                                element
                        ]

                    Nothing ->
                        []
                )
        )
    <|
        case model.frame of
            SettingsFrame ->
                el [ centerX, centerY ] <| text "Settings"

            TreeFrame ->
                case model.treeData of
                    -- draw tree
                    Just treeData ->
                        PanZoom.view model.panzoom
                            { viewportAttributes = [ width fill, height fill ], contentAttributes = [] }
                        <|
                            Tree.view
                                { treeData = treeData
                                , activePerson = model.activePerson
                                , onSelect = SelectPerson
                                }

                    Nothing ->
                        -- draw start page
                        column [ centerX, centerY, spacing 10 ]
                            [ text "Start a new tree or upload an existing file."
                            , row [ spacing 20, width fill ]
                                [ navIcon [] { icon = FeatherIcons.filePlus, onPress = Just New }
                                , navIcon [] { icon = FeatherIcons.upload, onPress = Just SelectFile }
                                ]
                            ]
