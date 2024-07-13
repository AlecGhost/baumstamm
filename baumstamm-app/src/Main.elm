module Main exposing (..)

import Browser
import Common exposing (Settings, modal, palette, toast)
import Connections exposing (view)
import Data exposing (GridItem(..), Pid, TreeData)
import Element exposing (..)
import Element.Background as Background
import Element.Font as Font
import Element.Region as Region
import FeatherIcons
import File exposing (File)
import File.Download
import File.Select
import Html exposing (Html)
import Html.Attributes as HA
import Json.Decode as Decode exposing (Value)
import Nav exposing (navIcon)
import PanZoom
import Person
import Rpc
import Settings
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
    { isTauri : Bool
    , treeData : Maybe TreeData
    , activePerson : Maybe Pid
    , frame : Frame
    , modal : Maybe Modal
    , toasts : List String
    , panzoom : PanZoom.Model Msg
    , infoTableKey : String
    , infoTableValue : String
    , settings : Settings
    }


type Frame
    = TreeFrame
    | SettingsFrame


type Modal
    = EditModal


type alias Flags =
    { isTauri : Bool
    , settings : Settings
    , treeData : Maybe TreeData
    }


defaultFlags : Flags
defaultFlags =
    { isTauri = False
    , settings = defaultSettings
    , treeData = Nothing
    }


decodeFlags : Value -> Flags
decodeFlags value =
    let
        decodeIsTauri =
            Decode.field "isTauri" Decode.bool

        decodeSettings =
            Decode.field "settings"
                (Decode.oneOf
                    [ Decode.map3 Settings
                        (Decode.field "showMiddleNames" Decode.bool)
                        (Decode.field "showProfilePictures" Decode.bool)
                        (Decode.field "showDates" Decode.bool)
                    , Decode.succeed defaultSettings
                    ]
                )

        decodeTreeData =
            Decode.field "treeData"
                (Decode.oneOf
                    [ Rpc.decodeTreeData
                        |> Decode.map Just
                    , Decode.succeed Nothing
                    ]
                )

        decode =
            Decode.map3 Flags
                decodeIsTauri
                decodeSettings
                decodeTreeData
    in
    Result.withDefault
        defaultFlags
        (Decode.decodeValue decode value)


clearInfoTable : Model -> Model
clearInfoTable model =
    { model | infoTableKey = "", infoTableValue = "" }


type Msg
    = SendRpc Rpc.Outgoing
    | ReceiveRcp Rpc.Incoming
    | SelectFile
    | LoadFile File
    | SaveFile
    | DownloadFile String
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
    | UpdateSettings Settings
    | ResetSettings
    | NoOp


defaultSettings : Settings
defaultSettings =
    { showProfilePictures = False
    , showMiddleNames = False
    , showDates = False
    }


init : Value -> ( Model, Cmd Msg )
init flags =
    let
        args =
            decodeFlags flags
    in
    ( { isTauri = args.isTauri
      , settings = args.settings
      , treeData = args.treeData
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

        ReceiveRcp (Rpc.Download content) ->
            update (DownloadFile content) model

        ReceiveRcp (Rpc.Error message) ->
            update (ShowToast message) model

        SelectFile ->
            ( model, File.Select.file [ "application/json" ] LoadFile )

        LoadFile file ->
            ( model
            , Task.perform (Rpc.Load >> SendRpc) (File.toString file)
            )

        SaveFile ->
            update (SendRpc Rpc.Save) model

        DownloadFile content ->
            ( model, content |> File.Download.string "tree.json" "application/json" )

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

        UpdateSettings settings ->
            ( { model | settings = settings }, Cmd.none )

        ResetSettings ->
            ( { model | settings = defaultSettings }, Cmd.none )

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
                , onDownload = Just SaveFile
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
                Settings.view
                    { settings = model.settings
                    , onUpdate = UpdateSettings
                    , onReset = ResetSettings
                    , onDismiss = ToggleSettings
                    }

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
                                , settings = model.settings
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
