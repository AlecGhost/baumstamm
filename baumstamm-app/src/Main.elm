module Main exposing (..)

import Browser
import Common exposing (modal, palette, toast)
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
import Person
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
    , modal : Maybe Modal
    , toasts : List String
    , panzoom : PanZoom.Model Msg
    , infoTableKey : String
    , infoTableValue : String
    }


type Frame
    = TreeFrame
    | SettingsFrame


type Modal
    = EditModal


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


clearInfoTable : Model -> Model
clearInfoTable model =
    { model | infoTableKey = "", infoTableValue = "" }


type Msg
    = SendRpc Rpc.Outgoing
    | ReceiveRcp Rpc.Incoming
    | SelectFile
    | LoadFile File
    | ToggleSettings
    | ShowEdit
    | DismissEdit
    | UpdatePanZoom PanZoom.MouseEvent
    | New
    | InsertInfo Rpc.InsertInfoPayload
    | SelectPerson Pid
    | ShowToast String
    | DismissToast Int
    | UpdateInfoTableKey String
    | UpdateInfoTableValue String
    | ClearInfoTable
    | NoOp


init : Value -> ( Model, Cmd Msg )
init flags =
    ( { flags = decodeFlags flags
      , file = ""
      , treeData = Nothing
      , activePerson = Nothing
      , frame = TreeFrame
      , modal = Nothing
      , toasts = []
      , panzoom =
            PanZoom.init
                (PanZoom.defaultConfig UpdatePanZoom)
                { scale = 1, position = { x = 600, y = 600 } }
      , infoTableKey = ""
      , infoTableValue = ""
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

        ReceiveRcp (Rpc.InvalidProc procName) ->
            update
                (ShowToast <|
                    "Failed to decode RPC: The procedure '"
                        ++ procName
                        ++ "' is unknown.'"
                )
                model

        ReceiveRcp Rpc.NoProc ->
            update (ShowToast "Failed to decode RPC: No procedure name was specified.") model

        ReceiveRcp (Rpc.NoPayload procName) ->
            update
                (ShowToast <|
                    "Failed to decode RPC: No payload was specified for procedure '"
                        ++ procName
                        ++ "'."
                )
                model

        ReceiveRcp (Rpc.Error message) ->
            update (ShowToast message) model

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

        ShowEdit ->
            ( { model | modal = Just EditModal }, Cmd.none )

        DismissEdit ->
            ( { model | modal = Nothing } |> clearInfoTable, Cmd.none )

        UpdatePanZoom event ->
            ( { model | panzoom = PanZoom.update event model.panzoom }, Cmd.none )

        New ->
            update (SendRpc Rpc.New) model

        InsertInfo payload ->
            update (SendRpc <| Rpc.InsertInfo payload) (model |> clearInfoTable)

        SelectPerson pid ->
            ( { model | activePerson = Just pid }, Cmd.none )

        ShowToast message ->
            ( { model | toasts = message :: model.toasts }, Cmd.none )

        DismissToast index ->
            ( { model
                | toasts =
                    List.take index model.toasts
                        ++ List.drop (index + 1) model.toasts
              }
            , Cmd.none
            )

        UpdateInfoTableKey key ->
            ( { model | infoTableKey = key }, Cmd.none )

        UpdateInfoTableValue value ->
            ( { model | infoTableValue = value }, Cmd.none )

        ClearInfoTable ->
            ( model |> clearInfoTable, Cmd.none )

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
                , onEdit = model.activePerson |> Maybe.map (\_ -> ShowEdit)
                }
            , body model
            ]


body : Model -> Element Msg
body model =
    let
        viewModal =
            case ( model.modal, model.activePerson, model.treeData ) of
                ( Just EditModal, Just pid, Just treeData ) ->
                    [ modal <|
                        el [ centerX, centerY, width fill, height fill ] <|
                            Person.viewEdit
                                { pid = pid
                                , treeData = treeData
                                , onDismiss = DismissEdit
                                , infoTableInput =
                                    { onCancel = ClearInfoTable
                                    , onSave =
                                        InsertInfo
                                            { pid = pid
                                            , key = model.infoTableKey
                                            , value = model.infoTableValue
                                            }
                                    , onKeyUpdate = UpdateInfoTableKey
                                    , onValueUpdate = UpdateInfoTableValue
                                    , key = model.infoTableKey
                                    , value = model.infoTableValue
                                    }
                                }
                    ]

                _ ->
                    []

        viewToasts =
            if List.length model.toasts /= 0 then
                [ inFront <|
                    column
                        [ alignBottom, centerX ]
                        (model.toasts
                            |> List.indexedMap
                                (\index message ->
                                    el [ paddingXY 0 5 ] <|
                                        toast message (DismissToast index)
                                )
                        )
                ]

            else
                []
    in
    el
        ([ Region.mainContent
         , width fill
         , height fill
         , HA.style "overflow" "hidden" |> htmlAttribute
         ]
            |> List.append viewModal
            |> List.append viewToasts
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
