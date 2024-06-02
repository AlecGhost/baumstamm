module Main exposing (..)

import Browser
import Common exposing (..)
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input exposing (button)
import Element.Region as Region
import FeatherIcons exposing (withSize)
import File exposing (File)
import File.Select as Select
import Html exposing (Html)
import Html.Attributes as HA
import Json.Decode as Decode exposing (Value)
import PanZoom
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
            [ navBar
            , body model
            ]


navBar : Element Msg
navBar =
    column
        [ Region.navigation
        , spacing 7
        , Background.color palette.fg
        , height fill
        , width (px 80)
        ]
        [ navIcon [] { icon = FeatherIcons.filePlus, onPress = Just New }
        , navIcon [] { icon = FeatherIcons.upload, onPress = Just SelectFile }
        , navIcon []
            { icon = FeatherIcons.edit
            , onPress = Just (ShowModal <| text "Modal")
            }
        , navIcon [ alignBottom ]
            { icon = FeatherIcons.settings
            , onPress = Just ToggleSettings
            }
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
                        [ modal <|
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
                            treeFrame treeData model.activePerson

                    Nothing ->
                        -- draw start page
                        column [ centerX, centerY, spacing 10 ]
                            [ text "Start a new tree or upload an existing file."
                            , row [ spacing 20, width fill ]
                                [ navIcon [] { icon = FeatherIcons.filePlus, onPress = Just New }
                                , navIcon [] { icon = FeatherIcons.upload, onPress = Just SelectFile }
                                ]
                            ]


modal : Element Msg -> Attribute Msg
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
                , inFront <|
                    button
                        [ alignTop
                        , alignRight
                        , Font.color palette.action
                        , mouseOver [ Font.color palette.marker ]
                        ]
                        { label =
                            FeatherIcons.x
                                |> withSize 40
                                |> FeatherIcons.toHtml []
                                |> Element.html
                        , onPress = Just HideModal
                        }
                ]
                element
            )


treeFrame : TreeData -> Maybe Pid -> Element Msg
treeFrame treeData activePerson =
    column []
        (treeData.grid
            |> List.map
                (\c ->
                    row
                        [ spacing 10
                        ]
                        (c
                            |> List.map
                                (\item ->
                                    el
                                        [ width (px 200)
                                        , height (px 200)
                                        ]
                                        (case item of
                                            PersonItem pid ->
                                                let
                                                    isActive =
                                                        activePerson == Just pid
                                                in
                                                personCard pid isActive treeData

                                            ConnectionsItem _ ->
                                                Element.none
                                        )
                                )
                        )
                )
        )


personCard : Pid -> Bool -> TreeData -> Element Msg
personCard pid isActive treeData =
    case getPerson pid treeData of
        Just person ->
            column
                [ width fill
                , height fill
                , Background.color palette.fg
                , Border.width 2
                , Border.rounded 15
                , Border.color
                    (if isActive then
                        palette.marker

                     else
                        palette.action
                    )
                , mouseOver [ Border.color palette.marker ]
                , onClick (SelectPerson pid)
                ]
                (let
                    firstName =
                        getFirstName person

                    middleNames =
                        getMiddleNames person

                    lastName =
                        getLastName person

                    names =
                        [ firstName, middleNames, lastName ]
                            |> List.filterMap identity
                            |> select List.isEmpty ((::) "?") identity
                 in
                 names
                    |> List.map text
                    |> List.map
                        (el
                            [ centerX, centerY ]
                        )
                )

        Nothing ->
            el [ Background.color (rgb 1 0 0) ] <| text "Inconsistent data!"
